//! Evaluation metrics, results, and test suite running.
//!
//! Provides types for evaluating AI model outputs against expected
//! results, tracking metrics like correctness and helpfulness, and
//! running evaluation suites with comparison helpers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A metric used to evaluate model output quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvalMetric {
    /// Whether the output is factually correct.
    Correctness,
    /// Whether the output is helpful and addresses the user's need.
    Helpfulness,
    /// Whether the output avoids harmful content.
    Safety,
    /// Whether the output follows the requested format.
    FormatCompliance,
    /// Whether the output is concise without losing information.
    Conciseness,
    /// Whether code output is syntactically valid.
    CodeValidity,
    /// Whether the output demonstrates proper reasoning.
    Reasoning,
    /// Latency / time to generate.
    Latency,
    /// Token efficiency (quality per token).
    TokenEfficiency,
}

impl EvalMetric {
    /// Return all available metrics.
    pub fn all() -> &'static [EvalMetric] {
        &[
            EvalMetric::Correctness,
            EvalMetric::Helpfulness,
            EvalMetric::Safety,
            EvalMetric::FormatCompliance,
            EvalMetric::Conciseness,
            EvalMetric::CodeValidity,
            EvalMetric::Reasoning,
            EvalMetric::Latency,
            EvalMetric::TokenEfficiency,
        ]
    }

    /// Return the default weight for this metric in an overall score.
    pub fn default_weight(&self) -> f64 {
        match self {
            EvalMetric::Correctness => 3.0,
            EvalMetric::Helpfulness => 2.0,
            EvalMetric::Safety => 3.0,
            EvalMetric::FormatCompliance => 1.0,
            EvalMetric::Conciseness => 1.0,
            EvalMetric::CodeValidity => 2.0,
            EvalMetric::Reasoning => 2.0,
            EvalMetric::Latency => 0.5,
            EvalMetric::TokenEfficiency => 0.5,
        }
    }
}

impl fmt::Display for EvalMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalMetric::Correctness => write!(f, "correctness"),
            EvalMetric::Helpfulness => write!(f, "helpfulness"),
            EvalMetric::Safety => write!(f, "safety"),
            EvalMetric::FormatCompliance => write!(f, "format-compliance"),
            EvalMetric::Conciseness => write!(f, "conciseness"),
            EvalMetric::CodeValidity => write!(f, "code-validity"),
            EvalMetric::Reasoning => write!(f, "reasoning"),
            EvalMetric::Latency => write!(f, "latency"),
            EvalMetric::TokenEfficiency => write!(f, "token-efficiency"),
        }
    }
}

/// A score on a single metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScore {
    /// The metric being scored.
    pub metric: EvalMetric,
    /// The score (0.0-1.0).
    pub score: f64,
    /// Optional explanation for the score.
    pub explanation: Option<String>,
    /// Confidence in the score (0.0-1.0).
    pub confidence: f64,
}

impl MetricScore {
    /// Create a new metric score.
    pub fn new(metric: EvalMetric, score: f64) -> Self {
        Self {
            metric,
            score: score.clamp(0.0, 1.0),
            explanation: None,
            confidence: 1.0,
        }
    }

    /// Add an explanation.
    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = Some(explanation.into());
        self
    }

    /// Set confidence level.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Whether this score passes a threshold.
    pub fn passes(&self, threshold: f64) -> bool {
        self.score >= threshold
    }
}

impl fmt::Display for MetricScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:.2}", self.metric, self.score)?;
        if let Some(ref explanation) = self.explanation {
            write!(f, " ({explanation})")?;
        }
        Ok(())
    }
}

/// The result of evaluating a single test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// The test case ID.
    pub test_case_id: String,
    /// The model used.
    pub model: String,
    /// Per-metric scores.
    pub scores: Vec<MetricScore>,
    /// The overall weighted score.
    pub overall_score: f64,
    /// Whether the test case passed.
    pub passed: bool,
    /// The model's output.
    pub output: String,
    /// The expected output (if any).
    pub expected: Option<String>,
    /// Time to generate in milliseconds.
    pub latency_ms: u64,
    /// Tokens consumed.
    pub tokens_used: usize,
}

impl EvalResult {
    /// Create a new eval result.
    pub fn new(test_case_id: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            test_case_id: test_case_id.into(),
            model: model.into(),
            scores: Vec::new(),
            overall_score: 0.0,
            passed: false,
            output: String::new(),
            expected: None,
            latency_ms: 0,
            tokens_used: 0,
        }
    }

    /// Add a metric score.
    pub fn add_score(&mut self, score: MetricScore) {
        self.scores.push(score);
        self.recalculate_overall();
    }

    /// Get the score for a specific metric.
    pub fn get_score(&self, metric: EvalMetric) -> Option<f64> {
        self.scores.iter().find(|s| s.metric == metric).map(|s| s.score)
    }

    /// Recalculate the overall weighted score.
    fn recalculate_overall(&mut self) {
        let total_weight: f64 = self.scores.iter().map(|s| s.metric.default_weight()).sum();
        if total_weight == 0.0 {
            self.overall_score = 0.0;
            return;
        }
        let weighted_sum: f64 = self
            .scores
            .iter()
            .map(|s| s.score * s.metric.default_weight())
            .sum();
        self.overall_score = weighted_sum / total_weight;
    }

    /// Mark the result as passing or failing based on a threshold.
    pub fn evaluate(&mut self, threshold: f64) {
        self.recalculate_overall();
        self.passed = self.overall_score >= threshold;
    }
}

impl fmt::Display for EvalResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        write!(
            f,
            "[{status}] {} ({}): overall={:.2}, {}ms, {} tokens",
            self.test_case_id, self.model, self.overall_score, self.latency_ms, self.tokens_used
        )
    }
}

/// A test case for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTestCase {
    /// Unique identifier for this test case.
    pub id: String,
    /// The input prompt.
    pub prompt: String,
    /// The expected output (for exact or fuzzy matching).
    pub expected_output: Option<String>,
    /// Which metrics to evaluate.
    pub metrics: Vec<EvalMetric>,
    /// Tags for categorizing test cases.
    pub tags: Vec<String>,
    /// Custom metadata.
    pub metadata: HashMap<String, String>,
}

impl EvalTestCase {
    /// Create a new test case.
    pub fn new(id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            expected_output: None,
            metrics: vec![EvalMetric::Correctness, EvalMetric::Helpfulness],
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the expected output.
    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected_output = Some(expected.into());
        self
    }

    /// Set the metrics to evaluate.
    pub fn with_metrics(mut self, metrics: Vec<EvalMetric>) -> Self {
        self.metrics = metrics;
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// An evaluation suite containing multiple test cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuite {
    /// Name of the evaluation suite.
    pub name: String,
    /// Description of what this suite evaluates.
    pub description: Option<String>,
    /// The test cases.
    pub test_cases: Vec<EvalTestCase>,
    /// Passing threshold (0.0-1.0).
    pub pass_threshold: f64,
    /// Models to evaluate.
    pub models: Vec<String>,
}

impl EvalSuite {
    /// Create a new evaluation suite.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            test_cases: Vec::new(),
            pass_threshold: 0.7,
            models: Vec::new(),
        }
    }

    /// Add a test case.
    pub fn add_case(&mut self, case: EvalTestCase) {
        self.test_cases.push(case);
    }

    /// Add a model to evaluate.
    pub fn add_model(&mut self, model: impl Into<String>) {
        self.models.push(model.into());
    }

    /// Set the passing threshold.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.pass_threshold = threshold;
        self
    }

    /// Return the number of test cases.
    pub fn case_count(&self) -> usize {
        self.test_cases.len()
    }

    /// Filter test cases by tag.
    pub fn cases_with_tag(&self, tag: &str) -> Vec<&EvalTestCase> {
        self.test_cases
            .iter()
            .filter(|c| c.tags.iter().any(|t| t == tag))
            .collect()
    }
}

/// A comparison between two model evaluation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelComparison {
    /// Model A identifier.
    pub model_a: String,
    /// Model B identifier.
    pub model_b: String,
    /// Per-metric comparison.
    pub metric_comparisons: Vec<MetricComparison>,
    /// Overall score for model A.
    pub overall_a: f64,
    /// Overall score for model B.
    pub overall_b: f64,
    /// Which model performed better overall.
    pub winner: Option<String>,
}

/// Comparison of a single metric between two models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    /// The metric.
    pub metric: EvalMetric,
    /// Model A's average score.
    pub score_a: f64,
    /// Model B's average score.
    pub score_b: f64,
    /// The difference (A - B).
    pub difference: f64,
}

impl fmt::Display for MetricComparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arrow = if self.difference > 0.01 {
            ">"
        } else if self.difference < -0.01 {
            "<"
        } else {
            "="
        };
        write!(
            f,
            "{}: {:.2} {} {:.2} (diff: {:+.2})",
            self.metric, self.score_a, arrow, self.score_b, self.difference
        )
    }
}

/// Compare evaluation results between two models.
pub fn compare_models(
    model_a: &str,
    results_a: &[EvalResult],
    model_b: &str,
    results_b: &[EvalResult],
) -> ModelComparison {
    let avg_a = if results_a.is_empty() {
        0.0
    } else {
        results_a.iter().map(|r| r.overall_score).sum::<f64>() / results_a.len() as f64
    };
    let avg_b = if results_b.is_empty() {
        0.0
    } else {
        results_b.iter().map(|r| r.overall_score).sum::<f64>() / results_b.len() as f64
    };

    let mut metric_comparisons = Vec::new();
    for metric in EvalMetric::all() {
        let score_a = results_a
            .iter()
            .filter_map(|r| r.get_score(*metric))
            .sum::<f64>()
            / results_a.len().max(1) as f64;
        let score_b = results_b
            .iter()
            .filter_map(|r| r.get_score(*metric))
            .sum::<f64>()
            / results_b.len().max(1) as f64;
        metric_comparisons.push(MetricComparison {
            metric: *metric,
            score_a,
            score_b,
            difference: score_a - score_b,
        });
    }

    let winner = if (avg_a - avg_b).abs() < 0.01 {
        None
    } else if avg_a > avg_b {
        Some(model_a.to_string())
    } else {
        Some(model_b.to_string())
    };

    ModelComparison {
        model_a: model_a.to_string(),
        model_b: model_b.to_string(),
        metric_comparisons,
        overall_a: avg_a,
        overall_b: avg_b,
        winner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_result_scoring() {
        let mut result = EvalResult::new("test-1", "claude-sonnet-4");
        result.add_score(MetricScore::new(EvalMetric::Correctness, 0.9));
        result.add_score(MetricScore::new(EvalMetric::Helpfulness, 0.8));
        result.evaluate(0.7);
        assert!(result.passed);
        assert!(result.overall_score > 0.8);
    }

    #[test]
    fn metric_passes_threshold() {
        let score = MetricScore::new(EvalMetric::Safety, 0.95);
        assert!(score.passes(0.9));
        assert!(!score.passes(0.99));
    }

    #[test]
    fn model_comparison() {
        let results_a = vec![{
            let mut r = EvalResult::new("t1", "model-a");
            r.add_score(MetricScore::new(EvalMetric::Correctness, 0.9));
            r.evaluate(0.5);
            r
        }];
        let results_b = vec![{
            let mut r = EvalResult::new("t1", "model-b");
            r.add_score(MetricScore::new(EvalMetric::Correctness, 0.7));
            r.evaluate(0.5);
            r
        }];
        let comparison = compare_models("model-a", &results_a, "model-b", &results_b);
        assert_eq!(comparison.winner.as_deref(), Some("model-a"));
    }
}
