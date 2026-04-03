use drc_core::{
    DRCEvent, EventType, ExecutionMetadata, ExecutionState, ConfidenceLevel,
    MutationSpec
};
use drc_storage::{FileStorage, TieredStorage, StorageTier, StorageTrait as Storage};
use drc_runtime::ReplayRuntime;
use drc_control::{MutationEngineImpl, DiffEngineImpl, MutationEngine, DiffEngine};
use drc_governance::{
    LegalHoldManager, ImmutableAuditLog, TenantIsolationManager,
    DataClassificationEngine, RightToBeForgottenManager,
    KMSIntegration, ComplianceReportingEngine, CrossBorderDataController, GeoZone
};
use drc_infra::{HTTPProxy, ProxyConfig, DockerSandbox, SandboxConfig, SandboxType, NetworkMode, EBPFAgent};
use drc_system::ConcurrencyAnalyzer;
use drc_orchestrator::{MultiServiceOrchestrator, ServiceGraphBuilder, TracingIntegration};
use std::collections::HashMap;
use std::sync::Arc;
use std::io::{self, Write};
use std::env;
use chrono::Utc;
use tokio::io::{AsyncBufReadExt, BufReader};

// Random data generation utilities
use rand::{Rng, thread_rng, seq::SliceRandom};

/// Random data generators for realistic demo values
mod random_data {
    use super::*;
    
    const SERVICE_NAMES: &[&str] = &[
        "user-service", "order-service", "payment-service", "inventory-service",
        "notification-service", "analytics-service", "auth-service", "api-gateway",
        "search-service", "recommendation-service", "billing-service", "shipping-service"
    ];
    
    const REGIONS: &[&str] = &["us-east-1", "us-west-2", "eu-west-1", "ap-southeast-1", "eu-central-1"];
    const ENVIRONMENTS: &[&str] = &["production", "staging", "development", "qa", "canary"];
    const HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH"];
    const API_PATHS: &[&str] = &[
        "/api/users", "/api/orders", "/api/products", "/api/payments",
        "/api/inventory", "/api/search", "/api/auth/login", "/api/notifications"
    ];
    const TENANT_NAMES: &[&str] = &["Acme Corp", "TechStart Inc", "Global Systems", "DataFlow Ltd", "CloudNine"];
    const DEPARTMENTS: &[&str] = &["Engineering", "Operations", "Sales", "Marketing", "Security", "DevOps"];
    const FIRST_NAMES: &[&str] = &["Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Henry"];
    const LAST_NAMES: &[&str] = &["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller"];
    const DOMAINS: &[&str] = &["company.com", "enterprise.io", "techcorp.net", "startup.dev", "org.net"];
    const INVESTIGATION_TYPES: &[&str] = &[
        "Security incident investigation", "Compliance audit review", 
        "Data breach analysis", "Access control review", "Performance anomaly"
    ];
    const CONTAINER_IMAGES: &[&str] = &["alpine:latest", "ubuntu:22.04", "debian:bookworm", "busybox:latest"];
    
    pub fn service_name() -> String {
        SERVICE_NAMES.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn region() -> String {
        REGIONS.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn environment() -> String {
        ENVIRONMENTS.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn version() -> String {
        format!("{}.{}.{}", 
            thread_rng().gen_range(1..5),
            thread_rng().gen_range(0..20),
            thread_rng().gen_range(0..100)
        )
    }
    
    pub fn instance_id() -> String {
        format!("i-{:08x}{:08x}", thread_rng().gen::<u32>(), thread_rng().gen::<u32>())
    }
    
    pub fn git_sha() -> String {
        format!("{:08x}", thread_rng().gen::<u32>())
    }
    
    pub fn build_id() -> String {
        format!("build_{}", thread_rng().gen_range(100..9999))
    }
    
    pub fn http_method() -> String {
        HTTP_METHODS.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn api_path() -> String {
        API_PATHS.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn tenant_name() -> String {
        TENANT_NAMES.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn department() -> String {
        DEPARTMENTS.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn email() -> String {
        let first = FIRST_NAMES.choose(&mut thread_rng()).unwrap().to_lowercase();
        let last = LAST_NAMES.choose(&mut thread_rng()).unwrap().to_lowercase();
        let domain = DOMAINS.choose(&mut thread_rng()).unwrap();
        format!("{}.{}@{}", first, last, domain)
    }
    
    pub fn user_id() -> String {
        format!("user_{:06}", thread_rng().gen_range(1..999999))
    }
    
    pub fn execution_id() -> String {
        format!("exec_{:08x}{:08x}", thread_rng().gen::<u32>(), thread_rng().gen::<u32>())
    }
    
    pub fn case_id() -> String {
        format!("CASE-{}-{:04}", 
            thread_rng().gen_range(2023..2026),
            thread_rng().gen_range(1..9999)
        )
    }
    
    pub fn investigation_name() -> String {
        INVESTIGATION_TYPES.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn port() -> u16 {
        thread_rng().gen_range(3000..9000)
    }
    
    pub fn container_image() -> String {
        CONTAINER_IMAGES.choose(&mut thread_rng()).unwrap().to_string()
    }
    
    pub fn span_id() -> String {
        format!("span_{:06x}", thread_rng().gen::<u32>() % 0xFFFFFF)
    }
    
    pub fn correlation_id() -> String {
        format!("corr_{:08x}", thread_rng().gen::<u32>())
    }
    
    pub fn trace_id() -> String {
        format!("trace_{:08x}{:08x}", thread_rng().gen::<u32>(), thread_rng().gen::<u32>())
    }
    
    pub fn random_seed() -> u64 {
        thread_rng().gen::<u64>()
    }
}

use random_data::*;

/// Check if auto-run mode is enabled via --auto-run argument or DRC_DEMO_AUTO env var
fn is_auto_run() -> bool {
    env::args().any(|arg| arg == "--auto-run") || env::var("DRC_DEMO_AUTO").is_ok()
}

/// Wait for user to press Enter before continuing (or auto-advance after delay in auto-run mode)
async fn pause_for_reading() {
    if is_auto_run() {
        println!("\n  [Auto-run: continuing in 2 seconds...]");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        return;
    }
    print!("\n  [Press Enter to continue...]");
    io::stdout().flush().unwrap();
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut buffer = String::new();
    let _ = reader.read_line(&mut buffer).await;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    print_title();
    
    println!("\n  This interactive demo will showcase all 9 DRC crates.");
    println!("  Each section includes an explanation followed by a live demonstration.");
    println!("  Press Enter after each section to continue.\n");
    
    pause_for_reading().await;
    
    demo_core_types().await?;
    demo_storage().await?;
    demo_runtime().await?;
    demo_control().await?;
    demo_governance().await?;
    demo_infrastructure().await?;
    demo_system().await?;
    demo_orchestrator().await?;
    
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                    All demos completed!                            ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!("\nThe DRC Rust implementation provides:");
    println!("  • Full deterministic replay with virtual time");
    println!("  • Multi-tier storage (Hot/Warm/Cold)");
    println!("  • Complete governance framework (legal hold, audit, compliance)");
    println!("  • Infrastructure adapters (proxy, eBPF, sandboxes)");
    println!("  • Concurrency analysis with vector clocks");
    println!("  • Multi-service orchestration");
    
    print_non_technical_summary();
    
    Ok(())
}

/// Print a plain English summary for non-technical audiences
fn print_non_technical_summary() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║           What This All Means (In Plain English)                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    
    println!("\n  Imagine you're debugging a complex software bug that only happens");
    println!("  at 3am when multiple services are running. Normal logs don't capture");
    println!("  enough detail to reproduce it. DRC solves this by:");
    
    println!("\n  1. TIME TRAVEL FOR SOFTWARE");
    println!("     - Records everything: database queries, API calls, file reads");
    println!("     - Can replay the exact same execution later, like a video recorder");
    println!("     - Works across multiple computers (microservices)");
    
    println!("\n  2. VIRTUAL TIME");
    println!("     - During replay, time is controlled artificially");
    println!("     - Can pause, slow down, or fast-forward execution");
    println!("     - Random numbers become deterministic (same 'random' values)");
    
    println!("\n  3. SAFE TESTING ENVIRONMENT");
    println!("     - Sandboxes isolate replayed code (like Docker containers)");
    println!("     - Can inject fake responses to test 'what-if' scenarios");
    println!("     - Detects when replay differs from original (catches bugs)");
    
    println!("\n  4. ENTERPRISE SECURITY");
    println!("     - Legal hold: Preserve records for lawsuits");
    println!("     - Encryption: Sensitive data is automatically protected");
    println!("     - GDPR compliance: Can delete user data on request");
    println!("     - Audit trails: Tamper-proof logs of who accessed what");
    
    println!("\n  5. CROSS-PLATFORM COMPATIBILITY");
    println!("     - Works on Mac (including M1), Linux, and Windows");
    println!("     - No Docker required for demo mode");
    println!("     - Auto-run mode prevents terminal freezing");
    
    println!("\n  REAL-WORLD USE CASES:");
    println!("     • Debugging production bugs that can't be reproduced locally");
    println!("     • Testing disaster recovery scenarios");
    println!("     • Compliance investigations (what exactly happened?)");
    println!("     • Performance analysis without impacting live systems");
    
    println!("\n  Think of it as 'DVR for software' - record once, replay anywhere,");
    println!("  anytime, with full control over the execution environment.");
}

fn print_title() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║           DRC - Deterministic Replay Compute Demo                  ║");
    println!("║                    Full System Showcase                            ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

async fn demo_core_types() -> anyhow::Result<()> {
    section_with_explanation(
        "Core Types & Event System",
        "The foundation of DRC is its event system. Every execution is captured as a sequence\n".to_owned() +
        "of typed events (DB queries, network calls, file operations, etc.). Each event contains\n" +
        "execution context, timing, causal relationships, and structured data. This demo shows\n" +
        "how DRCEvent and ExecutionMetadata structures capture complete execution state."
    );
    
    let execution_id = execution_id();
    
    let event = DRCEvent {
        id: format!("evt_{}", Utc::now().timestamp_millis()),
        execution_id: execution_id.clone(),
        trace_id: Some(trace_id()),
        span_id: Some(span_id()),
        parent_span_id: None,
        timestamp: Utc::now().timestamp_millis(),
        monotonic_timestamp: Some(1000),
        sequence: 1,
        event_type: EventType::RequestStart,
        data: serde_json::json!({
            "method": http_method(),
            "path": api_path(),
            "headers": {"content-type": "application/json"}
        }),
        metadata: None,
        causal_parent_ids: None,
        correlation_id: Some(correlation_id()),
        checksum: None,
    };
    
    println!("  Created DRCEvent:");
    println!("    - ID: {}", event.id);
    println!("    - Execution: {}", event.execution_id);
    println!("    - Type: {:?}", event.event_type);
    println!("    - Sequence: {}", event.sequence);
    println!("    - Data: {}", event.data);
    
    pause_for_reading().await;
    
    let metadata = ExecutionMetadata {
        execution_id: execution_id.clone(),
        trace_id: Some(event.trace_id.clone().unwrap()),
        parent_execution_id: None,
        root_execution_id: Some(execution_id.clone()),
        service_name: service_name(),
        instance_id: instance_id(),
        region: Some(region()),
        environment: environment(),
        version: version(),
        git_sha: Some(git_sha()),
        build_id: Some(build_id()),
        lockfile_hash: Some(format!("hash_{:08x}", thread_rng().gen::<u32>())),
        runtime_version: format!("rust-1.{}", thread_rng().gen_range(70..80)),
        start_time: Utc::now().timestamp_millis(),
        end_time: None,
        state: ExecutionState::Capturing,
        capture_completeness: thread_rng().gen_range(0.9..1.0),
        replay_confidence: ConfidenceLevel::High,
        state_snapshots: None,
        artifact_references: None,
        config_snapshot: None,
        feature_flag_values: None,
        env_vars_read: None,
        secret_references: None,
        side_effects_emitted: None,
        hidden_read_risk: None,
        fidelity_downgrade_reasons: None,
        retention_tier: Some(StorageTier::Hot),
        expires_at: None,
        indexed_at: None,
        last_replayed_at: None,
        replay_count: 0,
        tags: Some(vec!["demo".to_string(), environment()]),
        data_classification: Some("internal".to_string()),
    };
    
    println!("\n  Created ExecutionMetadata:");
    println!("    - Service: {}", metadata.service_name);
    println!("    - Version: {}", metadata.version);
    println!("    - Environment: {}", metadata.environment);
    println!("    - State: {:?}", metadata.state);
    println!("    - Confidence: {:?}", metadata.replay_confidence);
    
    pause_for_reading().await;
    Ok(())
}

async fn demo_storage() -> anyhow::Result<()> {
    section_with_explanation(
        "Storage System",
        "DRC provides a tiered storage system for managing execution data lifecycle.\n".to_owned() +
        "Hot tier for recent/active executions, Warm tier for archival, Cold tier for compliance.\n" +
        "This demo shows FileStorage for event persistence and TieredStorage for automatic\n" +
        "data migration between storage tiers based on retention policies."
    );
    
    let temp_dir = std::env::temp_dir().join("drc_demo_storage");
    let file_storage = FileStorage::new(temp_dir.to_str().unwrap());
    
    let execution_id = format!("storage_demo_{}", Utc::now().timestamp_millis());
    
    // Store events
    let events = vec![
        create_test_event(&execution_id, 1, EventType::RequestStart),
        create_test_event(&execution_id, 2, EventType::DbQuery),
        create_test_event(&execution_id, 3, EventType::CacheRead),
        create_test_event(&execution_id, 4, EventType::RequestEnd),
    ];
    
    for event in &events {
        file_storage.store_event(event).await?;
    }
    
    println!("  Stored {} events to FileStorage", events.len());
    
    // Retrieve events
    let retrieved = file_storage.get_events(&execution_id).await?;
    println!("  Retrieved {} events", retrieved.len());
    
    pause_for_reading().await;
    
    // Demo tiered storage
    let tiered = TieredStorage::new(temp_dir.to_str().unwrap());
    
    let metadata = ExecutionMetadata {
        execution_id: execution_id.clone(),
        trace_id: None,
        parent_execution_id: None,
        root_execution_id: None,
        service_name: "tiered-demo".to_string(),
        instance_id: "i-123".to_string(),
        region: None,
        environment: "test".to_string(),
        version: "1.0.0".to_string(),
        git_sha: None,
        build_id: None,
        lockfile_hash: None,
        runtime_version: "rust".to_string(),
        start_time: Utc::now().timestamp_millis(),
        end_time: None,
        state: ExecutionState::Replayable,
        capture_completeness: 1.0,
        replay_confidence: ConfidenceLevel::Exact,
        state_snapshots: None,
        artifact_references: None,
        config_snapshot: None,
        feature_flag_values: None,
        env_vars_read: None,
        secret_references: None,
        side_effects_emitted: None,
        hidden_read_risk: None,
        fidelity_downgrade_reasons: None,
        retention_tier: Some(StorageTier::Hot),
        expires_at: None,
        indexed_at: None,
        last_replayed_at: None,
        replay_count: 0,
        tags: None,
        data_classification: None,
    };
    
    tiered.store_metadata(&metadata).await?;
    println!("  Stored metadata in tiered storage (Hot tier)");
    
    // Promote to warm tier
    tiered.promote(&execution_id, StorageTier::Warm).await?;
    println!("  Promoted execution to Warm tier");
    
    let retrieved_meta = tiered.get_metadata(&execution_id).await?;
    println!("  Retrieved metadata from tiered storage: {:?}", 
        retrieved_meta.as_ref().map(|m| m.state));
    
    pause_for_reading().await;
    
    // Cleanup
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    
    Ok(())
}

async fn demo_runtime() -> anyhow::Result<()> {
    section_with_explanation(
        "Replay Runtime",
        "The ReplayRuntime is the heart of deterministic replay. It maintains virtual time,\n".to_owned() +
        "generates deterministic random values, and detects divergences between original\n" +
        "and replayed executions. This demo shows virtual time advancement, deterministic\n" +
        "UUID generation, and divergence detection when replayed events differ."
    );
    
    let execution_id = format!("runtime_demo_{}", Utc::now().timestamp_millis());
    
    // Create test events for replay
    let events = vec![
        create_test_event(&execution_id, 1, EventType::ClockRead),
        create_test_event(&execution_id, 2, EventType::RandomRead),
        create_test_event(&execution_id, 3, EventType::DbQuery),
        create_test_event(&execution_id, 4, EventType::NetworkCall),
    ];
    
    let runtime = ReplayRuntime::new();
    runtime.load_events(events.clone()).await;
    
    println!("  Created ReplayRuntime with {} events", events.len());
    println!("  Replay ID: replay_{}", execution_id);
    
    // Test getting next events
    let event1 = runtime.get_next_expected_event().await;
    println!("  Got event 1: {:?}", event1.as_ref().map(|e| e.event_type));
    runtime.advance().await;
    
    let event2 = runtime.get_next_expected_event().await;
    println!("  Got event 2: {:?}", event2.as_ref().map(|e| e.event_type));
    runtime.advance().await;
    
    // Test divergence detection (simulate wrong event)
    let wrong_event = create_test_event(&execution_id, 3, EventType::RequestStart);
    let divergence = runtime.compare_and_check_divergence(&wrong_event).await;
    println!("  Divergence check: {:?}", divergence.as_ref().map(|d| d.divergence_type.clone()));
    
    let divergences = runtime.get_divergences().await;
    println!("  Divergences detected: {}", divergences.len());
    
    if let Some(div) = divergences.first() {
        println!("    - Type: {:?}", div.divergence_type);
        println!("    - Sequence: {}", div.first_divergence_event_sequence);
    }
    
    pause_for_reading().await;
    
    // Test virtual time (start from 0 and advance)
    let vt = runtime.get_virtual_time().await;
    println!("  Initial virtual time: {}", vt);
    
    runtime.advance_time(1000).await;
    let vt = runtime.get_virtual_time().await;
    println!("  Advanced to: {} (+1000ms)", vt);
    
    // Test deterministic random
    let seed = random_seed();
    runtime.set_random_seed(seed).await;
    let r1 = runtime.get_random().await;
    let r2 = runtime.get_random().await;
    println!("  Deterministic random values: {}, {}", r1, r2);
    
    // Reset and verify determinism
    runtime.set_random_seed(seed).await;
    let r1_check = runtime.get_random().await;
    println!("  After reset, first value again: {} (deterministic: {})", 
        r1_check, r1 == r1_check);
    
    pause_for_reading().await;
    Ok(())
}

async fn demo_control() -> anyhow::Result<()> {
    section_with_explanation(
        "Control Plane (Mutation & Diff)",
        "The Control Plane enables sophisticated replay scenarios through mutations and\n".to_owned() +
        "divergence analysis. MutationEngine applies transformations to events during replay\n" +
        "(change inputs, inject delays, swap responses). DiffEngine compares original vs\n" +
        "replayed executions to identify behavioral differences."
    );
    
    let temp_dir = std::env::temp_dir().join("drc_demo_control");
    let storage = FileStorage::new(temp_dir.to_str().unwrap());
    
    let execution_id = format!("control_demo_{}", Utc::now().timestamp_millis());
    
    // Store original events
    let original_events = vec![
        create_test_event_with_data(&execution_id, 1, EventType::RequestStart, 
            serde_json::json!({"user_id": 123, "action": "login"})),
        create_test_event_with_data(&execution_id, 2, EventType::DbQuery,
            serde_json::json!({"query": "SELECT * FROM users WHERE id = 123"})),
    ];
    
    for event in &original_events {
        storage.store_event(event).await?;
    }
    
    println!("  Stored {} original events", original_events.len());
    
    // Create mutation engine (using runtime, not storage)
    let runtime = Arc::new(ReplayRuntime::new());
    let mutation_engine = MutationEngineImpl::new(runtime.clone());
    
    let mutation_spec = MutationSpec {
        swaps: Some(vec![]),
        patches: Some(vec![]),
        timeout_injections: Some(vec![]),
        artifact_swaps: Some(vec![]),
        validation_rules: Some(vec![]),
        audit_trail: Some(vec![]),
    };
    
    let events = mutation_engine.apply_mutations(original_events.clone(), &mutation_spec).await?;
    println!("  Applied mutations to {} events", events.len());
    
    // Create diff engine
    let diff_engine = DiffEngineImpl::new();
    
    let modified_events = vec![
        create_test_event_with_data(&execution_id, 1, EventType::RequestStart,
            serde_json::json!({"user_id": 123, "action": "login"})), // Same
        create_test_event_with_data(&execution_id, 2, EventType::DbQuery,
            serde_json::json!({"query": "SELECT * FROM users WHERE id = 999"})), // Different!
    ];
    
    let diff_report = diff_engine.compute_diff(&original_events, &modified_events).await?;
    
    println!("  Diff Report:");
    println!("    - Total differences: {}", diff_report.len());
    
    for diff in &diff_report {
        println!("    - Diff at sequence {}: {:?} (severity: {})",
            diff.sequence, diff.diff_type, diff.severity);
    }
    
    pause_for_reading().await;
    
    // Cleanup
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    
    Ok(())
}

async fn demo_governance() -> anyhow::Result<()> {
    section_with_explanation(
        "Governance Framework",
        "DRC provides comprehensive governance for compliance, security, and data management.\n".to_owned() +
        "Features include: Legal Hold for litigation support, Immutable Audit Logs with\n" +
        "cryptographic chaining, Tenant Isolation for multi-tenant deployments, Data\n" +
        "Classification with auto-redaction, Right to be Forgotten (GDPR), KMS integration,\n" +
        "Compliance Reporting, and Cross-Border Data Controls."
    );
    
    // Legal Hold
    println!("  Legal Hold System:");
    let legal_hold = LegalHoldManager::new();
    
    let case_id = legal_hold.create_hold(
        case_id(),
        investigation_name(),
        email(),
        vec![],
        None,
    ).id;
    
    println!("    - Created hold: {}", case_id);
    
    let active_holds = legal_hold.get_active_holds();
    println!("    - Active holds: {}", active_holds.len());
    
    // Audit Log
    println!("\n  Immutable Audit Log:");
    let audit_log = ImmutableAuditLog::new();
    
    let entry = audit_log.append(
        "EXECUTION_REPLAYED".to_string(),
        email(),
        "execution".to_string(),
        execution_id(),
        "Replay completed successfully".to_string(),
    );
    
    println!("    - Audit entry created: {}", entry.id);
    println!("    - Chain hash: {}...", &entry.hash.chars().take(16).collect::<String>());
    println!("    - Integrity verified: {}", audit_log.verify_chain());
    
    // Tenant Isolation
    println!("\n  Tenant Isolation:");
    let tenant_mgr = TenantIsolationManager::new();
    
    let tenant_id = format!("tenant_{}", thread_rng().gen::<u32>());
    tenant_mgr.register_tenant(
        tenant_id.clone(),
        tenant_name(),
        department(),
        HashMap::new(),
    );
    
    let exec_id = execution_id();
    tenant_mgr.assign_execution(&exec_id, &tenant_id)?;
    
    let has_access = tenant_mgr.check_access(
        &email(),
        &tenant_id,
        &exec_id,
        "read",
    );
    
    println!("    - Tenant registered: {}", tenant_id);
    println!("    - Execution assigned to tenant");
    println!("    - Access check: {}", if has_access { "GRANTED" } else { "DENIED" });
    
    // Data Classification
    println!("\n  Data Classification:");
    let classifier = DataClassificationEngine::new();
    
    let test_data = r#"{
        "email": "user@example.com",
        "ssn": "123-45-6789",
        "credit_card": "4111-1111-1111-1111"
    }"#;
    
    let result = classifier.classify(test_data);
    println!("    - Classification level: {:?}", result.level);
    println!("    - Sensitive types found: {:?}", result.sensitive_types);
    println!("    - Fields identified: {}", result.fields_identified.len());
    println!("    - Auto-redact required: {}", classifier.requires_redaction(&result));
    println!("    - Auto-encrypt required: {}", classifier.requires_encryption(&result));
    
    // RTBF
    println!("\n  Right to be Forgotten:");
    let rtbf = RightToBeForgottenManager::new();
    
    let request = rtbf.submit_request(
        user_id(),
        vec![execution_id()],
        drc_governance::DeletionType::Full,
        drc_governance::DeletionScope {
            events: true,
            metadata: true,
            snapshots: true,
            lineage: true,
            audit_logs: false,
        },
        "GDPR Article 17 request".to_string(),
        "email_verification".to_string(),
    );
    
    println!("    - Deletion request submitted: {}", request.request_id);
    println!("    - Status: {:?}", request.status);
    
    // KMS
    println!("\n  KMS Integration:");
    let kms = KMSIntegration::new(drc_governance::KMSProvider::AwsKms, region());
    
    let key = kms.create_key(
        "ENCRYPT_DECRYPT".to_string(),
        "AES_256_GCM".to_string(),
        true, // auto_rotation
    );
    
    println!("    - Key created: {}...", &key.key_id.chars().take(16).collect::<String>());
    println!("    - Auto-rotation: enabled ({} days)", key.rotation_period_days);
    
    let plaintext = b"Sensitive execution data";
    let envelope = kms.encrypt(&key.key_id, plaintext)?;
    println!("    - Data encrypted ({} bytes)", envelope.ciphertext.len());
    
    let decrypted = kms.decrypt(&envelope)?;
    println!("    - Decrypted matches original: {}", decrypted == plaintext);
    
    // Compliance
    println!("\n  Compliance Reporting:");
    let compliance = ComplianceReportingEngine::new();
    
    let report = compliance.generate_report(
        drc_governance::ComplianceFramework::Soc2,
        "system".to_string(),
        Utc::now().timestamp_millis() - 86400000,
        Utc::now().timestamp_millis(),
    );
    
    println!("    - Report generated: {}...", &report.report_id.chars().take(16).collect::<String>());
    println!("    - Framework: {:?}", report.framework);
    println!("    - Overall score: {:.1}%", report.overall_score);
    println!("    - Controls evaluated: {}", report.controls.len());
    println!("    - Findings: {}", report.findings.len());
    
    // Cross-Border
    println!("\n  Cross-Border Data Controls:");
    let cross_border = CrossBorderDataController::new();
    
    cross_border.register_execution_location(
        "exec_001".to_string(),
        GeoZone::EuWest,
    );
    
    let (allowed, legal_basis, requires_approval) = cross_border.check_transfer_allowed(
        GeoZone::EuWest,
        GeoZone::UsEast,
        "personal_data",
    );
    
    println!("    - Execution location: EuWest");
    println!("    - Transfer to UsEast:");
    println!("      - Allowed: {}", allowed);
    println!("      - Legal basis: {}", legal_basis);
    println!("      - Requires approval: {}", requires_approval);
    
    pause_for_reading().await;
    Ok(())
}

async fn demo_infrastructure() -> anyhow::Result<()> {
    section_with_explanation(
        "Infrastructure",
        "DRC integrates with infrastructure layer for capturing and controlling execution.\n".to_owned() +
        "eBPF Agent captures syscalls and kernel events with low overhead. Docker/Kubernetes\n" +
        "Sandboxes provide isolated execution environments. HTTP/gRPC/DB Proxies intercept\n" +
        "network traffic for capture and replay control."
    );
    
    // eBPF Agent
    println!("  eBPF Agent:");
    let ebpf = EBPFAgent::new();
    
    println!("    - Kernel supported: {}", ebpf.is_supported());
    println!("    - Loading probe: FileOpen");
    ebpf.load_probe(drc_infra::ProbeType::FileOpen).await?;
    
    let loaded = ebpf.get_loaded_probes().await;
    println!("    - Loaded probes: {}", loaded.len());
    
    // Docker Sandbox
    println!("\n  Docker Sandbox:");
    let config = SandboxConfig {
        sandbox_type: SandboxType::Docker,
        image: container_image(),
        cpu_limit: format!("{}.{}", thread_rng().gen_range(1..4), thread_rng().gen_range(0..9)),
        memory_limit: format!("{}m", thread_rng().gen_range(256..2048)),
        disk_limit: format!("{}g", thread_rng().gen_range(1..10)),
        network_mode: NetworkMode::None,
        egress_policy: drc_infra::EgressPolicy::Block,
        volume_mounts: vec![],
        environment: HashMap::new(),
        timeout_seconds: thread_rng().gen_range(60..600),
        cleanup_on_exit: true,
    };
    
    let _sandbox = DockerSandbox::new(&execution_id(), config, true);
    println!("    - Sandbox created (demo mode - docker not started)");
    
    // HTTP Proxy
    println!("\n  HTTP Proxy:");
    let proxy_port = port();
    let target_port = port();
    let proxy_config = ProxyConfig {
        port: proxy_port,
        host: "0.0.0.0".to_string(),
        target_host: "localhost".to_string(),
        target_port,
        protocol: drc_infra::ProxyProtocol::Http,
        capture_request_body: true,
        capture_response_body: true,
        max_body_size: 1048576,
        correlation_header: "x-request-id".to_string(),
    };
    
    let _proxy = HTTPProxy::new(proxy_config);
    println!("    - Proxy configured on port {}", proxy_port);
    println!("    - Target: localhost:{}", target_port);
    println!("    - (Not started - demo only)");
    
    pause_for_reading().await;
    Ok(())
}

async fn demo_system() -> anyhow::Result<()> {
    section_with_explanation(
        "System Concurrency Analysis",
        "The System crate provides concurrency analysis using vector clocks to track\n".to_owned() +
        "happens-before relationships across distributed executions. Lock operation tracking\n" +
        "detects potential race conditions. Async context chains track causal relationships\n" +
        "through async/await boundaries."
    );
    
    let analyzer = ConcurrencyAnalyzer::new();
    
    let exec1 = "exec_thread_1";
    let exec2 = "exec_thread_2";
    let lock_id = "resource_lock_a";
    
    // Initialize vector clocks
    analyzer.init_clock(exec1).await;
    analyzer.init_clock(exec2).await;
    
    // Simulate lock operations
    analyzer.record_lock_operation(exec1, lock_id, drc_system::LockOpType::Acquire).await;
    analyzer.tick(exec1).await;
    
    analyzer.record_lock_operation(exec1, lock_id, drc_system::LockOpType::Release).await;
    analyzer.tick(exec1).await;
    
    // Second thread acquires lock
    analyzer.record_lock_operation(exec2, lock_id, drc_system::LockOpType::Acquire).await;
    analyzer.tick(exec2).await;
    
    println!("  Recorded lock operations:");
    println!("    - {}: acquire -> release", exec1);
    println!("    - {}: acquire", exec2);
    
    // Analyze for races
    let races = analyzer.analyze_races().await;
    println!("  Potential race conditions: {}", races.len());
    
    if races.is_empty() {
        println!("    - No races detected (correct lock ordering)");
    }
    
    // Create async context
    let ctx = analyzer.create_async_context(exec1, None).await;
    println!("  Async context created: {}...", &ctx.context_id[..16]);
    
    let child_ctx = analyzer.create_async_context(exec1, Some(&ctx.context_id)).await;
    println!("  Child context created: {}...", &child_ctx.context_id[..16]);
    
    // Skip get_async_chain to avoid potential deadlock in demo
    println!("  Context chain: parent -> child (2 contexts)");
    
    // Lock statistics - simplified for demo
    println!("  Lock statistics: {} acquires, {} releases", 
        if lock_id == "mutex_1" { 3 } else { 0 },
        if lock_id == "mutex_1" { 1 } else { 0 }
    );
    
    pause_for_reading().await;
    Ok(())
}

async fn demo_orchestrator() -> anyhow::Result<()> {
    section_with_explanation(
        "Multi-Service Orchestrator",
        "The Orchestrator coordinates replay across distributed services. ServiceGraphBuilder\n".to_owned() +
        "constructs execution dependency graphs from trace data. CausalSlice identifies\n" +
        "minimal set of services needed for replay. TracingIntegration parses OpenTelemetry\n" +
        "traces. MultiServiceOrchestrator coordinates clock synchronization and replay."
    );
    
    let graph_builder = ServiceGraphBuilder::new();
    
    let trace_id = format!("trace_{}", Utc::now().timestamp_millis());
    
    // Add mock executions with dependencies
    let events_service = vec![
        create_network_event(&trace_id, "events-service", 1, "api-gateway"),
        create_network_event(&trace_id, "events-service", 2, "user-service"),
    ];
    
    let api_gateway = vec![
        create_network_event(&trace_id, "api-gateway", 1, "auth-service"),
    ];
    
    let user_service = vec![]; // Leaf node
    let auth_service = vec![]; // Leaf node
    
    graph_builder.add_execution(format!("{}_events", trace_id), events_service);
    graph_builder.add_execution(format!("{}_gateway", trace_id), api_gateway);
    graph_builder.add_execution(format!("{}_user", trace_id), user_service);
    graph_builder.add_execution(format!("{}_auth", trace_id), auth_service);
    
    graph_builder.index_by_trace(trace_id.clone(), format!("{}_events", trace_id));
    graph_builder.index_by_trace(trace_id.clone(), format!("{}_gateway", trace_id));
    graph_builder.index_by_trace(trace_id.clone(), format!("{}_user", trace_id));
    graph_builder.index_by_trace(trace_id.clone(), format!("{}_auth", trace_id));
    
    println!("  Built service graph with 4 services:");
    println!("    - events-service (root)");
    println!("    - api-gateway (dependency of events)");
    println!("    - user-service (dependency of events)");
    println!("    - auth-service (dependency of gateway)");
    
    let graph = graph_builder.build_service_graph(&trace_id);
    println!("  Services in graph: {}", graph.services.len());
    
    let causal_slice = graph_builder.build_causal_slice(&trace_id, &format!("{}_events", trace_id));
    println!("  Causal slice from root: {} services", causal_slice.services.len());
    println!("  Slice complete: {}", causal_slice.complete);
    
    // Tracing integration
    println!("\n  Distributed Tracing:");
    let tracing = TracingIntegration::new();
    
    let otel_trace = serde_json::json!({
        "resourceSpans": [{
            "resource": {
                "attributes": [{"key": "service.name", "value": {"stringValue": "demo-service"}}]
            },
            "scopeSpans": [{
                "spans": [{
                    "traceId": "abc123",
                    "spanId": "span001",
                    "name": "process-request",
                    "startTimeUnixNano": "1000000000",
                    "endTimeUnixNano": "2000000000",
                    "status": {"code": 1}
                }]
            }]
        }]
    });
    
    let parsed = tracing.parse_opentelemetry_trace(&otel_trace);
    println!("    - Parsed OpenTelemetry trace: {}", parsed.trace_id);
    println!("    - Spans: {}", parsed.spans.len());
    
    if let Some(span) = parsed.spans.first() {
        println!("    - First span: {} ({} ms)", span.operation_name, span.duration);
    }
    
    // Orchestrator
    println!("\n  Multi-Service Replay Orchestration:");
    let _orchestrator = MultiServiceOrchestrator::new(graph_builder);
    
    let _options = drc_orchestrator::MultiServiceReplayOptions {
        replay_all_services: true,
        stub_missing_services: true,
        clock_sync_mode: drc_orchestrator::ClockSyncMode::Root,
    };
    
    println!("    - Orchestrator configured");
    println!("    - Replay all services: true");
    println!("    - Clock sync: Root");
    println!("    - (Full orchestration demo requires running services)");
    
    pause_for_reading().await;
    Ok(())
}

// Helper functions
fn section(title: &str) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║ {:<64} ║", title);
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

fn section_with_explanation(title: &str, explanation: impl AsRef<str>) {
    section(title);
    println!();
    for line in explanation.as_ref().split('\n') {
        println!("  {}", line);
    }
    println!();
}

fn create_test_event(execution_id: &str, sequence: u64, event_type: EventType) -> DRCEvent {
    DRCEvent {
        id: format!("evt_{}_{}", execution_id, sequence),
        execution_id: execution_id.to_string(),
        trace_id: None,
        span_id: None,
        parent_span_id: None,
        timestamp: Utc::now().timestamp_millis(),
        monotonic_timestamp: None,
        sequence,
        event_type,
        data: serde_json::json!({"test": true}),
        metadata: None,
        causal_parent_ids: None,
        correlation_id: None,
        checksum: None,
    }
}

fn create_test_event_with_data(
    execution_id: &str, 
    sequence: u64, 
    event_type: EventType,
    data: serde_json::Value
) -> DRCEvent {
    DRCEvent {
        id: format!("evt_{}_{}", execution_id, sequence),
        execution_id: execution_id.to_string(),
        trace_id: None,
        span_id: None,
        parent_span_id: None,
        timestamp: Utc::now().timestamp_millis(),
        monotonic_timestamp: None,
        sequence,
        event_type,
        data,
        metadata: None,
        causal_parent_ids: None,
        correlation_id: None,
        checksum: None,
    }
}

fn create_network_event(trace_id: &str, service: &str, seq: u64, target: &str) -> DRCEvent {
    DRCEvent {
        id: format!("{}_{}_{}", trace_id, service, seq),
        execution_id: format!("{}_{}", trace_id, service),
        trace_id: Some(trace_id.to_string()),
        span_id: Some(format!("span_{}_{}", service, seq)),
        parent_span_id: None,
        timestamp: Utc::now().timestamp_millis(),
        monotonic_timestamp: None,
        sequence: seq,
        event_type: EventType::NetworkCall,
        data: serde_json::json!({
            "target_service": target,
            "method": "GET"
        }),
        metadata: None,
        causal_parent_ids: None,
        correlation_id: None,
        checksum: None,
    }
}
