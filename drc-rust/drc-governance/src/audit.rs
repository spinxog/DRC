use std::sync::Arc;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use hex::encode;

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: i64,
    pub action: String,
    pub actor: String,
    pub resource: String,
    pub resource_id: String,
    pub details: String,
    pub previous_hash: String,
    pub hash: String,
    pub signature: Option<String>,
}

/// Immutable audit log with cryptographic chain
#[derive(Debug, Clone)]
pub struct ImmutableAuditLog {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    private_key: Option<Vec<u8>>,
    public_key: Option<Vec<u8>>,
}

impl ImmutableAuditLog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            private_key: None,
            public_key: None,
        }
    }

    /// Create with existing keys for persistence
    pub fn with_keys(private_key: Vec<u8>, public_key: Vec<u8>) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            private_key: Some(private_key),
            public_key: Some(public_key),
        }
    }

    /// Generate RSA key pair
    pub fn generate_keys(&mut self) -> anyhow::Result<()> {
        use rsa::{RsaPrivateKey, RsaPublicKey};
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
        use rand::thread_rng;
        
        let mut rng = thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048)?;
        let public_key = RsaPublicKey::from(&private_key);
        
        self.private_key = Some(private_key.to_pkcs8_der()?.as_bytes().to_vec());
        self.public_key = Some(public_key.to_public_key_der()?.as_bytes().to_vec());
        
        Ok(())
    }

    /// Append an entry to the audit log
    pub fn append(&self, action: String, actor: String, resource: String, resource_id: String, details: String) -> AuditEntry {
        let entries = self.entries.read();
        let previous_hash = if let Some(last) = entries.last() {
            last.hash.clone()
        } else {
            "0".to_string()
        };
        
        let id = format!("audit_{}", Utc::now().timestamp_millis());
        let timestamp = Utc::now().timestamp_millis();
        
        // Calculate hash
        let data = format!("{}{}{}{}{}{}", id, timestamp, action, actor, resource, resource_id);
        let hash = self.calculate_hash(&data, &previous_hash);
        
        // Sign if keys available
        let signature = self.sign_entry(&hash);
        
        let entry = AuditEntry {
            id,
            timestamp,
            action,
            actor,
            resource,
            resource_id,
            details,
            previous_hash,
            hash: hash.clone(),
            signature,
        };
        
        drop(entries);
        let mut entries = self.entries.write();
        entries.push(entry.clone());
        
        entry
    }

    fn calculate_hash(&self, data: &str, previous_hash: &str) -> String {
        let input = format!("{}{}", data, previous_hash);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        encode(hasher.finalize())
    }

    fn sign_entry(&self, hash: &str) -> Option<String> {
        if let Some(ref _private_key) = self.private_key {
            // In a real implementation, use rsa crate to sign
            // For now, return a placeholder
            Some(format!("sig_{}", hash.chars().take(16).collect::<String>()))
        } else {
            None
        }
    }

    /// Verify integrity of the entire chain
    pub fn verify_chain(&self) -> bool {
        let entries = self.entries.read();
        
        for i in 0..entries.len() {
            let entry = &entries[i];
            let expected_prev = if i == 0 {
                "0".to_string()
            } else {
                entries[i - 1].hash.clone()
            };
            
            if entry.previous_hash != expected_prev {
                return false;
            }
            
            let data = format!("{}{}{}{}{}", entry.id, entry.timestamp, entry.action, entry.actor, entry.resource);
            let expected_hash = self.calculate_hash(&data, &entry.previous_hash);
            
            if entry.hash != expected_hash {
                return false;
            }
        }
        
        true
    }

    /// Get all entries
    pub fn get_entries(&self) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries.clone()
    }

    /// Get entries for a specific resource
    pub fn get_entries_for_resource(&self, resource_id: &str) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .filter(|e| e.resource_id == resource_id)
            .cloned()
            .collect()
    }
}

impl Default for ImmutableAuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// DRC-specific audit logger
pub struct DRCAuditLogger {
    audit_log: ImmutableAuditLog,
}

impl DRCAuditLogger {
    pub fn new() -> Self {
        Self {
            audit_log: ImmutableAuditLog::new(),
        }
    }

    pub fn with_keys(private_key: Vec<u8>, public_key: Vec<u8>) -> Self {
        Self {
            audit_log: ImmutableAuditLog::with_keys(private_key, public_key),
        }
    }

    /// Log a replay operation
    pub fn log_replay(&self, execution_id: &str, replay_id: &str, actor: &str) {
        self.audit_log.append(
            "REPLAY_STARTED".to_string(),
            actor.to_string(),
            "execution".to_string(),
            execution_id.to_string(),
            format!("Replay {} started", replay_id),
        );
    }

    /// Log a mutation
    pub fn log_mutation(&self, execution_id: &str, mutation_type: &str, actor: &str) {
        self.audit_log.append(
            "MUTATION_APPLIED".to_string(),
            actor.to_string(),
            "execution".to_string(),
            execution_id.to_string(),
            format!("Mutation {} applied", mutation_type),
        );
    }

    /// Log a divergence
    pub fn log_divergence(&self, execution_id: &str, divergence_type: &str, actor: &str) {
        self.audit_log.append(
            "DIVERGENCE_DETECTED".to_string(),
            actor.to_string(),
            "execution".to_string(),
            execution_id.to_string(),
            format!("Divergence {} detected", divergence_type),
        );
    }

    /// Log data access
    pub fn log_access(&self, resource_id: &str, resource_type: &str, action: &str, actor: &str) {
        self.audit_log.append(
            format!("ACCESS_{}", action.to_uppercase()),
            actor.to_string(),
            resource_type.to_string(),
            resource_id.to_string(),
            format!("Accessed {} {}", resource_type, resource_id),
        );
    }

    /// Log compliance event
    pub fn log_compliance(&self, event_type: &str, details: &str, actor: &str) {
        self.audit_log.append(
            format!("COMPLIANCE_{}", event_type.to_uppercase()),
            actor.to_string(),
            "compliance".to_string(),
            "system".to_string(),
            details.to_string(),
        );
    }

    /// Redact entries (creates new audit entry, doesn't modify history)
    pub fn redact_entries(&self, entry_ids: Vec<String>, reason: &str, actor: &str) {
        self.audit_log.append(
            "REDACTION".to_string(),
            actor.to_string(),
            "audit_log".to_string(),
            entry_ids.join(","),
            format!("Entries redacted: {}", reason),
        );
    }

    /// Verify audit log integrity
    pub fn verify_integrity(&self) -> bool {
        self.audit_log.verify_chain()
    }

    /// Get the underlying audit log
    pub fn get_audit_log(&self) -> &ImmutableAuditLog {
        &self.audit_log
    }
}

impl Default for DRCAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}
