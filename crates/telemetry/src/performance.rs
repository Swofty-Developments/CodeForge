//! Performance measurement utilities: timers, counters, histograms.
//!
//! Provides types for measuring operation latency, tracking throughput,
//! building latency histograms, and generating performance reports.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, Instant};

/// A timer for measuring the duration of an operation.
#[derive(Debug, Clone)]
pub struct PerfTimer {
    /// Label for what is being timed.
    pub label: String,
    /// When the timer was started.
    start: Instant,
    /// When the timer was stopped (if stopped).
    end: Option<Instant>,
}

impl PerfTimer {
    /// Start a new timer.
    pub fn start(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            start: Instant::now(),
            end: None,
        }
    }

    /// Stop the timer and return the elapsed duration.
    pub fn stop(&mut self) -> Duration {
        let now = Instant::now();
        self.end = Some(now);
        now - self.start
    }

    /// Get the elapsed time without stopping.
    pub fn elapsed(&self) -> Duration {
        match self.end {
            Some(end) => end - self.start,
            None => self.start.elapsed(),
        }
    }

    /// Get the elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed().as_secs_f64() * 1000.0
    }

    /// Whether the timer has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.end.is_some()
    }
}

impl fmt::Display for PerfTimer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:.2}ms", self.label, self.elapsed_ms())
    }
}

/// A monotonic counter for tracking event counts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerfCounter {
    /// Label for what is being counted.
    pub label: String,
    /// The current count.
    pub count: u64,
}

impl PerfCounter {
    /// Create a new counter.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            count: 0,
        }
    }

    /// Increment by 1.
    pub fn increment(&mut self) {
        self.count += 1;
    }

    /// Increment by a specific amount.
    pub fn increment_by(&mut self, n: u64) {
        self.count += n;
    }

    /// Reset to zero.
    pub fn reset(&mut self) {
        self.count = 0;
    }
}

impl fmt::Display for PerfCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.label, self.count)
    }
}

/// A histogram for tracking the distribution of latency values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyHistogram {
    /// Label for this histogram.
    pub label: String,
    /// All recorded values in milliseconds.
    values: Vec<f64>,
    /// Bucket boundaries in milliseconds.
    buckets: Vec<f64>,
    /// Count per bucket.
    bucket_counts: Vec<u64>,
}

impl LatencyHistogram {
    /// Create a new histogram with default buckets.
    pub fn new(label: impl Into<String>) -> Self {
        let buckets = vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 5000.0];
        let bucket_counts = vec![0u64; buckets.len() + 1]; // +1 for overflow
        Self {
            label: label.into(),
            values: Vec::new(),
            buckets,
            bucket_counts,
        }
    }

    /// Create a histogram with custom bucket boundaries.
    pub fn with_buckets(label: impl Into<String>, buckets: Vec<f64>) -> Self {
        let bucket_counts = vec![0u64; buckets.len() + 1];
        Self {
            label: label.into(),
            values: Vec::new(),
            buckets,
            bucket_counts,
        }
    }

    /// Record a latency value in milliseconds.
    pub fn record(&mut self, value_ms: f64) {
        self.values.push(value_ms);
        let idx = self
            .buckets
            .iter()
            .position(|&b| value_ms <= b)
            .unwrap_or(self.buckets.len());
        self.bucket_counts[idx] += 1;
    }

    /// Record a duration.
    pub fn record_duration(&mut self, duration: Duration) {
        self.record(duration.as_secs_f64() * 1000.0);
    }

    /// Return the number of recorded values.
    pub fn count(&self) -> usize {
        self.values.len()
    }

    /// Return the minimum value.
    pub fn min(&self) -> Option<f64> {
        self.values.iter().copied().reduce(f64::min)
    }

    /// Return the maximum value.
    pub fn max(&self) -> Option<f64> {
        self.values.iter().copied().reduce(f64::max)
    }

    /// Return the mean (average).
    pub fn mean(&self) -> Option<f64> {
        if self.values.is_empty() {
            return None;
        }
        Some(self.values.iter().sum::<f64>() / self.values.len() as f64)
    }

    /// Return the median (50th percentile).
    pub fn median(&self) -> Option<f64> {
        self.percentile(50.0)
    }

    /// Return a specific percentile (0-100).
    pub fn percentile(&self, p: f64) -> Option<f64> {
        if self.values.is_empty() {
            return None;
        }
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        Some(sorted[idx.min(sorted.len() - 1)])
    }

    /// Return the p50, p90, p95, p99 percentiles.
    pub fn percentiles(&self) -> PercentileSummary {
        PercentileSummary {
            p50: self.percentile(50.0).unwrap_or(0.0),
            p90: self.percentile(90.0).unwrap_or(0.0),
            p95: self.percentile(95.0).unwrap_or(0.0),
            p99: self.percentile(99.0).unwrap_or(0.0),
        }
    }

    /// Return the standard deviation.
    pub fn std_dev(&self) -> Option<f64> {
        let mean = self.mean()?;
        if self.values.len() < 2 {
            return None;
        }
        let variance = self
            .values
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / (self.values.len() - 1) as f64;
        Some(variance.sqrt())
    }

    /// Return bucket counts as (boundary, count) pairs.
    pub fn bucket_data(&self) -> Vec<(String, u64)> {
        let mut data = Vec::new();
        for (i, &boundary) in self.buckets.iter().enumerate() {
            data.push((format!("<={boundary}ms"), self.bucket_counts[i]));
        }
        data.push((
            format!(">{}ms", self.buckets.last().unwrap_or(&0.0)),
            *self.bucket_counts.last().unwrap_or(&0),
        ));
        data
    }
}

impl fmt::Display for LatencyHistogram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pcts = self.percentiles();
        write!(
            f,
            "{}: count={}, p50={:.1}ms, p90={:.1}ms, p99={:.1}ms",
            self.label,
            self.count(),
            pcts.p50,
            pcts.p90,
            pcts.p99
        )
    }
}

/// Summary of percentile values.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PercentileSummary {
    /// 50th percentile.
    pub p50: f64,
    /// 90th percentile.
    pub p90: f64,
    /// 95th percentile.
    pub p95: f64,
    /// 99th percentile.
    pub p99: f64,
}

/// A throughput tracker measuring operations per second.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputTracker {
    /// Label for this tracker.
    pub label: String,
    /// Total operations completed.
    pub total_ops: u64,
    /// Start time (epoch millis).
    start_ms: u64,
    /// Window samples for recent throughput.
    window_samples: Vec<(u64, u64)>, // (timestamp_ms, count)
}

impl ThroughputTracker {
    /// Create a new throughput tracker.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            total_ops: 0,
            start_ms: current_epoch_ms(),
            window_samples: Vec::new(),
        }
    }

    /// Record N completed operations.
    pub fn record(&mut self, count: u64) {
        self.total_ops += count;
        self.window_samples.push((current_epoch_ms(), count));
        // Keep only the last 60 seconds of samples.
        let cutoff = current_epoch_ms().saturating_sub(60_000);
        self.window_samples.retain(|(ts, _)| *ts >= cutoff);
    }

    /// Record a single operation.
    pub fn tick(&mut self) {
        self.record(1);
    }

    /// Return the overall operations per second.
    pub fn overall_ops_per_sec(&self) -> f64 {
        let elapsed_secs = (current_epoch_ms() - self.start_ms) as f64 / 1000.0;
        if elapsed_secs < 0.001 {
            return 0.0;
        }
        self.total_ops as f64 / elapsed_secs
    }

    /// Return the recent (last 60s) operations per second.
    pub fn recent_ops_per_sec(&self) -> f64 {
        if self.window_samples.is_empty() {
            return 0.0;
        }
        let total: u64 = self.window_samples.iter().map(|(_, c)| c).sum();
        let window_ms = self
            .window_samples
            .last()
            .map(|(ts, _)| *ts)
            .unwrap_or(0)
            .saturating_sub(
                self.window_samples
                    .first()
                    .map(|(ts, _)| *ts)
                    .unwrap_or(0),
            );
        let window_secs = window_ms as f64 / 1000.0;
        if window_secs < 0.001 {
            return total as f64;
        }
        total as f64 / window_secs
    }
}

impl fmt::Display for ThroughputTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} total ops, {:.1} ops/s",
            self.label,
            self.total_ops,
            self.overall_ops_per_sec()
        )
    }
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// A performance report combining multiple metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Report title.
    pub title: String,
    /// Counter metrics.
    pub counters: Vec<PerfCounter>,
    /// Latency summaries (label, p50, p90, p99).
    pub latencies: Vec<LatencySummary>,
    /// Throughput metrics.
    pub throughputs: Vec<ThroughputSummary>,
    /// When the report was generated.
    pub generated_at: String,
}

/// Serializable latency summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySummary {
    /// Metric label.
    pub label: String,
    /// Sample count.
    pub count: usize,
    /// Mean latency in ms.
    pub mean_ms: f64,
    /// Percentile summary.
    pub percentiles: PercentileSummary,
}

/// Serializable throughput summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputSummary {
    /// Metric label.
    pub label: String,
    /// Total operations.
    pub total_ops: u64,
    /// Operations per second.
    pub ops_per_sec: f64,
}

impl PerformanceReport {
    /// Create a new report.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            generated_at: Utc::now().to_rfc3339(),
            ..Default::default()
        }
    }

    /// Add a latency histogram to the report.
    pub fn add_histogram(&mut self, histogram: &LatencyHistogram) {
        self.latencies.push(LatencySummary {
            label: histogram.label.clone(),
            count: histogram.count(),
            mean_ms: histogram.mean().unwrap_or(0.0),
            percentiles: histogram.percentiles(),
        });
    }

    /// Add a counter to the report.
    pub fn add_counter(&mut self, counter: PerfCounter) {
        self.counters.push(counter);
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

use chrono::Utc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_basic() {
        let mut timer = PerfTimer::start("test");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();
        assert!(elapsed.as_millis() >= 5);
        assert!(timer.is_stopped());
    }

    #[test]
    fn counter_basic() {
        let mut counter = PerfCounter::new("requests");
        counter.increment();
        counter.increment_by(5);
        assert_eq!(counter.count, 6);
    }

    #[test]
    fn histogram_percentiles() {
        let mut hist = LatencyHistogram::new("api");
        for i in 1..=100 {
            hist.record(i as f64);
        }
        assert_eq!(hist.count(), 100);
        assert!((hist.percentile(50.0).unwrap() - 50.0).abs() < 1.5);
        assert!(hist.percentile(99.0).unwrap() > 95.0);
    }

    #[test]
    fn histogram_min_max_mean() {
        let mut hist = LatencyHistogram::new("test");
        hist.record(10.0);
        hist.record(20.0);
        hist.record(30.0);
        assert_eq!(hist.min(), Some(10.0));
        assert_eq!(hist.max(), Some(30.0));
        assert!((hist.mean().unwrap() - 20.0).abs() < 0.01);
    }
}
