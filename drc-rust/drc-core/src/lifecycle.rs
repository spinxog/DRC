use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use crate::types::*;

/// Manages the lifecycle of executions
#[derive(Debug, Clone)]
pub struct ExecutionLifecycleManager {
    executions: Arc<RwLock<HashMap<ExecutionId, ExecutionState>>>,
    metadata: Arc<RwLock<HashMap<ExecutionId, ExecutionMetadata>>>,
    active_captures: Arc<RwLock<HashMap<ExecutionId, ActiveCapture>>>,
}

#[derive(Debug, Clone)]
struct ActiveCapture {
    _start_time: i64,
    event_count: u64,
    checkpoints: Vec<u64>,
}

impl ExecutionLifecycleManager {
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            active_captures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new execution
    pub async fn register_execution(&self, metadata: ExecutionMetadata) -> anyhow::Result<()> {
        let execution_id = metadata.execution_id.clone();
        
        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), ExecutionState::Started);
        
        let mut meta = self.metadata.write().await;
        meta.insert(execution_id.clone(), metadata);
        
        info!("Registered execution: {}", execution_id);
        Ok(())
    }

    /// Start capturing for an execution
    pub async fn start_capture(&self, execution_id: &ExecutionId) -> anyhow::Result<()> {
        let mut executions = self.executions.write().await;
        
        if let Some(state) = executions.get_mut(execution_id) {
            *state = ExecutionState::Capturing;
            
            let mut captures = self.active_captures.write().await;
            captures.insert(
                execution_id.clone(),
                ActiveCapture {
                    _start_time: chrono::Utc::now().timestamp_millis(),
                    event_count: 0,
                    checkpoints: Vec::new(),
                },
            );
            
            info!("Started capture for execution: {}", execution_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", execution_id))
        }
    }

    /// Record an event checkpoint
    pub async fn checkpoint(&self, execution_id: &ExecutionId, sequence: u64) -> anyhow::Result<()> {
        let mut captures = self.active_captures.write().await;
        
        if let Some(capture) = captures.get_mut(execution_id) {
            capture.checkpoints.push(sequence);
            capture.event_count += 1;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active capture for execution: {}", execution_id))
        }
    }

    /// Get rollback plan to a checkpoint
    pub async fn get_rollback_plan(
        &self,
        execution_id: &ExecutionId,
        target_sequence: u64,
    ) -> anyhow::Result<RollbackPlan> {
        let captures = self.active_captures.read().await;
        
        if let Some(capture) = captures.get(execution_id) {
            let steps = capture
                .checkpoints
                .iter()
                .filter(|&&seq| seq > target_sequence)
                .map(|&seq| RollbackStep {
                    sequence: seq,
                    action: RollbackAction::RevertEvent,
                })
                .collect();
            
            Ok(RollbackPlan {
                execution_id: execution_id.clone(),
                target_sequence,
                steps,
            })
        } else {
            Err(anyhow::anyhow!("No active capture for execution: {}", execution_id))
        }
    }

    /// Finalize an execution capture
    pub async fn finalize_capture(&self, execution_id: &ExecutionId) -> anyhow::Result<()> {
        let mut executions = self.executions.write().await;
        
        if let Some(state) = executions.get_mut(execution_id) {
            *state = ExecutionState::Finalized;
            
            let mut captures = self.active_captures.write().await;
            if let Some(capture) = captures.remove(execution_id) {
                info!(
                    "Finalized capture for execution: {} with {} events",
                    execution_id, capture.event_count
                );
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", execution_id))
        }
    }

    /// Mark execution as replayable
    pub async fn mark_replayable(&self, execution_id: &ExecutionId) -> anyhow::Result<()> {
        let mut executions = self.executions.write().await;
        
        if let Some(state) = executions.get_mut(execution_id) {
            *state = ExecutionState::Replayable;
            info!("Marked execution as replayable: {}", execution_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", execution_id))
        }
    }

    /// Start a replay
    pub async fn start_replay(
        &self,
        execution_id: &ExecutionId,
        replay_id: String,
    ) -> anyhow::Result<()> {
        let executions = self.executions.read().await;
        
        if let Some(state) = executions.get(execution_id) {
            if *state != ExecutionState::Replayable && *state != ExecutionState::Replayed {
                return Err(anyhow::anyhow!(
                    "Execution {} is not replayable (current state: {:?})",
                    execution_id, state
                ));
            }
            
            info!("Started replay {} for execution: {}", replay_id, execution_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", execution_id))
        }
    }

    /// Mark replay as complete
    pub async fn complete_replay(
        &self,
        execution_id: &ExecutionId,
        success: bool,
    ) -> anyhow::Result<()> {
        let mut executions = self.executions.write().await;
        
        if let Some(state) = executions.get_mut(execution_id) {
            *state = if success {
                ExecutionState::Replayed
            } else {
                ExecutionState::Failed
            };
            
            let mut metadata = self.metadata.write().await;
            if let Some(meta) = metadata.get_mut(execution_id) {
                meta.replay_count += 1;
                meta.last_replayed_at = Some(chrono::Utc::now().timestamp_millis());
            }
            
            info!(
                "Completed replay for execution: {} (success: {})",
                execution_id, success
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", execution_id))
        }
    }

    /// Get execution state
    pub async fn get_state(&self, execution_id: &ExecutionId) -> Option<ExecutionState> {
        let executions = self.executions.read().await;
        executions.get(execution_id).copied()
    }

    /// Get execution metadata
    pub async fn get_metadata(&self, execution_id: &ExecutionId) -> Option<ExecutionMetadata> {
        let metadata = self.metadata.read().await;
        metadata.get(execution_id).cloned()
    }

    /// List all executions
    pub async fn list_executions(&self) -> Vec<(ExecutionId, ExecutionState)> {
        let executions = self.executions.read().await;
        executions
            .iter()
            .map(|(id, state)| (id.clone(), *state))
            .collect()
    }

    /// Get executions by state
    pub async fn get_by_state(&self, state: ExecutionState) -> Vec<ExecutionId> {
        let executions = self.executions.read().await;
        executions
            .iter()
            .filter(|(_, s)| **s == state)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Check if execution is in a terminal state
    pub async fn is_terminal(&self, execution_id: &ExecutionId) -> bool {
        if let Some(state) = self.get_state(execution_id).await {
            matches!(
                state,
                ExecutionState::Finalized
                    | ExecutionState::Replayable
                    | ExecutionState::Replayed
                    | ExecutionState::Failed
                    | ExecutionState::Expired
                    | ExecutionState::Corrupted
            )
        } else {
            false
        }
    }
}

impl Default for ExecutionLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Rollback plan for state reconstruction
#[derive(Debug, Clone)]
pub struct RollbackPlan {
    pub execution_id: ExecutionId,
    pub target_sequence: u64,
    pub steps: Vec<RollbackStep>,
}

#[derive(Debug, Clone)]
pub struct RollbackStep {
    pub sequence: u64,
    pub action: RollbackAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackAction {
    RevertEvent,
    CompensateSideEffect,
    RestoreSnapshot,
}
