use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use hex::encode;

/// Deletion request status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeletionStatus {
    Pending,
    Verified,
    LegalHoldCheck,
    Deleting,
    Completed,
    Failed,
    Partial,
}

/// Deletion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRequest {
    pub request_id: String,
    pub subject_id: String,
    pub execution_ids: Vec<String>,
    pub status: DeletionStatus,
    pub requested_at: i64,
    pub verification_method: String,
    pub legal_hold_check: bool,
    pub deletion_type: DeletionType,
    pub scope: DeletionScope,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeletionType {
    Full,
    Partial,
    Anonymization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionScope {
    pub events: bool,
    pub metadata: bool,
    pub snapshots: bool,
    pub lineage: bool,
    pub audit_logs: bool,
}

/// Deletion verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionVerification {
    pub request_id: String,
    pub verified_at: i64,
    pub verified_by: String,
    pub method: String,
    pub evidence: String,
}

/// Deletion certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionCertificate {
    pub certificate_id: String,
    pub request_id: String,
    pub subject_id: String,
    pub execution_ids: Vec<String>,
    pub deleted_at: i64,
    pub deleted_by: String,
    pub deletion_scope: DeletionScope,
    pub verification_hash: String,
    pub signature: String,
}

/// Retention policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub policy_id: String,
    pub data_classification: String,
    pub retention_days: u32,
    pub auto_delete: bool,
    pub require_approval: bool,
    pub legal_hold_exempt: bool,
}

/// Manager for Right to be Forgotten
#[derive(Debug, Clone)]
pub struct RightToBeForgottenManager {
    requests: Arc<RwLock<HashMap<String, DeletionRequest>>>,
    certificates: Arc<RwLock<HashMap<String, DeletionCertificate>>>,
    retention_policies: Arc<RwLock<HashMap<String, RetentionPolicy>>>,
}

impl RightToBeForgottenManager {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            certificates: Arc::new(RwLock::new(HashMap::new())),
            retention_policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Submit a deletion request
    pub fn submit_request(
        &self,
        subject_id: String,
        execution_ids: Vec<String>,
        deletion_type: DeletionType,
        scope: DeletionScope,
        reason: String,
        verification_method: String,
    ) -> DeletionRequest {
        let request_id = format!("del_req_{}", Utc::now().timestamp_millis());
        
        let request = DeletionRequest {
            request_id: request_id.clone(),
            subject_id,
            execution_ids,
            status: DeletionStatus::Pending,
            requested_at: Utc::now().timestamp_millis(),
            verification_method,
            legal_hold_check: false,
            deletion_type,
            scope,
            reason,
        };
        
        let mut requests = self.requests.write();
        requests.insert(request_id, request.clone());
        
        request
    }

    /// Verify a deletion request
    pub fn verify_request(
        &self,
        request_id: &str,
        verified_by: String,
        method: String,
        evidence: String,
    ) -> anyhow::Result<DeletionVerification> {
        let mut requests = self.requests.write();
        
        if let Some(request) = requests.get_mut(request_id) {
            request.status = DeletionStatus::Verified;
            
            let verification = DeletionVerification {
                request_id: request_id.to_string(),
                verified_at: Utc::now().timestamp_millis(),
                verified_by,
                method,
                evidence,
            };
            
            Ok(verification)
        } else {
            Err(anyhow::anyhow!("Request not found: {}", request_id))
        }
    }

    /// Check for legal holds
    pub fn check_legal_holds(&self, request_id: &str) -> anyhow::Result<Vec<String>> {
        let mut requests = self.requests.write();
        
        if let Some(request) = requests.get_mut(request_id) {
            request.status = DeletionStatus::LegalHoldCheck;
            request.legal_hold_check = true;
            
            // Return list of execution IDs under hold
            // In real implementation, query LegalHoldManager
            Ok(Vec::new())
        } else {
            Err(anyhow::anyhow!("Request not found: {}", request_id))
        }
    }

    /// Execute deletion
    pub fn execute_deletion(
        &self,
        request_id: &str,
        deleted_by: String,
    ) -> anyhow::Result<DeletionCertificate> {
        let mut requests = self.requests.write();
        
        if let Some(request) = requests.get_mut(request_id) {
            request.status = DeletionStatus::Deleting;
            
            // Calculate verification hash
            let hash_input = format!(
                "{}{}{:?}{}",
                request.subject_id,
                request.requested_at,
                request.execution_ids,
                deleted_by
            );
            let mut hasher = Sha256::new();
            hasher.update(hash_input.as_bytes());
            let verification_hash = encode(hasher.finalize());
            
            // Generate certificate
            let certificate = DeletionCertificate {
                certificate_id: format!("cert_{}", Utc::now().timestamp_millis()),
                request_id: request_id.to_string(),
                subject_id: request.subject_id.clone(),
                execution_ids: request.execution_ids.clone(),
                deleted_at: Utc::now().timestamp_millis(),
                deleted_by: deleted_by.clone(),
                deletion_scope: request.scope.clone(),
                verification_hash: verification_hash.clone(),
                signature: format!("sig_{}", &verification_hash[..16]),
            };
            
            request.status = DeletionStatus::Completed;
            
            let mut certificates = self.certificates.write();
            certificates.insert(certificate.certificate_id.clone(), certificate.clone());
            
            Ok(certificate)
        } else {
            Err(anyhow::anyhow!("Request not found: {}", request_id))
        }
    }

    /// Get deletion request
    pub fn get_request(&self, request_id: &str) -> Option<DeletionRequest> {
        let requests = self.requests.read();
        requests.get(request_id).cloned()
    }

    /// Get certificate
    pub fn get_certificate(&self, certificate_id: &str) -> Option<DeletionCertificate> {
        let certificates = self.certificates.read();
        certificates.get(certificate_id).cloned()
    }

    /// List requests by status
    pub fn list_requests_by_status(&self, status: DeletionStatus) -> Vec<DeletionRequest> {
        let requests = self.requests.read();
        requests
            .values()
            .filter(|r| r.status == status)
            .cloned()
            .collect()
    }

    /// Set retention policy
    pub fn set_retention_policy(&self, policy: RetentionPolicy) {
        let mut policies = self.retention_policies.write();
        policies.insert(policy.data_classification.clone(), policy);
    }

    /// Get retention policy
    pub fn get_retention_policy(&self, data_classification: &str) -> Option<RetentionPolicy> {
        let policies = self.retention_policies.read();
        policies.get(data_classification).cloned()
    }

    /// Check expired executions based on retention policy
    pub fn check_expired_executions(
        &self,
        execution_metadata: &[(String, i64, String)], // (id, timestamp, classification)
    ) -> Vec<String> {
        let policies = self.retention_policies.read();
        let now = Utc::now().timestamp_millis();
        let mut expired = Vec::new();
        
        for (id, timestamp, classification) in execution_metadata {
            if let Some(policy) = policies.get(classification) {
                let retention_ms = (policy.retention_days as i64) * 24 * 60 * 60 * 1000;
                if now - timestamp > retention_ms {
                    expired.push(id.clone());
                }
            }
        }
        
        expired
    }

    /// Generate deletion report
    pub fn generate_deletion_report(
        &self,
        subject_id: &str,
    ) -> DeletionReport {
        let requests = self.requests.read();
        let certificates = self.certificates.read();
        
        let subject_requests: Vec<_> = requests
            .values()
            .filter(|r| r.subject_id == subject_id)
            .collect();
        
        let subject_certificates: Vec<_> = certificates
            .values()
            .filter(|c| c.subject_id == subject_id)
            .cloned()
            .collect();
        
        DeletionReport {
            subject_id: subject_id.to_string(),
            total_requests: subject_requests.len(),
            completed_deletions: subject_requests.iter().filter(|r| r.status == DeletionStatus::Completed).count(),
            pending_requests: subject_requests.iter().filter(|r| r.status == DeletionStatus::Pending).count(),
            failed_requests: subject_requests.iter().filter(|r| r.status == DeletionStatus::Failed).count(),
            certificates: subject_certificates,
            generated_at: Utc::now().timestamp_millis(),
        }
    }
}

impl Default for RightToBeForgottenManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Deletion report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionReport {
    pub subject_id: String,
    pub total_requests: usize,
    pub completed_deletions: usize,
    pub pending_requests: usize,
    pub failed_requests: usize,
    pub certificates: Vec<DeletionCertificate>,
    pub generated_at: i64,
}
