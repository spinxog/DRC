use async_trait::async_trait;
use std::collections::HashMap;
use crate::types::*;
use crate::enhanced::*;

/// Core storage trait
#[async_trait]
pub trait Storage: Send + Sync {
    async fn store_event(&self, event: &DRCEvent) -> anyhow::Result<()>;
    async fn get_events(&self, execution_id: &ExecutionId) -> anyhow::Result<Vec<DRCEvent>>;
    async fn store_metadata(&self, metadata: &ExecutionMetadata) -> anyhow::Result<()>;
    async fn get_metadata(&self, execution_id: &ExecutionId) -> anyhow::Result<Option<ExecutionMetadata>>;
}

/// Capture trait for recording execution events
#[async_trait]
pub trait Capture: Send + Sync {
    async fn start_capture(&self, metadata: ExecutionMetadata) -> anyhow::Result<()>;
    async fn record_event(&self, event: DRCEvent) -> anyhow::Result<()>;
    async fn finalize_capture(&self, execution_id: &ExecutionId) -> anyhow::Result<()>;
}

/// Replay trait for replaying captured executions
#[async_trait]
pub trait Replay: Send + Sync {
    async fn start_replay(&self, config: ReplayConfig) -> anyhow::Result<ReplayHandle>;
    async fn get_next_event(&self) -> anyhow::Result<Option<DRCEvent>>;
    async fn report_divergence(&self, divergence: DivergenceReport) -> anyhow::Result<()>;
    async fn complete_replay(&self, result: ReplayResult) -> anyhow::Result<()>;
}

/// Handle for an active replay
#[derive(Debug, Clone)]
pub struct ReplayHandle {
    pub replay_id: String,
    pub execution_id: ExecutionId,
}

/// Mutation engine trait
#[async_trait]
pub trait MutationEngine: Send + Sync {
    async fn apply_mutations(
        &self,
        events: Vec<DRCEvent>,
        spec: &MutationSpec,
    ) -> anyhow::Result<Vec<DRCEvent>>;
    async fn validate_spec(&self, spec: &MutationSpec) -> anyhow::Result<Vec<String>>;
}

/// Diff engine trait
#[async_trait]
pub trait DiffEngine: Send + Sync {
    async fn compute_diff(
        &self,
        original: &[DRCEvent],
        replayed: &[DRCEvent],
    ) -> anyhow::Result<Vec<ReplayDiff>>;
    async fn find_first_divergence(
        &self,
        original: &[DRCEvent],
        replayed: &[DRCEvent],
    ) -> anyhow::Result<Option<DivergenceReport>>;
    async fn analyze_root_cause(
        &self,
        divergences: &[DivergenceReport],
    ) -> anyhow::Result<Option<RootCauseAnalysis>>;
}

/// Query layer trait
#[async_trait]
pub trait QueryLayer: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> anyhow::Result<SearchResult>;
    async fn get_execution(&self, execution_id: &ExecutionId) -> anyhow::Result<Option<ExecutionMetadata>>;
    async fn get_execution_events(&self, execution_id: &ExecutionId) -> anyhow::Result<Vec<DRCEvent>>;
}

/// Audit logger trait
#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log_action(
        &self,
        action: &str,
        author: &str,
        details: serde_json::Value,
    ) -> anyhow::Result<()>;
    async fn verify_integrity(&self) -> anyhow::Result<bool>;
}

/// Security policy enforcer
#[async_trait]
pub trait SecurityEnforcer: Send + Sync {
    async fn check_permission(
        &self,
        role: &str,
        action: Action,
        resource: &str,
    ) -> anyhow::Result<bool>;
    async fn redact_sensitive_fields(
        &self,
        event: &mut DRCEvent,
        rules: &[RedactionRule],
    ) -> anyhow::Result<()>;
}

/// Virtualized environment for replay
#[async_trait]
pub trait VirtualizedEnvironment: Send + Sync {
    async fn virtualize_time(&self, timestamp: i64) -> anyhow::Result<()>;
    async fn virtualize_random(&self, seed: u64) -> anyhow::Result<()>;
    async fn virtualize_fs(&self, snapshot: &SnapshotReference) -> anyhow::Result<()>;
    async fn virtualize_network(&self, allow: bool) -> anyhow::Result<()>;
}

/// State reconstruction trait
#[async_trait]
pub trait StateReconstruction: Send + Sync {
    async fn reconstruct_state(
        &self,
        snapshots: &[SnapshotReference],
    ) -> anyhow::Result<ReconstructedState>;
    async fn apply_state(&self, state: &ReconstructedState) -> anyhow::Result<()>;
    async fn rollback_state(&self) -> anyhow::Result<()>;
}

/// Reconstructed state container
#[derive(Debug, Clone)]
pub struct ReconstructedState {
    pub db_snapshots: Vec<SnapshotReference>,
    pub cache_snapshots: Vec<SnapshotReference>,
    pub file_snapshots: Vec<SnapshotReference>,
    pub config_values: HashMap<String, serde_json::Value>,
}

/// Proxy manager trait
#[async_trait]
pub trait ProxyManager: Send + Sync {
    async fn start_proxy(&self, config: ProxyConfig) -> anyhow::Result<String>;
    async fn stop_proxy(&self, proxy_id: &str) -> anyhow::Result<()>;
    async fn get_proxy_status(&self, proxy_id: &str) -> anyhow::Result<ProxyStatus>;
}

/// Proxy configuration
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub port: u16,
    pub host: String,
    pub target_host: String,
    pub target_port: u16,
    pub protocol: ProxyProtocol,
    pub capture_request_body: bool,
    pub capture_response_body: bool,
    pub max_body_size: usize,
    pub correlation_header: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyProtocol {
    Http,
    Https,
}

/// Proxy status
#[derive(Debug, Clone)]
pub struct ProxyStatus {
    pub id: String,
    pub running: bool,
    pub connections_active: u32,
    pub requests_total: u64,
}

/// Sandbox manager trait
#[async_trait]
pub trait SandboxManager: Send + Sync {
    async fn create_sandbox(
        &self,
        execution_id: &ExecutionId,
        config: SandboxConfig,
    ) -> anyhow::Result<SandboxHandle>;
    async fn execute_in_sandbox(
        &self,
        handle: &SandboxHandle,
        command: &[String],
    ) -> anyhow::Result<SandboxResult>;
    async fn destroy_sandbox(&self, handle: &SandboxHandle) -> anyhow::Result<()>;
}

/// Sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub sandbox_type: SandboxType,
    pub image: String,
    pub cpu_limit: String,
    pub memory_limit: String,
    pub disk_limit: String,
    pub network_mode: NetworkMode,
    pub egress_policy: EgressPolicy,
    pub volume_mounts: Vec<VolumeMount>,
    pub environment: HashMap<String, String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxType {
    Docker,
    Kubernetes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    None,
    Host,
    Bridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EgressPolicy {
    Block,
    Restricted,
    Allow,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub host: String,
    pub container: String,
    pub read_only: bool,
}

/// Sandbox handle
#[derive(Debug, Clone)]
pub struct SandboxHandle {
    pub id: String,
    pub execution_id: ExecutionId,
}

/// Sandbox execution result
#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Multi-service orchestrator trait
#[async_trait]
pub trait ServiceOrchestrator: Send + Sync {
    async fn orchestrate_replay(
        &self,
        trace_id: &TraceId,
        root_service: &str,
        options: OrchestrationOptions,
    ) -> anyhow::Result<OrchestrationResult>;
}

/// Orchestration options
#[derive(Debug, Clone)]
pub struct OrchestrationOptions {
    pub replay_all_services: bool,
    pub stub_missing_services: bool,
    pub clock_sync_mode: ClockSyncMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSyncMode {
    Root,
    PerService,
    EventDriven,
}

/// Orchestration result
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    pub success: bool,
    pub replayed_services: Vec<ExecutionId>,
    pub stubbed_services: Vec<String>,
    pub divergences: Vec<DivergenceReport>,
}
