//! `application::port_pool` — concurrent pool of free TCP/UDP ports.
//!
//! This is a pure concurrent data structure. It performs no IO, no async,
//! and holds no sockets: it merely tracks which port numbers in a
//! configured [`PortRange`] are currently free versus in use. The actual
//! bind / listen logic lives in the transport implementations.
//!
//! Sharing model: every implementation is `Send + Sync` and is intended
//! to be shared via the [`SharedPortPool`] alias (an `Arc<dyn PortPool>`).

use crate::domain::errors::AppError;
use crate::domain::port::{PortNumber, PortRange};
use std::sync::Arc;

use dashmap::DashSet;
use parking_lot::Mutex;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Thread-safe pool of port numbers.
pub trait PortPool: Send + Sync {
    /// Take a uniformly random free port from the pool.
    ///
    /// Returns [`AppError::NoPortAvailable`] when the pool is exhausted.
    fn take_random(&self) -> Result<PortNumber, AppError>;

    /// Take a specific `port` from the pool.
    ///
    /// Returns [`AppError::PortInUse`] when the port is outside the
    /// configured range or is already held by the pool.
    fn take(&self, port: PortNumber) -> Result<PortNumber, AppError>;

    /// Release a previously taken port back to the pool.
    ///
    /// Returns `true` if a port was actually released, `false` if the
    /// port was not currently held (including ports outside the range).
    fn recycle(&self, port: PortNumber) -> bool;

    /// Number of ports currently held by the pool.
    fn used(&self) -> usize;

    /// Number of ports currently free.
    fn available(&self) -> usize;

    /// Total configured size of the pool (used + available).
    fn capacity(&self) -> usize;
}

/// Convenience alias for sharing a `PortPool` across threads.
pub type SharedPortPool = Arc<dyn PortPool>;

/// Default `PortPool` implementation.
///
/// Stores the free ports in a `parking_lot::Mutex<Vec<u16>>` (so we can
/// do O(1) `swap_remove` for arbitrary random indices) and the in-use
/// ports in a `dashmap::DashSet<u16>` (so `contains` / `remove` are
/// fast and lock-free per shard).
pub struct RandomPortPool {
    range: PortRange,
    free: Mutex<Vec<u16>>,
    in_use: DashSet<u16>,
}

impl RandomPortPool {
    /// Build a pool seeded with every port in `range` (all free).
    pub fn new(range: PortRange) -> Self {
        let free: Vec<u16> = range.iter().collect();
        Self {
            range,
            free: Mutex::new(free),
            in_use: DashSet::new(),
        }
    }
}

impl PortPool for RandomPortPool {
    fn take_random(&self) -> Result<PortNumber, AppError> {
        let mut free = self.free.lock();
        let Some(&port) = free.choose(&mut thread_rng()) else {
            return Err(AppError::NoPortAvailable);
        };
        // The chosen value is guaranteed to be in the vec, so the
        // position lookup cannot fail.
        let pos = free
            .iter()
            .position(|&x| x == port)
            .expect("chosen port must exist in free vec");
        free.swap_remove(pos);
        self.in_use.insert(port);
        Ok(port)
    }

    fn take(&self, port: PortNumber) -> Result<PortNumber, AppError> {
        // Reject anything outside the configured range.
        if !self.range.iter().any(|p| p == port) {
            return Err(AppError::PortInUse(port));
        }
        // Fast path: if it's already in use, fail without touching the free vec.
        if self.in_use.contains(&port) {
            return Err(AppError::PortInUse(port));
        }
        let mut free = self.free.lock();
        // Re-check under the lock: another thread may have taken it
        // between our `contains` and acquiring the free lock.
        if self.in_use.contains(&port) {
            return Err(AppError::PortInUse(port));
        }
        let Some(pos) = free.iter().position(|&x| x == port) else {
            // Defensive: the port is in range, not in use, but is missing
            // from the free vec. Treat as in-use to preserve the invariant
            // that used + available == capacity.
            return Err(AppError::PortInUse(port));
        };
        free.swap_remove(pos);
        self.in_use.insert(port);
        Ok(port)
    }

    fn recycle(&self, port: PortNumber) -> bool {
        if self.in_use.remove(&port).is_some() {
            self.free.lock().push(port);
            true
        } else {
            false
        }
    }

    fn used(&self) -> usize {
        self.in_use.len()
    }

    fn available(&self) -> usize {
        self.free.lock().len()
    }

    fn capacity(&self) -> usize {
        self.range.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_range() -> PortRange {
        PortRange::new(2000, 2010).expect("valid range")
    }

    #[test]
    fn new_pool_starts_empty_and_full() {
        let pool = RandomPortPool::new(small_range());
        assert_eq!(pool.used(), 0);
        assert_eq!(pool.available(), 10);
        assert_eq!(pool.capacity(), 10);
        assert_eq!(pool.used() + pool.available(), pool.capacity());
    }

    #[test]
    fn take_random_drains_pool_then_errors() {
        let pool = RandomPortPool::new(small_range());
        let mut taken = std::collections::HashSet::new();
        for _ in 0..10 {
            let p = pool.take_random().expect("still has free");
            assert!(
                (2000..2010).contains(&p),
                "port {p} out of configured range"
            );
            assert!(taken.insert(p), "duplicate take {p}");
        }
        let err = pool.take_random().expect_err("should be empty");
        assert!(matches!(err, AppError::NoPortAvailable));
        assert_eq!(pool.used(), 10);
        assert_eq!(pool.available(), 0);
        assert_eq!(pool.used() + pool.available(), pool.capacity());
    }

    #[test]
    fn recycled_port_can_be_taken_again() {
        let pool = RandomPortPool::new(small_range());
        let p = pool.take_random().expect("ok");
        assert_eq!(pool.used(), 1);
        assert!(pool.recycle(p));
        assert_eq!(pool.used(), 0);
        assert_eq!(pool.available(), 10);
        // The recycled port must be one of the ports take_random can
        // hand back. Drain the pool and confirm `p` shows up exactly once.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10 {
            let q = pool.take_random().expect("ok");
            assert!(seen.insert(q), "duplicate {q}");
        }
        assert!(seen.contains(&p), "recycled port {p} missing from drain");
    }

    #[test]
    fn take_specific_then_recycle_then_take_same() {
        let pool = RandomPortPool::new(small_range());
        let p = pool.take(2005).expect("ok");
        assert_eq!(p, 2005);
        assert_eq!(pool.used(), 1);
        // A second take of the same port must fail.
        let err = pool.take(2005).expect_err("in use");
        assert!(matches!(err, AppError::PortInUse(2005)));
        // Recycle and re-take.
        assert!(pool.recycle(2005));
        let p2 = pool.take(2005).expect("ok again");
        assert_eq!(p2, 2005);
        assert_eq!(pool.used(), 1);
    }

    #[test]
    fn take_outside_range_returns_port_in_use() {
        let pool = RandomPortPool::new(small_range());
        // Below the range.
        let err = pool.take(1000).expect_err("below range");
        assert!(matches!(err, AppError::PortInUse(1000)));
        // Above the range.
        let err = pool.take(9999).expect_err("above range");
        assert!(matches!(err, AppError::PortInUse(9999)));
        // Counts must be unchanged.
        assert_eq!(pool.used(), 0);
        assert_eq!(pool.available(), 10);
    }

    #[test]
    fn take_already_used_returns_port_in_use() {
        let pool = RandomPortPool::new(small_range());
        let _ = pool.take(2003).expect("ok");
        let err = pool.take(2003).expect_err("in use");
        assert!(matches!(err, AppError::PortInUse(2003)));
    }

    #[test]
    fn recycle_unknown_port_is_noop() {
        let pool = RandomPortPool::new(small_range());
        let _ = pool.take(2001).expect("ok");
        // Recycle a port that is in range but was never taken.
        assert!(!pool.recycle(2005));
        // Recycle a port outside the range — also no-op.
        assert!(!pool.recycle(9999));
        // The used/available counts must be unchanged.
        assert_eq!(pool.used(), 1);
        assert_eq!(pool.available(), 9);
        assert_eq!(pool.used() + pool.available(), pool.capacity());
    }

    #[test]
    fn stress_take_recycle_keeps_invariants() {
        // 100 ports, 1000 take/recycle cycles. Every take in a cycle
        // returns a fresh, never-before-seen port in the cycle; after
        // each cycle we recycle and the pool is empty again.
        let range = PortRange::new(2000, 2100).expect("ok");
        let pool = RandomPortPool::new(range);
        let cap = pool.capacity();
        assert_eq!(cap, 100);

        for _ in 0..1000 {
            let p = pool.take_random().expect("ok");
            assert!(pool.recycle(p));
        }
        assert_eq!(pool.used(), 0);
        assert_eq!(pool.available(), cap);
        assert_eq!(pool.used() + pool.available(), pool.capacity());
    }

    #[test]
    fn stress_take_random_no_duplicates_until_drained() {
        // Drain the pool, recording every port we see; we must see each
        // port exactly once before NoPortAvailable fires.
        let range = PortRange::new(3000, 3050).expect("ok");
        let pool = RandomPortPool::new(range);
        let cap = pool.capacity();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..cap {
            let p = pool.take_random().expect("ok");
            assert!(seen.insert(p), "duplicate port {p} during drain");
        }
        assert_eq!(seen.len(), cap);
        let err = pool.take_random().expect_err("empty");
        assert!(matches!(err, AppError::NoPortAvailable));
    }

    #[test]
    fn shared_port_pool_arc_shares_state() {
        let shared: SharedPortPool = Arc::new(RandomPortPool::new(small_range()));
        let a = shared.clone();
        let b = shared.clone();

        let p1 = a.take_random().expect("ok");
        let p2 = b.take_random().expect("ok");
        // Both clones observe the same underlying state.
        assert_eq!(shared.used(), 2);
        assert_eq!(a.used(), 2);
        assert_eq!(b.used(), 2);
        // p1 and p2 are different ports in the same pool.
        assert_ne!(p1, p2);
        // Recycle via one clone is visible to all others.
        assert!(a.recycle(p1));
        assert_eq!(shared.used(), 1);
        assert_eq!(a.used(), 1);
        assert_eq!(b.used(), 1);
        // A second recycle of the same port is a no-op.
        assert!(!b.recycle(p1));
        assert_eq!(shared.used(), 1);
    }
}
