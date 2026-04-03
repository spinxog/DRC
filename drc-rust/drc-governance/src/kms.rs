use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// KMS provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KMSProvider {
    AwsKms,
    AzureKeyVault,
    GcpKms,
    HashiCorpVault,
    OnPremHsm,
}

/// Encryption key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    pub key_id: String,
    pub key_arn: String,
    pub provider: KMSProvider,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub key_state: KeyState,
    pub key_usage: String,
    pub key_spec: String,
    pub auto_rotation: bool,
    pub rotation_period_days: u32,
    pub last_rotated_at: Option<i64>,
    pub next_rotation_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KeyState {
    Enabled,
    Disabled,
    PendingDeletion,
}

/// Encrypted data envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEncryptionEnvelope {
    pub ciphertext: Vec<u8>,
    pub encrypted_key: Vec<u8>,
    pub iv: Vec<u8>,
    pub algorithm: String,
    pub key_id: String,
}

/// Key rotation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    pub enabled: bool,
    pub rotation_period_days: u32,
    pub automatic: bool,
    pub notify_before_days: u32,
}

/// KMS integration manager
#[derive(Debug, Clone)]
pub struct KMSIntegration {
    keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    rotation_policies: Arc<RwLock<HashMap<String, KeyRotationPolicy>>>,
    provider: KMSProvider,
    endpoint: String,
}

impl KMSIntegration {
    pub fn new(provider: KMSProvider, endpoint: String) -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            rotation_policies: Arc::new(RwLock::new(HashMap::new())),
            provider,
            endpoint,
        }
    }

    /// Create a new encryption key
    pub fn create_key(
        &self,
        key_usage: String,
        key_spec: String,
        auto_rotation: bool,
    ) -> EncryptionKey {
        let key_id = format!("key_{}", Utc::now().timestamp_millis());
        let key_arn = format!("arn:kms:{}:{}", self.endpoint, key_id);
        let now = Utc::now().timestamp_millis();
        
        let key = EncryptionKey {
            key_id: key_id.clone(),
            key_arn,
            provider: self.provider,
            created_at: now,
            expires_at: None,
            key_state: KeyState::Enabled,
            key_usage,
            key_spec,
            auto_rotation,
            rotation_period_days: if auto_rotation { 90 } else { 0 },
            last_rotated_at: None,
            next_rotation_at: if auto_rotation {
                Some(now + (90 * 24 * 60 * 60 * 1000))
            } else {
                None
            },
        };
        
        let mut keys = self.keys.write();
        keys.insert(key_id.clone(), key.clone());
        
        // Set default rotation policy
        if auto_rotation {
            let policy = KeyRotationPolicy {
                enabled: true,
                rotation_period_days: 90,
                automatic: true,
                notify_before_days: 7,
            };
            let mut policies = self.rotation_policies.write();
            policies.insert(key_id, policy);
        }
        
        key
    }

    /// Get key by ID
    pub fn get_key(&self, key_id: &str) -> Option<EncryptionKey> {
        let keys = self.keys.read();
        keys.get(key_id).cloned()
    }

    /// Encrypt data (simulated)
    pub fn encrypt(&self, key_id: &str, plaintext: &[u8]) -> anyhow::Result<DataEncryptionEnvelope> {
        let keys = self.keys.read();
        let key = keys.get(key_id)
            .ok_or_else(|| anyhow::anyhow!("Key not found: {}", key_id))?;
        
        if key.key_state != KeyState::Enabled {
            return Err(anyhow::anyhow!("Key is not enabled: {}", key_id));
        }
        
        // Simulated encryption - in production, use actual KMS API
        let iv = vec![0u8; 16]; // In production, generate random IV
        let encrypted_key = vec![0u8; 32]; // Simulated encrypted key
        
        // Simple XOR "encryption" for demo purposes
        let ciphertext: Vec<u8> = plaintext.iter().enumerate()
            .map(|(i, &b)| b ^ (i as u8))
            .collect();
        
        Ok(DataEncryptionEnvelope {
            ciphertext,
            encrypted_key,
            iv,
            algorithm: "AES-256-GCM".to_string(),
            key_id: key_id.to_string(),
        })
    }

    /// Decrypt data (simulated)
    pub fn decrypt(&self, envelope: &DataEncryptionEnvelope) -> anyhow::Result<Vec<u8>> {
        let keys = self.keys.read();
        let key = keys.get(&envelope.key_id)
            .ok_or_else(|| anyhow::anyhow!("Key not found: {}", envelope.key_id))?;
        
        if key.key_state != KeyState::Enabled {
            return Err(anyhow::anyhow!("Key is not enabled: {}", envelope.key_id));
        }
        
        // Simulated decryption - reverse the XOR
        let plaintext: Vec<u8> = envelope.ciphertext.iter().enumerate()
            .map(|(i, &b)| b ^ (i as u8))
            .collect();
        
        Ok(plaintext)
    }

    /// Rotate a key
    pub fn rotate_key(&self, key_id: &str) -> anyhow::Result<EncryptionKey> {
        let mut keys = self.keys.write();
        
        if let Some(key) = keys.get_mut(key_id) {
            let now = Utc::now().timestamp_millis();
            key.last_rotated_at = Some(now);
            key.next_rotation_at = Some(now + (key.rotation_period_days as i64 * 24 * 60 * 60 * 1000));
            
            Ok(key.clone())
        } else {
            Err(anyhow::anyhow!("Key not found: {}", key_id))
        }
    }

    /// Schedule key rotation
    pub fn schedule_rotation(&self, key_id: &str, policy: KeyRotationPolicy) -> anyhow::Result<()> {
        let keys = self.keys.read();
        if !keys.contains_key(key_id) {
            return Err(anyhow::anyhow!("Key not found: {}", key_id));
        }
        drop(keys);
        
        let mut policies = self.rotation_policies.write();
        policies.insert(key_id.to_string(), policy);
        
        Ok(())
    }

    /// Get rotation policy
    pub fn get_rotation_policy(&self, key_id: &str) -> Option<KeyRotationPolicy> {
        let policies = self.rotation_policies.read();
        policies.get(key_id).cloned()
    }

    /// Disable a key
    pub fn disable_key(&self, key_id: &str) -> anyhow::Result<()> {
        let mut keys = self.keys.write();
        
        if let Some(key) = keys.get_mut(key_id) {
            key.key_state = KeyState::Disabled;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Key not found: {}", key_id))
        }
    }

    /// Enable a key
    pub fn enable_key(&self, key_id: &str) -> anyhow::Result<()> {
        let mut keys = self.keys.write();
        
        if let Some(key) = keys.get_mut(key_id) {
            key.key_state = KeyState::Enabled;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Key not found: {}", key_id))
        }
    }

    /// Schedule key deletion
    pub fn schedule_key_deletion(&self, key_id: &str, pending_window_days: u32) -> anyhow::Result<()> {
        let mut keys = self.keys.write();
        
        if let Some(key) = keys.get_mut(key_id) {
            key.key_state = KeyState::PendingDeletion;
            let now = Utc::now().timestamp_millis();
            key.expires_at = Some(now + (pending_window_days as i64 * 24 * 60 * 60 * 1000));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Key not found: {}", key_id))
        }
    }

    /// Cancel key deletion
    pub fn cancel_key_deletion(&self, key_id: &str) -> anyhow::Result<()> {
        let mut keys = self.keys.write();
        
        if let Some(key) = keys.get_mut(key_id) {
            if key.key_state == KeyState::PendingDeletion {
                key.key_state = KeyState::Disabled;
                key.expires_at = None;
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Key not found: {}", key_id))
        }
    }

    /// Check if key needs rotation
    pub fn needs_rotation(&self, key_id: &str) -> bool {
        let keys = self.keys.read();
        
        if let Some(key) = keys.get(key_id) {
            if !key.auto_rotation {
                return false;
            }
            
            if let Some(next_rotation) = key.next_rotation_at {
                Utc::now().timestamp_millis() >= next_rotation
            } else {
                false
            }
        } else {
            false
        }
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<EncryptionKey> {
        let keys = self.keys.read();
        keys.values().cloned().collect()
    }

    /// List keys pending rotation
    pub fn list_keys_pending_rotation(&self) -> Vec<String> {
        let keys = self.keys.read();
        keys
            .values()
            .filter(|k| self.needs_rotation(&k.key_id))
            .map(|k| k.key_id.clone())
            .collect()
    }
}

impl Default for KMSIntegration {
    fn default() -> Self {
        Self::new(KMSProvider::AwsKms, "us-east-1".to_string())
    }
}
