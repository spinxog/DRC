use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type aliases for domain identifiers
pub type ExecutionId = String;
pub type TraceId = String;
pub type EventId = String;
pub type SpanId = String;
pub type ParentId = String;

/// Execution state in the lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionState {
    Started,
    Capturing,
    Finalized,
    Indexed,
    Replayable,
    Partial,
    Corrupted,
    Expired,
    Replaying,
    Replayed,
    Failed,
}

/// Types of events that can occur during execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    // Inbound/Outbound
    RequestStart,
    RequestEnd,
    
    // DB Operations
    DbRead,
    DbWrite,
    DbQuery,
    DbTransactionBegin,
    DbTransactionCommit,
    DbTransactionRollback,
    
    // Cache Operations
    CacheRead,
    CacheWrite,
    CacheDelete,
    
    // Queue Operations
    QueuePublish,
    QueueConsume,
    QueueAck,
    
    // File Operations
    FileRead,
    FileWrite,
    FileDelete,
    FileStat,
    
    // Network Operations
    NetworkCall,
    NetworkResponse,
    
    // Config/Feature Flags
    ConfigRead,
    FeatureFlagResolution,
    
    // Non-deterministic
    ClockRead,
    ClockMonotonicRead,
    RandomRead,
    UuidGeneration,
    
    // Function/Flow
    FunctionEnter,
    FunctionExit,
    FunctionMarker,
    SpanStart,
    SpanEnd,
    
    // Errors
    Error,
    Exception,
    
    // System
    Log,
    Checkpoint,
    Snapshot,
    SideEffectBlocked,
    
    // Causal
    ChildExecutionStart,
    ChildExecutionEnd,
    FanOut,
    FanIn,
    
    // Kernel/System
    Syscall,
}

/// Confidence level for replay fidelity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConfidenceLevel {
    Exact,
    High,
    Medium,
    Low,
    Approximate,
    Invalid,
}

/// Storage tier for data retention
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageTier {
    Hot,
    Warm,
    Cold,
    Expired,
}

/// Types of divergence during replay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DivergenceType {
    Data,
    ControlFlow,
    Timeout,
    Error,
    Timing,
    Output,
    StateWrite,
    Path,
    DependencyCall,
    Exception,
}

/// Replay mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReplayMode {
    Strict,
    Adaptive,
    Mutated,
    Approximate,
}

/// Side effect classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SideEffectClass {
    DbWrite,
    CacheWrite,
    QueuePublish,
    FileWrite,
    EmailSend,
    WebhookSend,
    ThirdPartyMutation,
    PaymentCall,
    NotificationSend,
}

/// Policy for handling side effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SideEffectPolicy {
    Block,
    Sink,
    Simulate,
    AllowExplicit,
}

/// Mutation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MutationType {
    PayloadMutation,
    HeaderMutation,
    AuthMutation,
    ConfigMutation,
    FlagMutation,
    DependencyResponseMutation,
    TimeoutInjection,
    ArtifactSwap,
    SchedulerMutation,
    DbRowMutation,
    QueueReorder,
    CodePatchInjection,
}

/// The main DRC event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DRCEvent {
    pub id: EventId,
    pub execution_id: ExecutionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<TraceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<SpanId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<SpanId>,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monotonic_timestamp: Option<i64>,
    pub sequence: u64,
    pub event_type: EventType,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<EventMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_parent_ids: Option<Vec<EventId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

/// Metadata for events
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_hash: Option<String>,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub execution_id: ExecutionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<TraceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_execution_id: Option<ExecutionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_execution_id: Option<ExecutionId>,
    pub service_name: String,
    pub instance_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub environment: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockfile_hash: Option<String>,
    pub runtime_version: String,
    pub start_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    pub state: ExecutionState,
    pub capture_completeness: f64,
    pub replay_confidence: ConfidenceLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_snapshots: Option<Vec<SnapshotReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_references: Option<Vec<ArtifactReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_snapshot: Option<ConfigSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_flag_values: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars_read: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_references: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side_effects_emitted: Option<Vec<SideEffectRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_read_risk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_downgrade_reasons: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_tier: Option<StorageTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_replayed_at: Option<i64>,
    pub replay_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_classification: Option<String>,
}

/// Snapshot reference for state reconstruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotReference {
    pub snapshot_type: String, // DB, CACHE, FILE, STATE
    pub resource_id: String,
    pub version: String,
    pub snapshot_id: String,
    pub checksum: String,
    pub captured_at: i64,
}

/// Artifact reference (container, binary, package)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactReference {
    pub artifact_type: String, // CONTAINER, BINARY, PACKAGE, SOURCE
    pub digest: String,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>, // EXACT, COMPATIBLE, MISMATCH
}

/// Config snapshot at capture time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub values: HashMap<String, serde_json::Value>,
    pub captured_at: i64,
    pub source: String,
}

/// Side effect record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideEffectRecord {
    pub effect_type: SideEffectClass,
    pub event_id: EventId,
    pub timestamp: i64,
    pub policy: SideEffectPolicy,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulated: Option<bool>,
    pub irreversible: bool,
}

/// Replay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    pub execution_id: ExecutionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_id: Option<String>,
    pub mode: ReplayMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_spec: Option<MutationSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_override: Option<ArtifactOverride>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_override: Option<StateOverride>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_variables: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_overrides: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_overrides: Option<HashMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side_effect_policy: Option<SideEffectPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_network_access: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_db_writes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolation_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_target: Option<ConfidenceLevel>,
}

/// Mutation specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MutationSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swaps: Option<Vec<EventMutation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patches: Option<Vec<CodePatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_injections: Option<Vec<TimeoutInjection>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_swaps: Option<Vec<ArtifactSwap>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_rules: Option<Vec<ValidationRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_trail: Option<Vec<MutationAuditEntry>>,
}

/// Event mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMutation {
    pub event_type: EventType,
    pub replacement: serde_json::Value,
    pub mutation_type: MutationType,
    pub description: String,
    pub author: String,
    pub timestamp: i64,
}

/// Code patch for mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePatch {
    pub file: String,
    pub line: u32,
    pub original: String,
    pub replacement: String,
    pub description: String,
}

/// Timeout injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutInjection {
    pub target_event_type: EventType,
    pub delay_ms: u64,
    pub probability: f64,
}

/// Artifact swap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSwap {
    pub original_digest: String,
    pub replacement_digest: String,
    pub compatibility_check: bool,
}

/// Validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_type: String, // MUTATION_CONFLICT, UNSUPPORTED, RISK
    pub message: String,
    pub severity: String, // ERROR, WARNING
}

/// Mutation audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationAuditEntry {
    pub action: String,
    pub author: String,
    pub timestamp: i64,
    pub details: serde_json::Value,
}

/// Artifact override
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtifactOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_versions: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,
}

/// State override
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_snapshots: Option<Vec<SnapshotReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_snapshots: Option<Vec<SnapshotReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_snapshots: Option<Vec<SnapshotReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_values: Option<HashMap<String, serde_json::Value>>,
}
