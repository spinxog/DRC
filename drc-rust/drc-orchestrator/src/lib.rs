use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Service node in the distributed execution graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNode {
    pub execution_id: String,
    pub name: String,
    pub version: String,
    pub events: Vec<drc_core::DRCEvent>,
    pub dependencies: Vec<String>,
    pub dependent_services: Vec<String>,
}

/// Service graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceGraph {
    pub trace_id: String,
    pub services: HashMap<String, ServiceNode>,
}

/// Causal slice for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalSlice {
    pub trace_id: String,
    pub root_service: String,
    pub services: Vec<ServiceNode>,
    pub complete: bool,
    pub missing_services: Vec<String>,
}

/// Clock sync mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClockSyncMode {
    Root,
    PerService,
    EventDriven,
}

/// Multi-service replay options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiServiceReplayOptions {
    pub replay_all_services: bool,
    pub stub_missing_services: bool,
    pub clock_sync_mode: ClockSyncMode,
}

/// Service graph builder
#[derive(Debug, Clone)]
pub struct ServiceGraphBuilder {
    executions: Arc<RwLock<HashMap<String, Vec<drc_core::DRCEvent>>>>,
    trace_index: Arc<RwLock<HashMap<String, Vec<String>>>>, // trace_id -> execution_ids
    service_graphs: Arc<RwLock<HashMap<String, ServiceGraph>>>,
}

impl ServiceGraphBuilder {
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            trace_index: Arc::new(RwLock::new(HashMap::new())),
            service_graphs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add execution events
    pub fn add_execution(&self, execution_id: String, events: Vec<drc_core::DRCEvent>) {
        let mut execs = self.executions.write();
        execs.insert(execution_id, events);
    }

    /// Index execution by trace ID
    pub fn index_by_trace(&self, trace_id: String, execution_id: String) {
        let mut index = self.trace_index.write();
        index
            .entry(trace_id)
            .or_default()
            .push(execution_id);
    }

    /// Build service graph from trace
    pub fn build_service_graph(&self, trace_id: &str) -> ServiceGraph {
        let index = self.trace_index.read();
        let execs = self.executions.read();

        let execution_ids = index.get(trace_id).cloned().unwrap_or_default();
        let mut services = HashMap::new();

        for exec_id in execution_ids {
            if let Some(events) = execs.get(&exec_id) {
                // Extract service name from events
                let service_name = events
                    .first()
                    .and_then(|e| e.metadata.as_ref())
                    .and_then(|m| m.service_name.clone())
                    .unwrap_or_else(|| format!("service_{}", exec_id[..8].to_string()));

                let version = events
                    .first()
                    .and_then(|e| e.metadata.as_ref())
                    .and_then(|m| m.version.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                // Extract dependencies from network events
                let mut dependencies = HashSet::new();
                for event in events {
                    if let drc_core::EventType::NetworkCall = event.event_type {
                        if let Some(service) = event.data.get("target_service").and_then(|v| v.as_str()) {
                            dependencies.insert(service.to_string());
                        }
                    }
                }

                let node = ServiceNode {
                    execution_id: exec_id.clone(),
                    name: service_name.clone(),
                    version,
                    events: events.clone(),
                    dependencies: dependencies.into_iter().collect(),
                    dependent_services: Vec::new(),
                };

                services.insert(exec_id, node);
            }
        }

        // Build dependent_services (reverse of dependencies)
        let service_names: HashMap<_, _> = services
            .iter()
            .map(|(id, node)| (node.name.clone(), id.clone()))
            .collect();

        // Collect all dependencies first to avoid borrow checker issues
        let updates: Vec<(String, String)> = services
            .values()
            .flat_map(|node| {
                node.dependencies.iter().filter_map(|dep_name| {
                    service_names.get(dep_name).map(|dep_id| (dep_id.clone(), node.name.clone()))
                })
            })
            .collect();

        // Apply updates
        for (dep_id, dependent_name) in updates {
            if let Some(dep_node) = services.get_mut(&dep_id) {
                dep_node.dependent_services.push(dependent_name);
            }
        }

        ServiceGraph {
            trace_id: trace_id.to_string(),
            services,
        }
    }

    /// Build causal slice from root service
    pub fn build_causal_slice(&self, trace_id: &str, root_service: &str) -> CausalSlice {
        let graph = self.build_service_graph(trace_id);
        let root_node = graph
            .services
            .values()
            .find(|n| n.name == root_service || n.execution_id == root_service);

        let mut included = HashSet::new();
        let mut queue = vec![];

        if let Some(root) = root_node {
            included.insert(root.execution_id.clone());
            queue.push(root.execution_id.clone());
        }

        // BFS to find all dependent services
        while let Some(current_id) = queue.pop() {
            if let Some(node) = graph.services.get(&current_id) {
                for dep_name in &node.dependencies {
                    if let Some((dep_id, _)) = graph
                        .services
                        .iter()
                        .find(|(_, n)| n.name == *dep_name)
                    {
                        if included.insert(dep_id.clone()) {
                            queue.push(dep_id.clone());
                        }
                    }
                }
            }
        }

        let services: Vec<_> = graph
            .services
            .values()
            .filter(|n| included.contains(&n.execution_id))
            .cloned()
            .collect();

        let missing: Vec<_> = graph
            .services
            .values()
            .filter(|n| {
                services.iter().any(|s| {
                    s.dependencies.contains(&n.name) && !included.contains(&n.execution_id)
                })
            })
            .map(|n| n.name.clone())
            .collect();

        CausalSlice {
            trace_id: trace_id.to_string(),
            root_service: root_service.to_string(),
            services,
            complete: missing.is_empty(),
            missing_services: missing,
        }
    }

    /// Get service graph
    pub fn get_service_graph(&self, trace_id: &str) -> ServiceGraph {
        let graphs = self.service_graphs.read();
        graphs
            .get(trace_id)
            .cloned()
            .unwrap_or_else(|| self.build_service_graph(trace_id))
    }

    /// Find missing services
    pub fn find_missing_services(&self, trace_id: &str) -> Vec<String> {
        let graph = self.build_service_graph(trace_id);
        let all_deps: HashSet<_> = graph
            .services
            .values()
            .flat_map(|n| n.dependencies.iter().cloned())
            .collect();

        let existing: HashSet<_> = graph
            .services
            .values()
            .map(|n| n.name.clone())
            .collect();

        all_deps
            .difference(&existing)
            .cloned()
            .collect()
    }
}

impl Default for ServiceGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-service orchestrator
#[derive(Debug, Clone)]
pub struct MultiServiceOrchestrator {
    service_graph: ServiceGraphBuilder,
    active_replays: Arc<RwLock<HashMap<String, ActiveReplay>>>,
}

#[derive(Debug, Clone)]
struct ActiveReplay {
    state: ReplayState,
    _started_at: i64,
    completed_at: Option<i64>,
    _stub_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayState {
    Pending,
    Running,
    Completed,
    Failed,
}

impl MultiServiceOrchestrator {
    pub fn new(service_graph: ServiceGraphBuilder) -> Self {
        Self {
            service_graph,
            active_replays: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Orchestrate multi-service replay
    pub async fn orchestrate_replay(
        &self,
        trace_id: &str,
        root_service: &str,
        options: MultiServiceReplayOptions,
    ) -> anyhow::Result<OrchestrationResult> {
        let slice = self.service_graph.build_causal_slice(trace_id, root_service);

        if !slice.complete && !options.stub_missing_services {
            return Err(anyhow::anyhow!(
                "Incomplete service graph. Missing: {}",
                slice.missing_services.join(", ")
            ));
        }

        let mut replayed_services = Vec::new();
        let mut stubbed_services = Vec::new();
        let divergences = Vec::new();

        // Compute replay order (topological sort)
        let replay_order = self.compute_replay_order(&slice.services);

        // Cache service graph for efficient lookup
        let service_graph = self.service_graph.get_service_graph(trace_id);

        for service in replay_order {
            // Check if all dependencies have been replayed
            let deps_ready = service.dependencies.iter().all(|dep| {
                replayed_services.iter().any(|id| {
                    service_graph
                        .services
                        .get(id)
                        .map(|n| n.name == *dep)
                        .unwrap_or(false)
                }) || stubbed_services.contains(dep)
            });

            if !deps_ready {
                warn!("Dependencies not ready for {}, stubbing", service.name);
                stubbed_services.push(service.name.clone());
                continue;
            }

            // Determine if we should replay or stub
            if !options.replay_all_services && service.name != root_service {
                stubbed_services.push(service.name.clone());
                continue;
            }

            // Start replay with Pending state
            self.active_replays.write().insert(
                service.execution_id.clone(),
                ActiveReplay {
                    state: ReplayState::Pending,
                    _started_at: chrono::Utc::now().timestamp_millis(),
                    completed_at: None,
                    _stub_mode: false,
                },
            );

            // Transition to Running
            if let Some(replay) = self.active_replays.write().get_mut(&service.execution_id) {
                replay.state = ReplayState::Running;
            }

            replayed_services.push(service.execution_id.clone());

            // Wait for replay to complete (simplified)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Mark as completed or failed
            if let Some(replay) = self.active_replays.write().get_mut(&service.execution_id) {
                if replayed_services.len() % 10 == 0 && !replayed_services.is_empty() {
                    // Simulate occasional failure for demo purposes
                    replay.state = ReplayState::Failed;
                } else {
                    replay.state = ReplayState::Completed;
                }
                replay.completed_at = Some(chrono::Utc::now().timestamp_millis());
            }
        }

        Ok(OrchestrationResult {
            success: true,
            replayed_services,
            stubbed_services,
            divergences,
        })
    }

    /// Compute replay order using Kahn's algorithm
    fn compute_replay_order(&self, services: &[ServiceNode]) -> Vec<ServiceNode> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();

        // Build graph
        for svc in services {
            in_degree.entry(svc.execution_id.clone()).or_insert(0);
            adj.entry(svc.execution_id.clone()).or_default();
        }

        for svc in services {
            for dep_name in &svc.dependencies {
                if let Some(dep) = services.iter().find(|s| s.name == *dep_name) {
                    adj.entry(dep.execution_id.clone())
                        .or_default()
                        .push(svc.execution_id.clone());
                    *in_degree.entry(svc.execution_id.clone()).or_insert(0) += 1;
                }
            }
        }

        // Find nodes with 0 in-degree
        let mut queue: Vec<_> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut order = Vec::new();
        let mut processed = HashSet::new();

        while let Some(id) = queue.pop() {
            if let Some(node) = services.iter().find(|s| s.execution_id == id) {
                order.push(node.clone());
                processed.insert(id.clone());

                for neighbor in adj.get(&id).unwrap_or(&Vec::new()) {
                    let new_degree = in_degree.get(neighbor).unwrap_or(&1) - 1;
                    in_degree.insert(neighbor.clone(), new_degree);

                    if new_degree == 0 && !processed.contains(neighbor) {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }

        if order.len() != services.len() {
            warn!("Circular dependency detected in service graph");
        }

        order
    }
}

impl Default for MultiServiceOrchestrator {
    fn default() -> Self {
        Self::new(ServiceGraphBuilder::new())
    }
}

/// Orchestration result
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    pub success: bool,
    pub replayed_services: Vec<String>,
    pub stubbed_services: Vec<String>,
    pub divergences: Vec<drc_core::DivergenceReport>,
}

/// Distributed trace span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedSpan {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub operation_name: String,
    pub start_time: i64,
    pub duration: i64,
    pub status: String,
    pub attributes: HashMap<String, String>,
}

/// Distributed trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTrace {
    pub trace_id: String,
    pub spans: Vec<DistributedSpan>,
}

/// Tracing integration
pub struct TracingIntegration;

impl TracingIntegration {
    pub fn new() -> Self {
        Self
    }

    /// Parse OpenTelemetry trace
    pub fn parse_opentelemetry_trace(&self, trace_data: &serde_json::Value) -> DistributedTrace {
        let trace_id = trace_data
            .get("resourceSpans")
            .and_then(|s| s.get(0))
            .and_then(|s| s.get("scopeSpans"))
            .and_then(|s| s.get(0))
            .and_then(|s| s.get("spans"))
            .and_then(|s| s.get(0))
            .and_then(|s| s.get("traceId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let spans = trace_data
            .get("resourceSpans")
            .and_then(|rs| rs.as_array())
            .map(|resources| {
                resources
                    .iter()
                    .flat_map(|resource| {
                        resource
                            .get("scopeSpans")
                            .and_then(|ss| ss.as_array())
                            .map(|scopes| {
                                scopes
                                    .iter()
                                    .flat_map(|scope| {
                                        scope
                                            .get("spans")
                                            .and_then(|s| s.as_array())
                                            .map(|spans| {
                                                spans
                                                    .iter()
                                                    .map(|span| DistributedSpan {
                                                        span_id: span
                                                            .get("spanId")
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("")
                                                            .to_string(),
                                                        parent_span_id: span
                                                            .get("parentSpanId")
                                                            .and_then(|v| v.as_str())
                                                            .map(|s| s.to_string()),
                                                        service_name: resource
                                                            .get("resource")
                                                            .and_then(|r| r.get("attributes"))
                                                            .and_then(|a| a.as_array())
                                                            .and_then(|attrs| {
                                                                attrs.iter().find(|attr| {
                                                                    attr.get("key")
                                                                        .and_then(|k| k.as_str())
                                                                        == Some("service.name")
                                                                })
                                                            })
                                                            .and_then(|attr| {
                                                                attr.get("value")
                                                                    .and_then(|v| v.get("stringValue"))
                                                                    .and_then(|v| v.as_str())
                                                            })
                                                            .unwrap_or("unknown")
                                                            .to_string(),
                                                        operation_name: span
                                                            .get("name")
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("")
                                                            .to_string(),
                                                        start_time: span
                                                            .get("startTimeUnixNano")
                                                            .and_then(|v| v.as_i64())
                                                            .map(|n| (n / 1_000_000) as i64)
                                                            .unwrap_or(0),
                                                        duration: span
                                                            .get("endTimeUnixNano")
                                                            .and_then(|v| v.as_i64())
                                                            .map(|n| {
                                                                let start = span
                                                                    .get("startTimeUnixNano")
                                                                    .and_then(|v| v.as_i64())
                                                                    .unwrap_or(0);
                                                                ((n - start) / 1_000_000) as i64
                                                            })
                                                            .unwrap_or(0),
                                                        status: span
                                                            .get("status")
                                                            .and_then(|s| s.get("code"))
                                                            .and_then(|c| c.as_i64())
                                                            .map(|c| {
                                                                if c == 2 {
                                                                    "error".to_string()
                                                                } else {
                                                                    "ok".to_string()
                                                                }
                                                            })
                                                            .unwrap_or_else(|| "ok".to_string()),
                                                        attributes: HashMap::new(),
                                                    })
                                                    .collect::<Vec<_>>()
                                            })
                                            .unwrap_or_default()
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();

        DistributedTrace { trace_id, spans }
    }

    /// Parse Jaeger trace
    pub fn parse_jaeger_trace(&self, trace_data: &serde_json::Value) -> DistributedTrace {
        let trace_id = trace_data
            .get("data")
            .and_then(|d| d.get(0))
            .and_then(|s| s.get("traceID"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let spans = trace_data
            .get("data")
            .and_then(|d| d.as_array())
            .map(|spans| {
                spans
                    .iter()
                    .map(|span| DistributedSpan {
                        span_id: span
                            .get("spanID")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        parent_span_id: span
                            .get("references")
                            .and_then(|r| r.as_array())
                            .and_then(|refs| {
                                refs.iter().find(|r: &&serde_json::Value| {
                                    r.get("refType")
                                        .and_then(|t| t.as_str())
                                        == Some("CHILD_OF")
                                })
                            })
                            .and_then(|r: &serde_json::Value| r.get("spanID"))
                            .and_then(|v: &serde_json::Value| v.as_str())
                            .map(|s: &str| s.to_string()),
                        service_name: span
                            .get("process")
                            .and_then(|p| p.get("serviceName"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        operation_name: span
                            .get("operationName")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        start_time: span
                            .get("startTime")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                        duration: span
                            .get("duration")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                        status: span
                            .get("tags")
                            .and_then(|t| t.as_array())
                            .and_then(|tags| {
                                tags.iter().find(|tag| {
                                    tag.get("key").and_then(|k| k.as_str()) == Some("error")
                                })
                            })
                            .map(|_| "error".to_string())
                            .unwrap_or_else(|| "ok".to_string()),
                        attributes: HashMap::new(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        DistributedTrace { trace_id, spans }
    }
}

impl Default for TracingIntegration {
    fn default() -> Self {
        Self::new()
    }
}
