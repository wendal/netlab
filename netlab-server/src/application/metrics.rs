//! Prometheus metrics constants and emission helpers.
//!
//! The actual recorder is installed at bootstrap time by
//! `infrastructure::metrics::exporter::install`. This module only
//! centralises the metric names and the small helpers used from the
//! application code so that label keys and counter names stay
//! consistent across the port service, the TCP/UDP/SSL entities, and
//! the WebSocket dispatch path.

/// Names of the Prometheus metrics emitted by the application.
pub struct MetricsSpec;

impl MetricsSpec {
    /// Number of port-allocation requests received, partitioned by wire type.
    pub const PORT_REQ_TOTAL: &'static str = "port_req_total";
    /// Number of ports currently held (gauge, partitioned by wire type).
    pub const PORT_USED: &'static str = "port_used";
    /// Total number of bytes that flowed through a port (counter, by wire type).
    pub const DATA_TOTAL: &'static str = "data_total";
    /// Total number of client connections ever opened (cumulative, by wire type).
    pub const CLIENT: &'static str = "client";
    /// Number of currently connected clients (gauge, by wire type).
    pub const CONNECTED_CLIENT: &'static str = "connected_client";
}

/// Bump the new-port metrics. Called once per successful `new_port`.
pub fn on_new_port(type_label: &'static str) {
    // `Counter::increment` takes `u64`; `Gauge::increment` takes `f64`.
    // Cast at the call site so the two call shapes stay explicit.
    metrics::counter!(MetricsSpec::PORT_REQ_TOTAL, "type" => type_label).increment(1);
    metrics::gauge!(MetricsSpec::PORT_USED, "type" => type_label).increment(1.0);
}

/// Decrement the port-used gauge. Called when a port is shut down and
/// returned to the pool.
pub fn on_close_port(type_label: &'static str) {
    metrics::gauge!(MetricsSpec::PORT_USED, "type" => type_label).decrement(1.0);
}

/// Add `bytes` to the data-flow counter. Called from the entity layer
/// whenever a payload is sent or received.
pub fn on_data(type_label: &'static str, bytes: u64) {
    metrics::counter!(MetricsSpec::DATA_TOTAL, "type" => type_label).increment(bytes);
}

/// Bump client counters when a new client connects on a port.
pub fn on_client_open(type_label: &'static str) {
    metrics::gauge!(MetricsSpec::CONNECTED_CLIENT, "type" => type_label).increment(1.0);
    metrics::gauge!(MetricsSpec::CLIENT, "type" => type_label).increment(1.0);
}

/// Decrement client counters when a client disconnects.
pub fn on_client_close(type_label: &'static str) {
    metrics::gauge!(MetricsSpec::CONNECTED_CLIENT, "type" => type_label).decrement(1.0);
    metrics::gauge!(MetricsSpec::CLIENT, "type" => type_label).decrement(1.0);
}

#[cfg(test)]
pub mod test_util {
    //! Minimal in-process metrics recorder used by the test suite to
    //! assert that the right `counter!` / `gauge!` calls happen.
    //!
    //! Usage:
    //! ```ignore
    //! let r = TestRecorder::default();
    //! metrics::with_local_recorder(&r, || {
    //!     metrics::counter!("foo", "type" => "tcp").increment(1.0);
    //! });
    //! let calls = r.calls.lock().unwrap();
    //! assert!(calls.iter().any(|c| c.name == "foo"));
    //! ```

    use std::sync::{Arc, Mutex};

    use metrics::{
        Counter, CounterFn, Gauge, GaugeFn, Histogram, Key, KeyName, Metadata, Recorder,
        SharedString, Unit,
    };

    /// What kind of operation a recorded call represents.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Op {
        Increment,
        Decrement,
        Absolute,
    }

    /// A single recorded metric call.
    #[derive(Debug, Clone)]
    pub struct Call {
        pub name: String,
        pub labels: Vec<(String, String)>,
        pub value: f64,
        pub op: Op,
    }

    /// A [`metrics::Recorder`] that appends every call into a shared
    /// vector. Cheap, no allocations per call, and inspectable from any
    /// test.
    #[derive(Default, Clone)]
    pub struct TestRecorder {
        pub calls: Arc<Mutex<Vec<Call>>>,
    }

    impl Recorder for TestRecorder {
        fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_gauge(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}

        fn register_counter(&self, key: &Key, _: &Metadata<'_>) -> Counter {
            Counter::from_arc(Arc::new(TestCounter {
                key: key.clone(),
                calls: self.calls.clone(),
            }))
        }

        fn register_gauge(&self, key: &Key, _: &Metadata<'_>) -> Gauge {
            Gauge::from_arc(Arc::new(TestGauge {
                key: key.clone(),
                calls: self.calls.clone(),
            }))
        }

        fn register_histogram(&self, _: &Key, _: &Metadata<'_>) -> Histogram {
            Histogram::noop()
        }
    }

    struct TestCounter {
        key: Key,
        calls: Arc<Mutex<Vec<Call>>>,
    }

    impl CounterFn for TestCounter {
        fn increment(&self, value: u64) {
            self.calls.lock().unwrap().push(Call {
                name: self.key.name().to_string(),
                labels: labels_of(&self.key),
                value: value as f64,
                op: Op::Increment,
            });
        }

        fn absolute(&self, value: u64) {
            self.calls.lock().unwrap().push(Call {
                name: self.key.name().to_string(),
                labels: labels_of(&self.key),
                value: value as f64,
                op: Op::Absolute,
            });
        }
    }

    struct TestGauge {
        key: Key,
        calls: Arc<Mutex<Vec<Call>>>,
    }

    impl GaugeFn for TestGauge {
        fn increment(&self, value: f64) {
            self.calls.lock().unwrap().push(Call {
                name: self.key.name().to_string(),
                labels: labels_of(&self.key),
                value,
                op: Op::Increment,
            });
        }

        fn decrement(&self, value: f64) {
            self.calls.lock().unwrap().push(Call {
                name: self.key.name().to_string(),
                labels: labels_of(&self.key),
                value,
                op: Op::Decrement,
            });
        }

        fn set(&self, value: f64) {
            self.calls.lock().unwrap().push(Call {
                name: self.key.name().to_string(),
                labels: labels_of(&self.key),
                value,
                op: Op::Absolute,
            });
        }
    }

    fn labels_of(key: &Key) -> Vec<(String, String)> {
        key.labels()
            .map(|l| (l.key().to_string(), l.value().to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::{Call, Op, TestRecorder};
    use super::*;
    use std::sync::OnceLock;

    fn find<'a>(calls: &'a [Call], name: &str) -> Vec<&'a Call> {
        calls.iter().filter(|c| c.name == name).collect()
    }

    /// Lazily install a process-wide `TestRecorder` as the metrics
    /// global recorder and return a handle to it. Tests share the
    /// recorder, so each test clears its `calls` buffer first.
    fn install_global_recorder_once() -> TestRecorder {
        static RECORDER: OnceLock<TestRecorder> = OnceLock::new();
        RECORDER
            .get_or_init(|| {
                let r = TestRecorder::default();
                let _ = metrics::set_global_recorder(r.clone());
                r
            })
            .clone()
    }

    #[test]
    fn on_new_port_emits_counter_and_gauge_increment() {
        let r = install_global_recorder_once();
        r.calls.lock().unwrap().clear();
        on_new_port("tcp");
        let calls = r.calls.lock().unwrap();
        let counter = find(&calls, MetricsSpec::PORT_REQ_TOTAL);
        let gauge = find(&calls, MetricsSpec::PORT_USED);
        assert_eq!(counter.len(), 1, "PORT_REQ_TOTAL should fire once");
        assert_eq!(gauge.len(), 1, "PORT_USED should fire once");
        assert_eq!(counter[0].op, Op::Increment);
        assert_eq!(gauge[0].op, Op::Increment);
        assert_eq!(counter[0].value, 1.0);
        assert_eq!(
            gauge[0].labels,
            vec![("type".to_string(), "tcp".to_string())]
        );
    }

    #[test]
    fn on_close_port_decrements_gauge() {
        let r = install_global_recorder_once();
        r.calls.lock().unwrap().clear();
        on_close_port("udp");
        let calls = r.calls.lock().unwrap();
        let g = find(&calls, MetricsSpec::PORT_USED);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].op, Op::Decrement);
        assert_eq!(g[0].value, 1.0);
        assert_eq!(g[0].labels, vec![("type".to_string(), "udp".to_string())]);
    }

    #[test]
    fn on_data_increments_counter_by_byte_count() {
        let r = install_global_recorder_once();
        r.calls.lock().unwrap().clear();
        on_data("ssl-tcp", 42);
        let calls = r.calls.lock().unwrap();
        let c = find(&calls, MetricsSpec::DATA_TOTAL);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].value, 42.0);
        assert_eq!(
            c[0].labels,
            vec![("type".to_string(), "ssl-tcp".to_string())]
        );
    }

    #[test]
    fn on_client_open_bumps_both_client_gauges() {
        let r = install_global_recorder_once();
        r.calls.lock().unwrap().clear();
        on_client_open("tcp");
        let calls = r.calls.lock().unwrap();
        assert_eq!(find(&calls, MetricsSpec::CONNECTED_CLIENT).len(), 1);
        assert_eq!(find(&calls, MetricsSpec::CLIENT).len(), 1);
    }

    #[test]
    fn on_client_close_decrements_both_client_gauges() {
        let r = install_global_recorder_once();
        r.calls.lock().unwrap().clear();
        on_client_close("tcp");
        let calls = r.calls.lock().unwrap();
        let conn = find(&calls, MetricsSpec::CONNECTED_CLIENT);
        let total = find(&calls, MetricsSpec::CLIENT);
        assert_eq!(conn.len(), 1);
        assert_eq!(conn[0].op, Op::Decrement);
        assert_eq!(total.len(), 1);
        assert_eq!(total[0].op, Op::Decrement);
    }

    #[test]
    fn metric_name_constants_are_stable() {
        // Hard-coded values are part of the Prometheus contract; if you
        // change them, scrape configs break.
        assert_eq!(MetricsSpec::PORT_REQ_TOTAL, "port_req_total");
        assert_eq!(MetricsSpec::PORT_USED, "port_used");
        assert_eq!(MetricsSpec::DATA_TOTAL, "data_total");
        assert_eq!(MetricsSpec::CLIENT, "client");
        assert_eq!(MetricsSpec::CONNECTED_CLIENT, "connected_client");
    }
}
