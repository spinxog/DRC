use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;
use drc_core::*;
use drc_runtime::ReplayRuntime;

/// Mutation engine implementation
pub struct MutationEngineImpl {
    _runtime: Arc<ReplayRuntime>,
}

impl MutationEngineImpl {
    pub fn new(runtime: Arc<ReplayRuntime>) -> Self {
        Self { _runtime: runtime }
    }

    /// Apply event mutation
    fn apply_event_mutation(
        &self,
        event: &mut DRCEvent,
        mutation: &EventMutation,
    ) -> anyhow::Result<()> {
        if event.event_type == mutation.event_type {
            event.data = mutation.replacement.clone();
            event.metadata = Some(EventMetadata {
                mutated: Some(true),
                mutation_type: Some(format!("{:?}", mutation.mutation_type)),
                mutation_description: Some(mutation.description.clone()),
                ..Default::default()
            });
        }
        Ok(())
    }

    /// Apply timeout injection
    fn apply_timeout_injection(
        &self,
        event: &mut DRCEvent,
        injection: &TimeoutInjection,
    ) -> anyhow::Result<()> {
        if event.event_type == injection.target_event_type {
            // Modify event to include timeout delay
            if let Some(ref mut metadata) = event.metadata {
                metadata.payload_size = Some(injection.delay_ms as usize);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl MutationEngine for MutationEngineImpl {
    async fn apply_mutations(
        &self,
        mut events: Vec<DRCEvent>,
        spec: &MutationSpec,
    ) -> anyhow::Result<Vec<DRCEvent>> {
        // Apply event mutations
        if let Some(ref swaps) = spec.swaps {
            for event in &mut events {
                for mutation in swaps {
                    self.apply_event_mutation(event, mutation)?;
                }
            }
        }

        // Apply timeout injections
        if let Some(ref injections) = spec.timeout_injections {
            for event in &mut events {
                for injection in injections {
                    self.apply_timeout_injection(event, injection)?;
                }
            }
        }

        info!("Applied mutations to {} events", events.len());
        Ok(events)
    }

    async fn validate_spec(&self, spec: &MutationSpec) -> anyhow::Result<Vec<String>> {
        let mut violations = Vec::new();

        // Validate timeout injections
        if let Some(ref injections) = spec.timeout_injections {
            for injection in injections {
                if injection.probability < 0.0 || injection.probability > 1.0 {
                    violations.push(format!(
                        "Invalid probability {} for timeout injection",
                        injection.probability
                    ));
                }
            }
        }

        // Validate rules
        if let Some(ref rules) = spec.validation_rules {
            for rule in rules {
                if rule.severity == "ERROR" {
                    violations.push(rule.message.clone());
                }
            }
        }

        Ok(violations)
    }
}

/// Diff engine implementation
pub struct DiffEngineImpl;

impl DiffEngineImpl {
    pub fn new() -> Self {
        Self
    }

    /// Compare two events and generate diff if different
    fn compare_events(
        &self,
        original: &DRCEvent,
        replayed: &DRCEvent,
    ) -> Option<ReplayDiff> {
        if original.event_type != replayed.event_type {
            return Some(ReplayDiff {
                sequence: replayed.sequence,
                event_id: replayed.id.clone(),
                diff_type: replayed.event_type,
                path: Some("type".to_string()),
                dimension: DiffDimension::Path,
                original: serde_json::json!(original.event_type),
                replayed: serde_json::json!(replayed.event_type),
                severity: "CRITICAL".to_string(),
                normalized: None,
                explanation: Some("Event type mismatch".to_string()),
            });
        }

        if original.data != replayed.data {
            return Some(ReplayDiff {
                sequence: replayed.sequence,
                event_id: replayed.id.clone(),
                diff_type: replayed.event_type,
                path: Some("data".to_string()),
                dimension: DiffDimension::Output,
                original: original.data.clone(),
                replayed: replayed.data.clone(),
                severity: "WARNING".to_string(),
                normalized: None,
                explanation: Some("Event data differs".to_string()),
            });
        }

        None
    }
}

impl Default for DiffEngineImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DiffEngine for DiffEngineImpl {
    async fn compute_diff(
        &self,
        original: &[DRCEvent],
        replayed: &[DRCEvent],
    ) -> anyhow::Result<Vec<ReplayDiff>> {
        let mut diffs = Vec::new();
        let min_len = original.len().min(replayed.len());

        for i in 0..min_len {
            if let Some(diff) = self.compare_events(&original[i], &replayed[i]) {
                diffs.push(diff);
            }
        }

        // Handle length mismatch
        if original.len() > replayed.len() {
            for i in replayed.len()..original.len() {
                diffs.push(ReplayDiff {
                    sequence: original[i].sequence,
                    event_id: original[i].id.clone(),
                    diff_type: original[i].event_type,
                    path: Some("missing".to_string()),
                    dimension: DiffDimension::Path,
                    original: serde_json::json!("present"),
                    replayed: serde_json::json!("missing"),
                    severity: "CRITICAL".to_string(),
                    normalized: None,
                    explanation: Some("Event missing in replay".to_string()),
                });
            }
        } else if replayed.len() > original.len() {
            for i in original.len()..replayed.len() {
                diffs.push(ReplayDiff {
                    sequence: replayed[i].sequence,
                    event_id: replayed[i].id.clone(),
                    diff_type: replayed[i].event_type,
                    path: Some("extra".to_string()),
                    dimension: DiffDimension::Path,
                    original: serde_json::json!("missing"),
                    replayed: serde_json::json!("present"),
                    severity: "INFO".to_string(),
                    normalized: None,
                    explanation: Some("Extra event in replay".to_string()),
                });
            }
        }

        Ok(diffs)
    }

    async fn find_first_divergence(
        &self,
        original: &[DRCEvent],
        replayed: &[DRCEvent],
    ) -> anyhow::Result<Option<DivergenceReport>> {
        let min_len = original.len().min(replayed.len());

        for i in 0..min_len {
            if original[i].event_type != replayed[i].event_type
                || original[i].data != replayed[i].data
            {
                return Ok(Some(DivergenceReport {
                    execution_id: original[i].execution_id.clone(),
                    replay_id: format!("replay_{}", original[i].execution_id),
                    first_divergence_event_sequence: replayed[i].sequence,
                    first_divergence_event_id: Some(replayed[i].id.clone()),
                    divergence_type: DivergenceType::Data,
                    original_event: original[i].clone(),
                    replayed_event: replayed[i].clone(),
                    context: DivergenceContext {
                        expected_type: Some(original[i].event_type),
                        actual_type: Some(replayed[i].event_type),
                        expected_value: Some(original[i].data.clone()),
                        actual_value: Some(replayed[i].data.clone()),
                        path: Some(format!("event_{}", i)),
                        error: Some("First divergence detected".to_string()),
                        timing: None,
                        stack_trace: None,
                    },
                    causal_chain: None,
                    severity: "CRITICAL".to_string(),
                    explanation: Some(format!(
                        "First divergence at sequence {}",
                        replayed[i].sequence
                    )),
                    suggested_actions: Some(vec!["Check input mutations".to_string()]),
                }));
            }
        }

        Ok(None)
    }

    async fn analyze_root_cause(
        &self,
        divergences: &[DivergenceReport],
    ) -> anyhow::Result<Option<RootCauseAnalysis>> {
        if divergences.is_empty() {
            return Ok(None);
        }

        let first = &divergences[0];

        Ok(Some(RootCauseAnalysis {
            first_changed_input: None,
            first_changed_read: Some(ChangedValue {
                event_id: first.first_divergence_event_id.clone().unwrap_or_default(),
                sequence: first.first_divergence_event_sequence,
                name: "first_divergence".to_string(),
                original: first.context.expected_value.clone().unwrap_or(serde_json::json!({})),
                replayed: first.context.actual_value.clone().unwrap_or(serde_json::json!({})),
                path: first.context.path.clone().unwrap_or_default(),
            }),
            first_changed_branch: None,
            first_changed_write: None,
            downstream_effects: Some(
                divergences.iter().skip(1).map(|d| d.explanation.clone().unwrap_or_default()).collect()
            ),
            confidence: 0.8,
            explanation: format!(
                "Root cause: First divergence at sequence {} - {}",
                first.first_divergence_event_sequence,
                first.explanation.clone().unwrap_or_default()
            ),
        }))
    }
}

/// Root cause analyzer
pub struct RootCauseAnalyzer;

impl RootCauseAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, divergences: &[DivergenceReport]) -> Option<RootCauseAnalysis> {
        if divergences.is_empty() {
            return None;
        }

        let first = &divergences[0];

        Some(RootCauseAnalysis {
            first_changed_input: None,
            first_changed_read: Some(ChangedValue {
                event_id: first.first_divergence_event_id.clone().unwrap_or_default(),
                sequence: first.first_divergence_event_sequence,
                name: "first_divergence".to_string(),
                original: first.context.expected_value.clone().unwrap_or(serde_json::json!({})),
                replayed: first.context.actual_value.clone().unwrap_or(serde_json::json!({})),
                path: first.context.path.clone().unwrap_or_default(),
            }),
            first_changed_branch: None,
            first_changed_write: None,
            downstream_effects: Some(
                divergences.iter().skip(1).map(|d| d.explanation.clone().unwrap_or_default()).collect()
            ),
            confidence: 0.85,
            explanation: format!(
                "Analysis: First divergence occurred at sequence {}",
                first.first_divergence_event_sequence
            ),
        })
    }
}

impl Default for RootCauseAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

pub use drc_core::MutationEngine;
pub use drc_core::DiffEngine;
