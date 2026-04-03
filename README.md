# Deterministic Replay Compute (DRC) System - Rust Implementation

A production-ready system for deterministic capture and replay of software executions, fully implemented in Rust.

## Overview

The DRC system captures non-deterministic inputs during execution (time, randomness, I/O, network, etc.) and enables deterministic replay of those executions in an isolated environment. This enables:

- **Debugging production issues** with exact reproduction
- **Regression testing** by replaying historical executions against new code
- **Mutation testing** to understand how code changes affect outcomes
- **Root cause analysis** with causal chain tracking
- **Compliance & Governance** with legal hold, audit logs, and data classification

## Architecture

### Rust Workspace Structure

```
drc-rust/
├── drc-core/          # Core types, events, traits, lifecycle
├── drc-storage/       # File & tiered storage implementations
├── drc-runtime/       # Replay runtime with virtual time
├── drc-control/       # Mutation & diff engines
├── drc-governance/    # Legal hold, audit, compliance, KMS, RTBF
├── drc-infra/         # Proxy, eBPF, sandboxes, DB adapters
├── drc-system/        # Concurrency analysis, vector clocks
├── drc-orchestrator/  # Multi-service orchestration
└── drc-cli/           # Command-line interface
```

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                      DRC Rust System                        │
├─────────────────────────────────────────────────────────────┤
│  Core Layer (drc-core)                                      │
│  ├── Types: DRCEvent, ExecutionMetadata, ReplayConfig       │
│  ├── Traits: Storage, Capture, Replay, Mutation, Diff       │
│  └── Lifecycle: Execution state management                  │
├─────────────────────────────────────────────────────────────┤
│  Storage Layer (drc-storage)                                │
│  ├── FileStorage: JSONL-based event storage                 │
│  └── TieredStorage: Hot/Warm/Cold tier management           │
├─────────────────────────────────────────────────────────────┤
│  Runtime Layer (drc-runtime)                                │
│  ├── ReplayRuntime: Virtual time, deterministic random      │
│  └── VirtualSyscallHandler: Network, FS, async interception │
├─────────────────────────────────────────────────────────────┤
│  Control Layer (drc-control)                                │
│  ├── MutationEngine: Payload swaps, timeout injection       │
│  └── DiffEngine: Execution comparison, divergence detection │
├─────────────────────────────────────────────────────────────┤
│  Governance Layer (drc-governance)                          │
│  ├── LegalHoldManager: Litigation hold management           │
│  ├── ImmutableAuditLog: Cryptographic audit chain           │
│  ├── TenantIsolation: Multi-tenant access control           │
│  ├── DataClassification: PII/PCI/PHI detection              │
│  ├── KMSIntegration: Encryption key management              │
│  ├── RightToBeForgotten: GDPR deletion requests             │
│  ├── ComplianceReporting: SOC2/HIPAA/GDPR/PCI-DSS           │
│  └── CrossBorderData: Data residency controls               │
├─────────────────────────────────────────────────────────────┤
│  Infrastructure (drc-infra)                                 │
│  ├── HTTP/gRPC/PostgreSQL/Redis Proxies                     │
│  ├── eBPFAgent: Kernel syscall capture                      │
│  ├── DockerSandbox: Containerized replay environments       │
│  └── DB Adapters: PostgreSQL, MySQL, Redis, MongoDB         │
├─────────────────────────────────────────────────────────────┤
│  System Layer (drc-system)                                  │
│  └── ConcurrencyAnalyzer: Vector clocks, race detection     │
├─────────────────────────────────────────────────────────────┤
│  Orchestration (drc-orchestrator)                           │
│  ├── MultiServiceOrchestrator: Distributed replay           │
│  ├── ServiceGraphBuilder: Causal dependency graphs          │
│  └── TracingIntegration: OpenTelemetry/Jaeger parsing       │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+ with Cargo
- (Optional) Docker for sandboxed replay
- (Optional) eBPF support for kernel capture

### Installation

```bash
git clone https://github.com/your-org/drc.git
cd drc/drc-rust
cargo build --release
```

### Running the Demo

```bash
# Full system demo (interactive mode)
cargo run --bin demo

# Auto-run mode (no user input required, 2-second delays)
cargo run --bin demo -- --auto-run
# OR
DRC_DEMO_AUTO=1 cargo run --bin demo

# Cross-platform support:
# - Mac (Intel & M1): Native async I/O, no Docker required
# - Linux: Full async runtime support
# - Windows: Demo mode without Docker dependencies
```

See `demo.rs` in the root folder for a comprehensive example showcasing all capabilities.

### Basic Capture & Replay

```rust
use drc_core::{DRCEvent, EventType, ExecutionMetadata};
use drc_storage::FileStorage;
use drc_runtime::ReplayRuntime;
use drc_control::DRCControlPlane;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize storage
    let storage = FileStorage::new("./drc_data").await?;
    
    // 2. Capture execution
    let execution_id = "exec_123".to_string();
    let events = capture_execution().await;
    
    for event in &events {
        storage.store_event(event).await?;
    }
    
    // 3. Replay
    let control_plane = DRCControlPlane::new(storage);
    let config = ReplayConfig {
        execution_id: execution_id.clone(),
        mode: ReplayMode::Strict,
        ..Default::default()
    };
    
    let result = control_plane.run_replay(
        &config,
        |runtime| async move {
            // Your replay code here
            process_request().await
        }
    ).await?;
    
    println!("Replay success: {}", result.success);
    println!("Divergences: {}", result.divergences.len());
    
    Ok(())
}
```

## Features

### Capture Coverage

- **Time**: `std::time::Instant`, `chrono::DateTime`
- **Randomness**: `rand` crate, `getrandom`
- **File System**: `tokio::fs`, `std::fs`
- **Network**: `hyper`, `reqwest`, `tonic` (gRPC)
- **Async**: `tokio::time`, `async-trait`
- **UUID**: `uuid` crate

### Replay Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `Strict` | Exact replay, fail on any divergence | Debugging, testing |
| `Adaptive` | Allow minor timing differences | Performance analysis |
| `Mutated` | Replay with explicit mutations | What-if analysis |
| `Approximate` | Best-effort replay | Legacy compatibility |

### Storage Tiers

| Tier | Retention | Access Pattern | Compression |
|------|-----------|----------------|-------------|
| Hot | 1 day | Real-time | None |
| Warm | 7 days | Query | LZ4 |
| Cold | 90 days | Archive | Zstd |

### Governance Features

- **Legal Hold**: Litigation hold management with WORM storage
- **Audit Logging**: Cryptographically chained immutable logs
- **Tenant Isolation**: Multi-tenant RBAC with cross-tenant grants
- **Data Classification**: Automatic PII/PCI/PHI detection
- **KMS Integration**: AWS KMS, Azure Key Vault, GCP KMS, HashiCorp Vault
- **Right to be Forgotten**: GDPR Article 17 compliant deletion
- **Compliance Reporting**: SOC2, HIPAA, GDPR, PCI-DSS frameworks
- **Cross-Border Controls**: Data residency and transfer rules

## What This Means (In Plain English)

Imagine you're debugging a complex software bug that only happens at 3am when multiple services are running. Normal logs don't capture enough detail to reproduce it. DRC solves this by:

### 1. Time Travel for Software
- Records everything: database queries, API calls, file reads
- Can replay the exact same execution later, like a video recorder
- Works across multiple computers (microservices)

### 2. Virtual Time
- During replay, time is controlled artificially
- Can pause, slow down, or fast-forward execution
- Random numbers become deterministic (same 'random' values)

### 3. Safe Testing Environment
- Sandboxes isolate replayed code (like Docker containers)
- Can inject fake responses to test "what-if" scenarios
- Detects when replay differs from original (catches bugs)

### 4. Enterprise Security
- Legal hold: Preserve records for lawsuits
- Encryption: Sensitive data is automatically protected
- GDPR compliance: Can delete user data on request
- Audit trails: Tamper-proof logs of who accessed what

### Real-World Use Cases
- **Debugging production bugs** that can't be reproduced locally
- **Testing disaster recovery** scenarios
- **Compliance investigations** (what exactly happened?)
- **Performance analysis** without impacting live systems

Think of it as **"DVR for software"** - record once, replay anywhere, anytime, with full control over the execution environment.

## CLI Usage

```bash
# Capture a new execution
drc capture --service my-service --output ./captures --tags prod,api

# Replay an execution
drc replay --execution-id exec_123 --mode strict

# Search executions
drc search --service my-service --time-range 24h

# Compare executions
drc diff exec_123 exec_456

# Governance commands
drc governance legal-hold create --case "Investigation-001" --targets exec_001,exec_002
drc governance audit verify
drc governance compliance report --framework soc2 --output report.json

# Start proxy
drc proxy --port 8080 --target-host localhost --target-port 3000

# Start API server
drc server start --port 3000
```

## API Reference

### Core Types

```rust
// Event structure
pub struct DRCEvent {
    pub id: EventId,
    pub execution_id: ExecutionId,
    pub trace_id: Option<TraceId>,
    pub timestamp: i64,
    pub sequence: u64,
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub metadata: Option<EventMetadata>,
}

// Execution metadata
pub struct ExecutionMetadata {
    pub execution_id: ExecutionId,
    pub service_name: String,
    pub version: String,
    pub environment: String,
    pub state: ExecutionState,
    pub capture_completeness: f64,
    pub replay_confidence: ConfidenceLevel,
}

// Replay configuration
pub struct ReplayConfig {
    pub execution_id: ExecutionId,
    pub mode: ReplayMode,
    pub mutation_spec: Option<MutationSpec>,
    pub timeout_ms: Option<u64>,
    pub side_effect_policy: SideEffectPolicy,
}
```

### Storage Trait

```rust
#[async_trait]
pub trait Storage: Send + Sync {
    async fn store_event(&self, event: &DRCEvent) -> Result<()>;
    async fn get_events(&self, execution_id: &ExecutionId) -> Result<Vec<DRCEvent>>;
    async fn store_metadata(&self, metadata: &ExecutionMetadata) -> Result<()>;
    async fn get_metadata(&self, execution_id: &ExecutionId) -> Result<Option<ExecutionMetadata>>;
}
```

## Configuration

### Cargo.toml

```toml
[dependencies]
drc-core = { path = "drc-rust/drc-core" }
drc-storage = { path = "drc-rust/drc-storage" }
drc-runtime = { path = "drc-rust/drc-runtime" }
drc-control = { path = "drc-rust/drc-control" }
drc-governance = { path = "drc-rust/drc-governance" }
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
```

## Testing

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p drc-core
cargo test -p drc-runtime

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html
```

## Deployment

### Docker

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/drc /usr/local/bin/
EXPOSE 3000
CMD ["drc", "server", "start", "--port", "3000"]
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: drc-system
spec:
  replicas: 3
  selector:
    matchLabels:
      app: drc-system
  template:
    metadata:
      labels:
        app: drc-system
    spec:
      containers:
      - name: drc
        image: drc-system:latest
        command: ["drc", "server", "start"]
        ports:
        - containerPort: 3000
        env:
        - name: RUST_LOG
          value: info
        volumeMounts:
        - name: data
          mountPath: /data
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: drc-data
```

## Metrics & Observability

### Key Metrics

- `drc_capture_events_total`: Total events captured
- `drc_replay_success_rate`: Replay success percentage
- `drc_storage_compression_ratio`: Storage efficiency
- `drc_fidelity_score`: Overall replay fidelity

### Prometheus Integration

```rust
use drc_core::metrics::MetricsCollector;

let metrics = MetricsCollector::new();
metrics.record_capture_rate(1000.0);
metrics.record_replay_success(true);
```

## Performance

| Metric | Target | Notes |
|--------|--------|-------|
| Capture throughput | 10k+ events/sec | Per instance |
| Replay latency | <10ms overhead | Per event |
| Storage compression | 10:1 ratio | Zstd on cold tier |
| Query latency | <100ms | Hot tier, indexed |

## Security

- **Encryption**: AES-256-GCM for data at rest
- **TLS**: mTLS for inter-service communication
- **Audit**: Immutable, cryptographically signed logs
- **RBAC**: Role-based access control with tenant isolation
- **PII**: Automatic detection and redaction

## Best Practices

1. **Enable strict capture mode** for critical paths
2. **Capture initial state snapshots** for DB/cache
3. **Use appropriate replay mode** for your use case
4. **Validate mutations** before applying
5. **Monitor fidelity scores** to ensure replay quality
6. **Set retention policies** to manage storage costs
7. **Enable redaction** for sensitive data
8. **Use tiered storage** for cost optimization
9. **Enable audit logging** for compliance
10. **Implement legal hold** for litigation readiness

## Troubleshooting

### Common Issues

**Build fails with linking errors**
- Ensure you have OpenSSL development libraries: `libssl-dev` (Ubuntu), `openssl-devel` (RHEL)

**eBPF capture not working**
- Check kernel version (5.15+ required): `uname -r`
- Verify CAP_BPF capability: `sudo setcap cap_bpf+ep ./drc`

**Replay divergence detected**
- Check state reconstruction completeness
- Verify no hidden reads in original execution
- Ensure artifact compatibility

**High storage usage**
- Enable compression: `storage.enable_compression = true`
- Adjust retention policies
- Use tiered storage effectively

**Slow queries**
- Ensure proper indexing
- Use time-range filters
- Consider warm/cold tier access patterns

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Built with [Tokio](https://tokio.rs/) async runtime
- Serialization via [serde](https://serde.rs/)
- gRPC support via [tonic](https://github.com/hyperium/tonic)
- eBPF capture via [aya](https://github.com/aya-rs/aya)
