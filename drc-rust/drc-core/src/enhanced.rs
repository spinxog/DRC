use serde::{Deserialize, Serialize};

/// Diff dimensions for comparing replays
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiffDimension {
    Output,
    StateWrite,
    Path,
    DependencyCall,
    Exception,
    Timing,
}

/// Fidelity report for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FidelityReport {
    pub overall: super::ConfidenceLevel,
    pub capture_completeness: f64,
    pub state_reconstruction_completeness: f64,
    pub hidden_read_risk: String, // NONE, LOW, MEDIUM, HIGH
    pub artifact_mismatch: String, // NONE, COMPATIBLE, MISMATCH
    pub unsupported_features: Vec<String>,
    pub downgrade_reasons: Vec<String>,
    pub confidence_score: f64,
}

/// Replay diff entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayDiff {
    pub sequence: u64,
    pub event_id: super::EventId,
    pub diff_type: super::EventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub dimension: DiffDimension,
    pub original: serde_json::Value,
    pub replayed: serde_json::Value,
    pub severity: String, // INFO, WARNING, CRITICAL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

/// Divergence context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_type: Option<super::EventType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_type: Option<super::EventType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<TimingInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingInfo {
    pub original: u64,
    pub replayed: u64,
    pub delta: i64,
}

/// Divergence point in causal chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergencePoint {
    pub sequence: u64,
    pub event_id: super::EventId,
    pub point_type: super::EventType,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_input: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_read: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_branch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_write: Option<bool>,
}

/// Changed value tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedValue {
    pub event_id: super::EventId,
    pub sequence: u64,
    pub name: String,
    pub original: serde_json::Value,
    pub replayed: serde_json::Value,
    pub path: String,
}

/// Changed branch tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedBranch {
    pub event_id: super::EventId,
    pub sequence: u64,
    pub condition: String,
    pub original_branch: String,
    pub replayed_branch: String,
    pub path: String,
}

/// Root cause analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCauseAnalysis {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_changed_input: Option<ChangedValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_changed_read: Option<ChangedValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_changed_branch: Option<ChangedBranch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_changed_write: Option<ChangedValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downstream_effects: Option<Vec<String>>,
    pub confidence: f64,
    pub explanation: String,
}

/// Divergence report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceReport {
    pub execution_id: super::ExecutionId,
    pub replay_id: String,
    pub first_divergence_event_sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_divergence_event_id: Option<super::EventId>,
    pub divergence_type: super::DivergenceType,
    pub original_event: super::DRCEvent,
    pub replayed_event: super::DRCEvent,
    pub context: DivergenceContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_chain: Option<Vec<DivergencePoint>>,
    pub severity: String, // INFO, WARNING, CRITICAL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_actions: Option<Vec<String>>,
}

/// Replay result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub execution_id: super::ExecutionId,
    pub replay_id: String,
    pub success: bool,
    pub state: super::ExecutionState,
    pub confidence: super::ConfidenceLevel,
    pub fidelity: FidelityReport,
    pub divergences: Vec<DivergenceReport>,
    pub diffs: Vec<ReplayDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_divergence: Option<DivergenceReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_cause: Option<RootCauseAnalysis>,
    pub side_effects_blocked: Vec<super::SideEffectRecord>,
    pub side_effects_simulated: Vec<super::SideEffectRecord>,
    pub timing: ReplayTiming,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<ExecutionLog>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<ReplayMetrics>,
}

/// Replay timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayTiming {
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_duration_ms: Option<u64>,
    pub timing_variance: f64,
    pub virtual_time_advancement: u64,
}

/// Execution log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub timestamp: i64,
    pub level: LogLevel,
    pub message: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Replay metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayMetrics {
    pub events_replayed: u64,
    pub events_diverged: u64,
    pub events_mutated: u64,
    pub side_effects_blocked: u64,
    pub side_effects_simulated: u64,
    pub state_reconstruction_time_ms: u64,
    pub replay_execution_time_ms: u64,
    pub diff_computation_time_ms: u64,
}

/// Search query
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<super::ExecutionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<super::TraceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replayable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity: Option<Vec<super::ConfidenceLevel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Vec<super::ExecutionState>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_divergence: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_side_effects: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub executions: Vec<ExecutionSummary>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<SearchFacets>,
}

/// Execution summary for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub execution_id: super::ExecutionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<super::TraceId>,
    pub service_name: String,
    pub start_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    pub duration: u64,
    pub state: super::ExecutionState,
    pub confidence: super::ConfidenceLevel,
    pub event_count: u64,
    pub has_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_divergence_sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

/// Search facets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    pub services: Vec<FacetCount<String>>,
    pub states: Vec<FacetCount<super::ExecutionState>>,
    pub confidences: Vec<FacetCount<super::ConfidenceLevel>>,
    pub error_classes: Vec<FacetCount<String>>,
    pub time_distribution: Vec<FacetCount<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCount<T> {
    pub value: T,
    pub count: u64,
}

/// Retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub hot_tier_days: u32,
    pub warm_tier_days: u32,
    pub cold_tier_days: u32,
    pub auto_expire_days: u32,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub legal_hold_available: bool,
}

/// Privacy policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyPolicy {
    pub pii_detection_enabled: bool,
    pub auto_redaction_enabled: bool,
    pub field_redaction_rules: Vec<RedactionRule>,
    pub retention_override_allowed: bool,
    pub deletion_requests_supported: bool,
    pub data_residency: String,
    pub customer_managed_keys: bool,
}

/// Redaction rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionRule {
    pub field_pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_types: Option<Vec<super::EventType>>,
    pub redaction_type: RedactionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RedactionType {
    Full,
    Partial,
    Hash,
}

/// Security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub auth_model: AuthModel,
    pub tenant_isolation: TenantIsolation,
    pub field_encryption: bool,
    pub audit_logging: bool,
    pub role_permissions: Vec<RolePermission>,
    pub approval_gates: Vec<ApprovalGate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthModel {
    ApiKey,
    OAuth,
    Mtls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantIsolation {
    Database,
    Schema,
    RowLevel,
}

/// Role permission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePermission {
    pub role: String,
    pub actions: Vec<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_filter: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    ViewCapture,
    CreateReplay,
    MutateReplay,
    SwapArtifact,
    AllowLiveAccess,
    ExportData,
    DeleteExecution,
}

/// Approval gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalGate {
    pub action: String,
    pub required_approvers: u32,
    pub approver_roles: Vec<String>,
    pub auto_expire_hours: u32,
}

/// Performance budget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBudget {
    pub max_latency_overhead_ms: u64,
    pub max_cpu_overhead_percent: f64,
    pub max_memory_overhead_mb: u64,
    pub max_storage_amplification: f64,
    pub sampling_rate: f64,
    pub adaptive_throttling_enabled: bool,
    pub strict_capture_mode_available: bool,
}

/// Ingestion config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub max_retries: u32,
    pub backpressure_strategy: BackpressureStrategy,
    pub out_of_order_handling: OutOfOrderHandling,
    pub duplicate_handling: DuplicateHandling,
    pub integrity_checks: bool,
    pub exactly_once: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackpressureStrategy {
    Drop,
    Sample,
    Buffer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutOfOrderHandling {
    Reorder,
    TimestampIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DuplicateHandling {
    Dedup,
    Allow,
}

/// Metrics config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub capture_coverage_enabled: bool,
    pub replay_success_rate_enabled: bool,
    pub fidelity_levels_enabled: bool,
    pub storage_amplification_enabled: bool,
    pub ingest_lag_enabled: bool,
    pub agent_drop_rate_enabled: bool,
    pub replay_overhead_enabled: bool,
    pub alerting_enabled: bool,
}
