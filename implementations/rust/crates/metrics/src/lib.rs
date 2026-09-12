//! Deterministic, side-effect-free observability hooks for Veridag hot paths.
//!
//! The [`Metrics`] trait is the contract. The default impl is a no-op so that
//! enabling/disabling observability cannot change consensus, cannot introduce
//! non-determinism, and cannot pull heavy dependencies into the Release build.
//! Concrete impls (e.g. text-telegraf, Prometheus push, a per-label counter map)
//! are swapped in by the deployer at the call site; the business crates only
//! depend on the trait and call it.
//!
//! Determinism constraint: `now()` must be a monotonic, reproducible clock. In
//! production this is usually a wall clock; in a deterministic replay / QA the
//! impl can be an event counter so that durations are integer ticks rather than
//! wall seconds, making profiles reproducible. The business code never calls
//! wall-clock APIs itself.

#![forbid(unsafe_code)]

use std::sync::Mutex;
use std::time::Instant;

/// An instantaneous, monotonic clock abstraction so that duration measurements
/// are deterministic under replay and do not depend on a particular wall-clock
/// implementation.
pub trait Clock: Send + Sync {
    /// A monotonic timestamp tick. Ticks must be monotonic and cheap.
    fn now(&self) -> u64;
}

/// A monotonic wall-clock backed by `std::time::Instant`.
#[derive(Debug, Clone, Default)]
pub struct WallClock;

impl Clock for WallClock {
    fn now(&self) -> u64 {
        Instant::now().elapsed().as_nanos() as u64
    }
}

/// Labeling for a single observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub &'static str);

/// A single metric observation.
#[derive(Debug, Clone, Copy)]
pub enum Observation {
    Counter(Label, u64),
    Gauge(Label, i64),
    Duration(Label, u64),
}

/// The observability contract. Default impl must be a no-op.
///
/// All implementations must be `Send + Sync` because the global backend is
/// stored in a `OnceLock` shared across threads. The no-op and counter-map
/// impls both satisfy this.
pub trait Metrics: Send + Sync {
    /// Record a monotonic instant. The impl is free to ignore it.
    fn clock(&self) -> &dyn Clock;

    /// Emit an observation. The default no-op impl must not panic.
    fn observe(&self, obs: Observation);
}

/// A no-op metrics backend. Swapping this for a real backend is the only change
/// required to enable observability; it never reaches into business logic.
#[derive(Debug, Clone, Default)]
pub struct NoOpMetrics;

impl Clock for NoOpMetrics {
    fn now(&self) -> u64 {
        0
    }
}

impl Metrics for NoOpMetrics {
    fn clock(&self) -> &dyn Clock {
        self
    }
    fn observe(&self, _obs: Observation) {}
}

/// A counting / telemetry backend that records observations into per-label
/// counters. Useful in QA/CI to assert that hot paths are exercised the expected
/// number of times without pulling a real telemetry stack.
///
/// Thread-safe via interior mutability so it can be installed as a `&'static
/// dyn Metrics` and called through a shared reference.
#[derive(Debug, Default)]
pub struct CounterMap {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    counters: std::collections::HashMap<&'static str, u64>,
    gauges: std::collections::HashMap<&'static str, i64>,
    durations: Vec<(&'static str, u64)>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("counters", &self.counters.len())
            .field("gauges", &self.gauges.len())
            .field("durations", &self.durations.len())
            .finish()
    }
}

impl CounterMap {
    pub fn counter(&self, label: &str) -> u64 {
        self.inner
            .lock()
            .unwrap()
            .counters
            .get(label)
            .copied()
            .unwrap_or(0)
    }
    pub fn gauge(&self, label: &str) -> i64 {
        self.inner
            .lock()
            .unwrap()
            .gauges
            .get(label)
            .copied()
            .unwrap_or(0)
    }
    pub fn durations(&self) -> Vec<(&'static str, u64)> {
        self.inner.lock().unwrap().durations.clone()
    }
}

impl Metrics for CounterMap {
    fn clock(&self) -> &dyn Clock {
        &WallClock
    }
    fn observe(&self, obs: Observation) {
        let mut inner = self.inner.lock().unwrap();
        match obs {
            Observation::Counter(Label(l), v) => {
                *inner.counters.entry(l).or_default() += v;
            }
            Observation::Gauge(Label(l), v) => {
                inner.gauges.insert(l, v);
            }
            Observation::Duration(Label(l), v) => {
                inner.durations.push((l, v));
            }
        }
    }
}

/// Prometheus and OpenMetrics compliant exposition formatter.
#[derive(Debug, Default)]
pub struct PrometheusExporter {
    map: CounterMap,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a reference to the internal CounterMap.
    pub fn map(&self) -> &CounterMap {
        &self.map
    }

    /// Render all recorded metrics in canonical Prometheus text format.
    pub fn render(&self) -> String {
        let inner = self.map.inner.lock().unwrap();
        let mut out = String::new();

        // Sort keys for deterministic output
        let mut sorted_counters: Vec<_> = inner.counters.iter().collect();
        sorted_counters.sort_by_key(|(k, _)| *k);

        for (name, val) in sorted_counters {
            let clean_name = name.replace('-', "_");
            out.push_str(&format!(
                "# HELP veridag_{clean_name} Veridag metric {clean_name}\n"
            ));
            out.push_str(&format!("# TYPE veridag_{clean_name} counter\n"));
            out.push_str(&format!("veridag_{clean_name} {val}\n\n"));
        }

        let mut sorted_gauges: Vec<_> = inner.gauges.iter().collect();
        sorted_gauges.sort_by_key(|(k, _)| *k);

        for (name, val) in sorted_gauges {
            let clean_name = name.replace('-', "_");
            out.push_str(&format!(
                "# HELP veridag_{clean_name} Veridag metric {clean_name}\n"
            ));
            out.push_str(&format!("# TYPE veridag_{clean_name} gauge\n"));
            out.push_str(&format!("veridag_{clean_name} {val}\n\n"));
        }

        out
    }
}

impl Metrics for PrometheusExporter {
    fn clock(&self) -> &dyn Clock {
        self.map.clock()
    }

    fn observe(&self, obs: Observation) {
        self.map.observe(obs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_metrics_is_a_true_no_op() {
        let m = NoOpMetrics;
        assert_eq!(m.clock().now(), 0);
        m.observe(Observation::Counter(Label("k"), 42));
        m.observe(Observation::Gauge(Label("g"), -1));
        m.observe(Observation::Duration(Label("d"), 99));
    }

    #[test]
    fn counter_map_records_observations() {
        let m = CounterMap::default();
        m.observe(Observation::Counter(Label("sign_batches"), 1));
        m.observe(Observation::Counter(Label("sign_batches"), 2));
        m.observe(Observation::Gauge(Label("epoch"), 7));
        m.observe(Observation::Duration(Label("hash"), 123));
        assert_eq!(m.counter("sign_batches"), 3);
        assert_eq!(m.gauge("epoch"), 7);
        assert_eq!(m.durations().len(), 1);
        assert_eq!(m.durations()[0], ("hash", 123));
    }

    #[test]
    fn counter_map_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CounterMap>();
    }

    #[test]
    fn test_prometheus_exporter_renders_openmetrics_format() {
        let exporter = PrometheusExporter::new();
        exporter.observe(Observation::Counter(Label("consensus_commits"), 42));
        exporter.observe(Observation::Gauge(Label("current_epoch"), 3));

        let rendered = exporter.render();
        assert!(rendered.contains("# HELP veridag_consensus_commits"));
        assert!(rendered.contains("# TYPE veridag_consensus_commits counter"));
        assert!(rendered.contains("veridag_consensus_commits 42"));

        assert!(rendered.contains("# HELP veridag_current_epoch"));
        assert!(rendered.contains("# TYPE veridag_current_epoch gauge"));
        assert!(rendered.contains("veridag_current_epoch 3"));
    }
}
