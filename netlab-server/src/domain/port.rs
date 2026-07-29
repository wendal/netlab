//! Domain types describing port numbers and ranges.
//!
//! These types are pure data and contain no IO. Actual allocation,
//! binding, and tracking live in the application layer (`port_pool`).
//! The boundary is enforced by keeping the domain types `Send + Sync`
//! with no `tokio` / `std::net` dependencies.

use crate::domain::errors::AppError;

/// A TCP/UDP port number.
pub type PortNumber = u16;

/// Inclusive `[start, end)` port range used to configure the pool.
///
/// `start` must be at least `1024` (we never hand out privileged ports) and
/// `end` must be strictly greater than `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    pub start: PortNumber,
    pub end: PortNumber,
}

impl PortRange {
    /// Construct a range, validating that:
    /// * `start >= 1024` (no privileged ports)
    /// * `end > start` (non-empty range; upper bound is exclusive)
    pub fn new(start: PortNumber, end: PortNumber) -> Result<Self, AppError> {
        if start < 1024 {
            return Err(AppError::BadType(format!(
                "port range start {start} must be >= 1024"
            )));
        }
        if end <= start {
            return Err(AppError::BadType(format!(
                "port range end {end} must be > start {start}"
            )));
        }
        Ok(Self { start, end })
    }

    /// Number of ports in the range.
    pub fn len(&self) -> usize {
        (self.end - self.start) as usize
    }

    /// True iff the range contains no ports.
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// Iterator over every port in the range.
    pub fn iter(&self) -> impl Iterator<Item = PortNumber> + '_ {
        self.start..self.end
    }
}

/// The lifecycle state of a single port slot in the pool.
///
/// `Bound` carries the `entity id` (an opaque handle into the
/// `application::port_pool` registry) so that we can map back to the
/// concrete `PortEntity` once a slot is allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortState {
    /// The slot is free and may be allocated.
    Free,
    /// The slot is bound to the given entity.
    Bound(PortNumber),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_valid_range() {
        let r = PortRange::new(2000, 2010).expect("valid range");
        assert_eq!(r.start, 2000);
        assert_eq!(r.end, 2010);
        assert_eq!(r.len(), 10);
        assert!(!r.is_empty());
    }

    #[test]
    fn new_rejects_start_below_1024() {
        let err = PortRange::new(1023, 2000).expect_err("must reject < 1024");
        assert!(matches!(err, AppError::BadType(_)));
    }

    #[test]
    fn new_rejects_start_equal_to_end() {
        let err = PortRange::new(2000, 2000).expect_err("end must be > start");
        assert!(matches!(err, AppError::BadType(_)));
    }

    #[test]
    fn new_rejects_end_below_start() {
        let err = PortRange::new(5000, 4000).expect_err("end < start must fail");
        assert!(matches!(err, AppError::BadType(_)));
    }

    #[test]
    fn len_counts_exclusive_upper_bound() {
        let r = PortRange::new(1024, 1025).expect("two ports");
        assert_eq!(r.len(), 1);
        let r = PortRange::new(1024, 1124).expect("100 ports");
        assert_eq!(r.len(), 100);
    }

    #[test]
    fn iter_yields_all_ports_in_order() {
        let r = PortRange::new(2000, 2003).expect("valid");
        let collected: Vec<u16> = r.iter().collect();
        assert_eq!(collected, vec![2000, 2001, 2002]);
    }

    #[test]
    fn port_state_free_and_bound_compare() {
        assert_eq!(PortState::Free, PortState::Free);
        assert_ne!(PortState::Free, PortState::Bound(1));
        assert_eq!(PortState::Bound(42), PortState::Bound(42));
        assert_ne!(PortState::Bound(42), PortState::Bound(43));
    }
}
