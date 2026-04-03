use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Legal hold status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LegalHoldStatus {
    Active,
    Released,
    Expired,
}

/// Legal hold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalHold {
    pub id: String,
    pub case_name: String,
    pub description: String,
    pub status: LegalHoldStatus,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub created_by: String,
    pub targets: Vec<HoldTarget>,
    pub history: Vec<HoldHistoryEntry>,
}

/// Target for legal hold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldTarget {
    pub target_type: TargetType,
    pub identifier: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TargetType {
    Execution,
    Department,
    Service,
}

/// History entry for hold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldHistoryEntry {
    pub timestamp: i64,
    pub action: String,
    pub actor: String,
    pub details: String,
}

/// Manager for legal holds
#[derive(Debug, Clone)]
pub struct LegalHoldManager {
    holds: Arc<RwLock<HashMap<String, LegalHold>>>,
}

impl LegalHoldManager {
    pub fn new() -> Self {
        Self {
            holds: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new legal hold
    pub fn create_hold(
        &self,
        case_name: String,
        description: String,
        created_by: String,
        targets: Vec<HoldTarget>,
        expires_at: Option<i64>,
    ) -> LegalHold {
        let id = format!("hold_{}", Utc::now().timestamp_millis());
        let now = Utc::now().timestamp_millis();
        
        let hold = LegalHold {
            id: id.clone(),
            case_name,
            description,
            status: LegalHoldStatus::Active,
            created_at: now,
            expires_at,
            created_by: created_by.clone(),
            targets,
            history: vec![HoldHistoryEntry {
                timestamp: now,
                action: "CREATED".to_string(),
                actor: created_by,
                details: "Legal hold created".to_string(),
            }],
        };
        
        let mut holds = self.holds.write();
        holds.insert(id, hold.clone());
        
        hold
    }

    /// Release a legal hold
    pub fn release_hold(&self, hold_id: &str, released_by: &str) -> Option<LegalHold> {
        let mut holds = self.holds.write();
        
        if let Some(hold) = holds.get_mut(hold_id) {
            hold.status = LegalHoldStatus::Released;
            hold.history.push(HoldHistoryEntry {
                timestamp: Utc::now().timestamp_millis(),
                action: "RELEASED".to_string(),
                actor: released_by.to_string(),
                details: "Legal hold released".to_string(),
            });
            return Some(hold.clone());
        }
        
        None
    }

    /// Check if an execution can be deleted
    pub fn can_delete(&self, execution_id: &str) -> (bool, Vec<String>) {
        let holds = self.holds.read();
        let mut blocking_holds = Vec::new();
        
        for hold in holds.values() {
            if hold.status == LegalHoldStatus::Active {
                // Check if hold is expired
                if let Some(expires) = hold.expires_at {
                    if Utc::now().timestamp_millis() > expires {
                        continue; // Hold expired
                    }
                }
                
                // Check if execution is under hold
                for target in &hold.targets {
                    if target.target_type == TargetType::Execution && target.identifier == execution_id {
                        blocking_holds.push(hold.case_name.clone());
                    }
                }
            }
        }
        
        (blocking_holds.is_empty(), blocking_holds)
    }

    /// Get all active holds
    pub fn get_active_holds(&self) -> Vec<LegalHold> {
        let holds = self.holds.read();
        holds
            .values()
            .filter(|h| h.status == LegalHoldStatus::Active)
            .cloned()
            .collect()
    }

    /// Get hold by ID
    pub fn get_hold(&self, hold_id: &str) -> Option<LegalHold> {
        let holds = self.holds.read();
        holds.get(hold_id).cloned()
    }
}

impl Default for LegalHoldManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WORM (Write Once Read Many) storage for hold history
#[derive(Debug, Clone)]
pub struct WORMStorage {
    records: Arc<RwLock<Vec<WORMRecord>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WORMRecord {
    pub timestamp: i64,
    pub record_type: String,
    pub data: String,
    pub checksum: String,
}

impl WORMStorage {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Append an immutable record
    pub fn append(&self, record_type: String, data: String) -> WORMRecord {
        use sha2::{Sha256, Digest};
        use hex::encode;
        
        let timestamp = Utc::now().timestamp_millis();
        let input = format!("{}{}{}", timestamp, record_type, data);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let checksum = encode(hasher.finalize());
        
        let record = WORMRecord {
            timestamp,
            record_type,
            data,
            checksum,
        };
        
        let mut records = self.records.write();
        records.push(record.clone());
        
        record
    }

    /// Verify integrity of all records
    pub fn verify_integrity(&self) -> bool {
        let records = self.records.read();
        
        for record in records.iter() {
            use sha2::{Sha256, Digest};
            use hex::encode;
            
            let input = format!("{}{}{}", record.timestamp, record.record_type, record.data);
            let mut hasher = Sha256::new();
            hasher.update(input.as_bytes());
            let expected = encode(hasher.finalize());
            
            if record.checksum != expected {
                return false;
            }
        }
        
        true
    }

    /// Get all records
    pub fn get_all(&self) -> Vec<WORMRecord> {
        let records = self.records.read();
        records.clone()
    }
}

impl Default for WORMStorage {
    fn default() -> Self {
        Self::new()
    }
}
