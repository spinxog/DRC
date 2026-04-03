use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use sha2::{Sha256, Digest};
use hex::encode;

/// Data lineage node representing an entity in the lineage graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: String,
    pub node_type: NodeType,
    pub name: String,
    pub created_at: i64,
    pub metadata: HashMap<String, String>,
    pub parent_ids: Vec<String>,
    pub hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeType {
    Capture,
    Replay,
    Mutation,
    Export,
    Deletion,
    Retention,
    Classification,
}

/// Data transformation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransformation {
    pub transformation_type: String,
    pub input_fields: Vec<String>,
    pub output_fields: Vec<String>,
    pub transformation_logic: String,
    pub timestamp: i64,
}

/// Provenance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub node_id: String,
    pub transformations: Vec<DataTransformation>,
    pub source_nodes: Vec<String>,
    pub integrity_score: f64, // 0.0 - 1.0
}

/// Data lineage tracker
#[derive(Debug, Clone)]
pub struct DataLineageTracker {
    nodes: Arc<RwLock<HashMap<String, LineageNode>>>,
    transformations: Arc<RwLock<HashMap<String, Vec<DataTransformation>>>>,
    provenance: Arc<RwLock<HashMap<String, ProvenanceRecord>>>,
}

impl DataLineageTracker {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            transformations: Arc::new(RwLock::new(HashMap::new())),
            provenance: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Calculate hash for node
    fn calculate_hash(&self, node_type: &NodeType, name: &str, metadata: &HashMap<String, String>) -> String {
        let input = format!("{:?}{}{:?}", node_type, name, metadata);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        encode(hasher.finalize())
    }

    /// Record a capture node
    pub fn record_capture(
        &self,
        execution_id: &str,
        metadata: HashMap<String, String>,
    ) -> LineageNode {
        let id = format!("capture_{}", execution_id);
        let hash = self.calculate_hash(&NodeType::Capture, execution_id, &metadata);
        
        let node = LineageNode {
            id: id.clone(),
            node_type: NodeType::Capture,
            name: execution_id.to_string(),
            created_at: Utc::now().timestamp_millis(),
            metadata,
            parent_ids: Vec::new(),
            hash: hash.clone(),
        };
        
        let mut nodes = self.nodes.write();
        nodes.insert(id.clone(), node.clone());
        
        // Create provenance record
        let provenance = ProvenanceRecord {
            node_id: id.clone(),
            transformations: Vec::new(),
            source_nodes: Vec::new(),
            integrity_score: 1.0,
        };
        
        let mut prov = self.provenance.write();
        prov.insert(id, provenance);
        
        node
    }

    /// Record a replay node
    pub fn record_replay(
        &self,
        replay_id: &str,
        execution_id: &str,
        parent_replay_id: Option<&str>,
        metadata: HashMap<String, String>,
    ) -> LineageNode {
        let id = format!("replay_{}", replay_id);
        let mut parent_ids = vec![format!("capture_{}", execution_id)];
        
        if let Some(parent) = parent_replay_id {
            parent_ids.push(format!("replay_{}", parent));
        }
        
        let hash = self.calculate_hash(&NodeType::Replay, replay_id, &metadata);
        
        let node = LineageNode {
            id: id.clone(),
            node_type: NodeType::Replay,
            name: replay_id.to_string(),
            created_at: Utc::now().timestamp_millis(),
            metadata,
            parent_ids: parent_ids.clone(),
            hash: hash.clone(),
        };
        
        let mut nodes = self.nodes.write();
        nodes.insert(id.clone(), node.clone());
        
        // Create provenance record
        let provenance = ProvenanceRecord {
            node_id: id.clone(),
            transformations: Vec::new(),
            source_nodes: parent_ids,
            integrity_score: 1.0,
        };
        
        let mut prov = self.provenance.write();
        prov.insert(id, provenance);
        
        node
    }

    /// Record a mutation
    pub fn record_mutation(
        &self,
        mutation_id: &str,
        execution_id: &str,
        mutation_type: &str,
        description: &str,
    ) -> LineageNode {
        let id = format!("mutation_{}", mutation_id);
        let parent_id = format!("capture_{}", execution_id);
        
        let mut metadata = HashMap::new();
        metadata.insert("mutation_type".to_string(), mutation_type.to_string());
        metadata.insert("description".to_string(), description.to_string());
        
        let hash = self.calculate_hash(&NodeType::Mutation, mutation_id, &metadata);
        
        let node = LineageNode {
            id: id.clone(),
            node_type: NodeType::Mutation,
            name: mutation_id.to_string(),
            created_at: Utc::now().timestamp_millis(),
            metadata,
            parent_ids: vec![parent_id.clone()],
            hash: hash.clone(),
        };
        
        let mut nodes = self.nodes.write();
        nodes.insert(id.clone(), node.clone());
        
        // Record transformation
        let transformation = DataTransformation {
            transformation_type: mutation_type.to_string(),
            input_fields: vec!["original".to_string()],
            output_fields: vec!["mutated".to_string()],
            transformation_logic: description.to_string(),
            timestamp: Utc::now().timestamp_millis(),
        };
        
        let mut trans = self.transformations.write();
        trans.insert(id.clone(), vec![transformation.clone()]);
        
        // Create provenance record
        let provenance = ProvenanceRecord {
            node_id: id.clone(),
            transformations: vec![transformation],
            source_nodes: vec![parent_id],
            integrity_score: 0.95,
        };
        
        let mut prov = self.provenance.write();
        prov.insert(id, provenance);
        
        node
    }

    /// Add transformation to a node
    pub fn add_transformation(
        &self,
        node_id: &str,
        transformation: DataTransformation,
    ) -> anyhow::Result<()> {
        let mut trans = self.transformations.write();
        let entry = trans.entry(node_id.to_string()).or_default();
        entry.push(transformation);
        Ok(())
    }

    /// Get node by ID
    pub fn get_node(&self, node_id: &str) -> Option<LineageNode> {
        let nodes = self.nodes.read();
        nodes.get(node_id).cloned()
    }

    /// Get lineage chain for a node
    pub fn get_lineage_chain(&self, node_id: &str) -> Vec<LineageNode> {
        let nodes = self.nodes.read();
        let mut chain = Vec::new();
        let mut current = node_id.to_string();
        let mut visited = HashSet::new();
        
        while let Some(node) = nodes.get(&current) {
            if visited.contains(&current) {
                break; // Circular reference
            }
            visited.insert(current.clone());
            chain.push(node.clone());
            
            // Follow first parent
            if let Some(parent) = node.parent_ids.first() {
                current = parent.clone();
            } else {
                break;
            }
        }
        
        chain.reverse();
        chain
    }

    /// Get all children of a node
    pub fn get_children(&self, node_id: &str) -> Vec<LineageNode> {
        let nodes = self.nodes.read();
        nodes
            .values()
            .filter(|node| node.parent_ids.contains(&node_id.to_string()))
            .cloned()
            .collect()
    }

    /// Get impact analysis - what depends on this node
    pub fn get_impact(&self, node_id: &str) -> Vec<LineageNode> {
        let mut impacted = Vec::new();
        let mut to_check = vec![node_id.to_string()];
        let mut visited = HashSet::new();
        
        while let Some(current) = to_check.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            
            let children = self.get_children(&current);
            for child in &children {
                impacted.push(child.clone());
                to_check.push(child.id.clone());
            }
        }
        
        impacted
    }

    /// Verify integrity of a lineage chain
    pub fn verify_integrity(&self, node_id: &str) -> bool {
        let chain = self.get_lineage_chain(node_id);
        
        for node in &chain {
            let expected_hash = self.calculate_hash(&node.node_type, &node.name, &node.metadata);
            if node.hash != expected_hash {
                return false;
            }
        }
        
        true
    }

    /// Get provenance record
    pub fn get_provenance(&self, node_id: &str) -> Option<ProvenanceRecord> {
        let prov = self.provenance.read();
        prov.get(node_id).cloned()
    }

    /// Get transformations for a node
    pub fn get_transformations(&self, node_id: &str) -> Vec<DataTransformation> {
        let trans = self.transformations.read();
        trans.get(node_id).cloned().unwrap_or_default()
    }

    /// Search nodes by type
    pub fn search_by_type(&self, node_type: NodeType) -> Vec<LineageNode> {
        let nodes = self.nodes.read();
        nodes
            .values()
            .filter(|node| node.node_type == node_type)
            .cloned()
            .collect()
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<LineageNode> {
        let nodes = self.nodes.read();
        nodes.values().cloned().collect()
    }
}

impl Default for DataLineageTracker {
    fn default() -> Self {
        Self::new()
    }
}
