use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Vector clock for happens-before tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorClock {
    pub clock: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            clock: HashMap::new(),
        }
    }

    /// Increment the clock for a node
    pub fn increment(&mut self, node_id: &str) {
        let entry = self.clock.entry(node_id.to_string()).or_insert(0);
        *entry += 1;
    }

    /// Merge with another vector clock (takes max of each component)
    pub fn merge(&mut self, other: &VectorClock) {
        for (node, time) in &other.clock {
            let entry = self.clock.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(*time);
        }
    }

    /// Check if this clock happens-before another
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let all_keys: HashSet<_> = self
            .clock
            .keys()
            .chain(other.clock.keys())
            .cloned()
            .collect();

        let mut all_less_or_equal = true;
        let mut at_least_one_less = false;

        for key in all_keys {
            let self_val = self.clock.get(&key).copied().unwrap_or(0);
            let other_val = other.clock.get(&key).copied().unwrap_or(0);

            if self_val > other_val {
                all_less_or_equal = false;
                break;
            }
            if self_val < other_val {
                at_least_one_less = true;
            }
        }

        all_less_or_equal && at_least_one_less
    }

    /// Check if concurrent (neither happens-before the other)
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    /// Get the time for a specific node
    pub fn get(&self, node_id: &str) -> u64 {
        self.clock.get(node_id).copied().unwrap_or(0)
    }
}

/// Lock operation for concurrency analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockOperation {
    pub operation_id: String,
    pub execution_id: String,
    pub lock_id: String,
    pub operation_type: LockOpType,
    pub timestamp: i64,
    pub vector_clock: VectorClock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LockOpType {
    Acquire,
    Release,
}

/// Race condition detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceCondition {
    pub race_id: String,
    pub data_race: bool,
    pub lock_set_1: Vec<String>,
    pub lock_set_2: Vec<String>,
    pub vector_clock_1: VectorClock,
    pub vector_clock_2: VectorClock,
    pub confidence: f64, // 0.0 - 1.0
    pub shared_variable: String,
    pub execution_id_1: String,
    pub execution_id_2: String,
}

/// Happens-before graph edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HappensBeforeEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeType {
    ProgramOrder,
    LockReleaseAcquire,
    ForkJoin,
    MessagePassing,
    ExternalSync,
}

/// Async operation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncContext {
    pub context_id: String,
    pub parent_context: Option<String>,
    pub execution_id: String,
    pub created_at: i64,
    pub vector_clock: VectorClock,
}

/// Concurrency analyzer
#[derive(Debug, Clone)]
pub struct ConcurrencyAnalyzer {
    vector_clocks: Arc<RwLock<HashMap<String, VectorClock>>>,
    lock_operations: Arc<RwLock<HashMap<String, Vec<LockOperation>>>>,
    happens_before_graph: Arc<RwLock<Vec<HappensBeforeEdge>>>,
    async_contexts: Arc<RwLock<HashMap<String, AsyncContext>>>,
    potential_races: Arc<RwLock<Vec<RaceCondition>>>,
}

impl ConcurrencyAnalyzer {
    pub fn new() -> Self {
        Self {
            vector_clocks: Arc::new(RwLock::new(HashMap::new())),
            lock_operations: Arc::new(RwLock::new(HashMap::new())),
            happens_before_graph: Arc::new(RwLock::new(Vec::new())),
            async_contexts: Arc::new(RwLock::new(HashMap::new())),
            potential_races: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize vector clock for a new execution
    pub async fn init_clock(&self, execution_id: &str) {
        let mut clocks = self.vector_clocks.write().await;
        clocks.insert(execution_id.to_string(), VectorClock::new());
    }

    /// Increment clock for an execution
    pub async fn tick(&self, execution_id: &str) {
        let mut clocks = self.vector_clocks.write().await;
        if let Some(clock) = clocks.get_mut(execution_id) {
            clock.increment(execution_id);
        }
    }

    /// Get current vector clock for execution
    pub async fn get_clock(&self, execution_id: &str) -> VectorClock {
        let clocks = self.vector_clocks.read().await;
        clocks.get(execution_id).cloned().unwrap_or_default()
    }

    /// Record a lock operation
    pub async fn record_lock_operation(
        &self,
        execution_id: &str,
        lock_id: &str,
        op_type: LockOpType,
    ) -> LockOperation {
        let op_id = format!("lock_op_{}", chrono::Utc::now().timestamp_millis());
        let clock = self.get_clock(execution_id).await;

        let op = LockOperation {
            operation_id: op_id.clone(),
            execution_id: execution_id.to_string(),
            lock_id: lock_id.to_string(),
            operation_type: op_type,
            timestamp: chrono::Utc::now().timestamp_millis(),
            vector_clock: clock,
        };

        // If this is an acquire, create happens-before edge before acquiring write lock
        if op_type == LockOpType::Acquire {
            self.create_lock_happens_before(lock_id, &op).await;
        }

        let mut ops = self.lock_operations.write().await;
        ops.entry(lock_id.to_string())
            .or_default()
            .push(op.clone());

        op
    }

    /// Create happens-before edge for lock release-acquire
    async fn create_lock_happens_before(&self, lock_id: &str, acquire_op: &LockOperation) {
        let ops = self.lock_operations.read().await;
        if let Some(lock_ops) = ops.get(lock_id) {
            // Find the last release before this acquire
            let last_release = lock_ops.iter().rev().find(|op| {
                op.operation_type == LockOpType::Release
                    && op.timestamp < acquire_op.timestamp
            });

            if let Some(release) = last_release {
                let mut graph = self.happens_before_graph.write().await;
                graph.push(HappensBeforeEdge {
                    from: release.execution_id.clone(),
                    to: acquire_op.execution_id.clone(),
                    edge_type: EdgeType::LockReleaseAcquire,
                    timestamp: chrono::Utc::now().timestamp_millis(),
                });
            }
        }
    }

    /// Create async context
    pub async fn create_async_context(
        &self,
        execution_id: &str,
        parent_context: Option<&str>,
    ) -> AsyncContext {
        let context_id = format!("async_{}", chrono::Utc::now().timestamp_millis());
        
        // Get parent clock if parent context exists
        let parent_clock = if let Some(pid) = parent_context {
            if let Some(ctx) = self.get_async_context(pid).await {
                ctx.vector_clock
            } else {
                self.get_clock(execution_id).await
            }
        } else {
            self.get_clock(execution_id).await
        };

        let context = AsyncContext {
            context_id: context_id.clone(),
            parent_context: parent_context.map(|s| s.to_string()),
            execution_id: execution_id.to_string(),
            created_at: chrono::Utc::now().timestamp_millis(),
            vector_clock: parent_clock,
        };

        let mut contexts = self.async_contexts.write().await;
        contexts.insert(context_id.clone(), context.clone());

        context
    }

    /// Get async context
    pub async fn get_async_context(&self, context_id: &str) -> Option<AsyncContext> {
        let contexts = self.async_contexts.read().await;
        contexts.get(context_id).cloned()
    }

    /// Get async context chain
    pub async fn get_async_chain(&self, context_id: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current = context_id.to_string();

        while let Some(context) = self.get_async_context(&current).await {
            chain.push(current.clone());
            match context.parent_context {
                Some(parent) => current = parent,
                None => break,
            }
        }

        chain.reverse();
        chain
    }

    /// Analyze for potential race conditions
    pub async fn analyze_races(&self) -> Vec<RaceCondition> {
        let ops = self.lock_operations.read().await;
        let _clocks = self.vector_clocks.read().await;
        let mut races = Vec::new();

        // Check for data races: concurrent access to same lock without proper synchronization
        for (lock_id, lock_ops) in ops.iter() {
            for (i, op1) in lock_ops.iter().enumerate() {
                for op2 in lock_ops.iter().skip(i + 1) {
                    // Check if concurrent
                    if op1.vector_clock.is_concurrent(&op2.vector_clock) {
                        let race = RaceCondition {
                            race_id: format!("race_{}_{}", op1.operation_id, op2.operation_id),
                            data_race: true,
                            lock_set_1: vec![lock_id.clone()],
                            lock_set_2: vec![lock_id.clone()],
                            vector_clock_1: op1.vector_clock.clone(),
                            vector_clock_2: op2.vector_clock.clone(),
                            confidence: 0.9,
                            shared_variable: lock_id.clone(),
                            execution_id_1: op1.execution_id.clone(),
                            execution_id_2: op2.execution_id.clone(),
                        };
                        races.push(race);
                    }
                }
            }
        }

        let mut stored_races = self.potential_races.write().await;
        *stored_races = races.clone();

        races
    }

    /// Check if two executions have potential data race
    pub async fn check_potential_race(&self, exec1: &str, exec2: &str) -> bool {
        let clock1 = self.get_clock(exec1).await;
        let clock2 = self.get_clock(exec2).await;
        clock1.is_concurrent(&clock2)
    }

    /// Get happens-before graph
    pub async fn get_happens_before_graph(&self) -> Vec<HappensBeforeEdge> {
        let graph = self.happens_before_graph.read().await;
        graph.clone()
    }

    /// Detect lockset violations
    pub async fn detect_lockset_violations(&self) -> Vec<String> {
        let ops = self.lock_operations.read().await;
        let mut violations = Vec::new();

        // Simplified lockset analysis
        for (lock_id, lock_ops) in ops.iter() {
            let acquires: Vec<_> = lock_ops
                .iter()
                .filter(|op| op.operation_type == LockOpType::Acquire)
                .collect();

            if acquires.len() > 1 {
                // Check if same lock acquired by different executions without happens-before
                for (i, acq1) in acquires.iter().enumerate() {
                    for acq2 in acquires.iter().skip(i + 1) {
                        if acq1.vector_clock.is_concurrent(&acq2.vector_clock) {
                            violations.push(format!(
                                "Potential lockset violation on {} between {} and {}",
                                lock_id, acq1.execution_id, acq2.execution_id
                            ));
                        }
                    }
                }
            }
        }

        violations
    }

    /// Get lock statistics
    pub async fn get_lock_statistics(&self) -> HashMap<String, LockStats> {
        let ops = self.lock_operations.read().await;
        let mut stats = HashMap::new();

        for (lock_id, lock_ops) in ops.iter() {
            let acquires = lock_ops
                .iter()
                .filter(|op| op.operation_type == LockOpType::Acquire)
                .count();
            let releases = lock_ops
                .iter()
                .filter(|op| op.operation_type == LockOpType::Release)
                .count();

            let unique_executions: HashSet<_> = lock_ops
                .iter()
                .map(|op| op.execution_id.clone())
                .collect();

            stats.insert(
                lock_id.clone(),
                LockStats {
                    total_acquires: acquires as u64,
                    total_releases: releases as u64,
                    unique_executions: unique_executions.len() as u64,
                    potential_contentions: if acquires > releases {
                        acquires - releases
                    } else {
                        0
                    } as u64,
                },
            );
        }

        stats
    }
}

impl Default for ConcurrencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Lock statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockStats {
    pub total_acquires: u64,
    pub total_releases: u64,
    pub unique_executions: u64,
    pub potential_contentions: u64,
}
