use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Geographic zone for data residency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GeoZone {
    UsEast,
    UsWest,
    EuWest,
    EuCentral,
    AsiaPacific,
    SouthAmerica,
    Africa,
    MiddleEast,
}

/// Data residency policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataResidencyPolicy {
    pub policy_id: String,
    pub data_classification: String,
    pub primary_zone: GeoZone,
    pub allowed_zones: Vec<GeoZone>,
    pub restricted_zones: Vec<GeoZone>,
    pub requires_local_processing: bool,
    pub encryption_required: bool,
}

/// Cross-border transfer rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRule {
    pub rule_id: String,
    pub from_zone: GeoZone,
    pub to_zone: GeoZone,
    pub data_classification: String,
    pub allowed: bool,
    pub legal_basis: String, // AdequacyDecision, SCCs, BCRs, Consent
    pub requires_approval: bool,
    pub approval_workflow: Option<String>,
    pub encryption_required: bool,
}

/// Data transfer approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferApproval {
    pub approval_id: String,
    pub transfer_request_id: String,
    pub approved_by: String,
    pub approved_at: i64,
    pub expires_at: i64,
    pub conditions: Vec<String>,
}

/// Data transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub request_id: String,
    pub execution_id: String,
    pub from_zone: GeoZone,
    pub to_zone: GeoZone,
    pub data_classification: String,
    pub requested_by: String,
    pub requested_at: i64,
    pub status: TransferRequestStatus,
    pub legal_basis: String,
    pub data_volume_estimate: Option<String>,
    pub retention_period_days: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferRequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

/// Cross-border data controller
#[derive(Debug, Clone)]
pub struct CrossBorderDataController {
    residency_policies: Arc<RwLock<HashMap<String, DataResidencyPolicy>>>,
    transfer_rules: Arc<RwLock<HashMap<String, TransferRule>>>,
    transfer_requests: Arc<RwLock<HashMap<String, TransferRequest>>>,
    transfer_approvals: Arc<RwLock<HashMap<String, TransferApproval>>>,
    execution_locations: Arc<RwLock<HashMap<String, GeoZone>>>,
    transfer_history: Arc<RwLock<Vec<TransferRecord>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    pub timestamp: i64,
    pub execution_id: String,
    pub from_zone: GeoZone,
    pub to_zone: GeoZone,
    pub data_classification: String,
    pub legal_basis: String,
    pub approved: bool,
}

impl CrossBorderDataController {
    pub fn new() -> Self {
        let controller = Self {
            residency_policies: Arc::new(RwLock::new(HashMap::new())),
            transfer_rules: Arc::new(RwLock::new(HashMap::new())),
            transfer_requests: Arc::new(RwLock::new(HashMap::new())),
            transfer_approvals: Arc::new(RwLock::new(HashMap::new())),
            execution_locations: Arc::new(RwLock::new(HashMap::new())),
            transfer_history: Arc::new(RwLock::new(Vec::new())),
        };
        
        controller.initialize_default_rules();
        controller
    }

    fn initialize_default_rules(&self) {
        let mut rules = self.transfer_rules.write();
        
        // EU to US transfers (require SCCs or other mechanism)
        let eu_us_rule = TransferRule {
            rule_id: "eu_to_us".to_string(),
            from_zone: GeoZone::EuWest,
            to_zone: GeoZone::UsEast,
            data_classification: "all".to_string(),
            allowed: true,
            legal_basis: "SCCs".to_string(),
            requires_approval: false,
            approval_workflow: None,
            encryption_required: true,
        };
        rules.insert("eu_to_us".to_string(), eu_us_rule);
        
        // Within EU (adequacy)
        let eu_eu_rule = TransferRule {
            rule_id: "eu_to_eu".to_string(),
            from_zone: GeoZone::EuWest,
            to_zone: GeoZone::EuCentral,
            data_classification: "all".to_string(),
            allowed: true,
            legal_basis: "AdequacyDecision".to_string(),
            requires_approval: false,
            approval_workflow: None,
            encryption_required: false,
        };
        rules.insert("eu_to_eu".to_string(), eu_eu_rule);
        
        // US internal
        let us_us_rule = TransferRule {
            rule_id: "us_to_us".to_string(),
            from_zone: GeoZone::UsEast,
            to_zone: GeoZone::UsWest,
            data_classification: "all".to_string(),
            allowed: true,
            legal_basis: "Domestic".to_string(),
            requires_approval: false,
            approval_workflow: None,
            encryption_required: false,
        };
        rules.insert("us_to_us".to_string(), us_us_rule);
    }

    /// Set data residency policy
    pub fn set_residency_policy(&self, policy: DataResidencyPolicy) {
        let mut policies = self.residency_policies.write();
        policies.insert(policy.data_classification.clone(), policy);
    }

    /// Get residency policy
    pub fn get_residency_policy(&self, data_classification: &str) -> Option<DataResidencyPolicy> {
        let policies = self.residency_policies.read();
        policies.get(data_classification).cloned()
    }

    /// Register execution location
    pub fn register_execution_location(&self, execution_id: String, zone: GeoZone) {
        let mut locations = self.execution_locations.write();
        locations.insert(execution_id, zone);
    }

    /// Get execution location
    pub fn get_execution_location(&self, execution_id: &str) -> Option<GeoZone> {
        let locations = self.execution_locations.read();
        locations.get(execution_id).copied()
    }

    /// Check if transfer is allowed
    pub fn check_transfer_allowed(
        &self,
        from_zone: GeoZone,
        to_zone: GeoZone,
        data_classification: &str,
    ) -> (bool, String, bool) {
        // Check for specific transfer rule
        let rules = self.transfer_rules.read();
        let rule_key = format!("{:?}_to_{:?}", from_zone, to_zone).to_lowercase();
        
        if let Some(rule) = rules.get(&rule_key) {
            if rule.data_classification == "all" || rule.data_classification == data_classification {
                return (
                    rule.allowed,
                    rule.legal_basis.clone(),
                    rule.requires_approval,
                );
            }
        }
        
        // Check residency policy
        let policies = self.residency_policies.read();
        if let Some(policy) = policies.get(data_classification) {
            if policy.restricted_zones.contains(&to_zone) {
                return (false, "RestrictedZone".to_string(), false);
            }
            
            if policy.allowed_zones.contains(&to_zone) || policy.primary_zone == to_zone {
                return (true, "ResidencyPolicy".to_string(), policy.requires_local_processing);
            }
        }
        
        // Default: allow with caution
        (true, "Default".to_string(), false)
    }

    /// Submit transfer request
    pub fn submit_transfer_request(
        &self,
        execution_id: String,
        from_zone: GeoZone,
        to_zone: GeoZone,
        data_classification: String,
        requested_by: String,
        legal_basis: String,
        data_volume_estimate: Option<String>,
        retention_period_days: u32,
    ) -> TransferRequest {
        let request_id = format!("xfer_req_{}", Utc::now().timestamp_millis());
        
        let request = TransferRequest {
            request_id: request_id.clone(),
            execution_id: execution_id.clone(),
            from_zone,
            to_zone,
            data_classification: data_classification.clone(),
            requested_by,
            requested_at: Utc::now().timestamp_millis(),
            status: TransferRequestStatus::Pending,
            legal_basis,
            data_volume_estimate,
            retention_period_days,
        };
        
        // Check if auto-approved
        let (allowed, _, requires_approval) = self.check_transfer_allowed(
            from_zone,
            to_zone,
            &data_classification,
        );
        
        let mut requests = self.transfer_requests.write();
        let mut request_mut = request.clone();
        
        if allowed && !requires_approval {
            request_mut.status = TransferRequestStatus::Approved;
        }
        
        requests.insert(request_id, request_mut.clone());
        
        request_mut
    }

    /// Approve transfer request
    pub fn approve_transfer(
        &self,
        request_id: &str,
        approved_by: String,
        conditions: Vec<String>,
    ) -> anyhow::Result<TransferApproval> {
        let mut requests = self.transfer_requests.write();
        
        if let Some(request) = requests.get_mut(request_id) {
            request.status = TransferRequestStatus::Approved;
            
            let approval = TransferApproval {
                approval_id: format!("approval_{}", Utc::now().timestamp_millis()),
                transfer_request_id: request_id.to_string(),
                approved_by,
                approved_at: Utc::now().timestamp_millis(),
                expires_at: Utc::now().timestamp_millis() + (30 * 24 * 60 * 60 * 1000), // 30 days
                conditions,
            };
            
            let mut approvals = self.transfer_approvals.write();
            approvals.insert(approval.approval_id.clone(), approval.clone());
            
            // Record in history
            let mut history = self.transfer_history.write();
            history.push(TransferRecord {
                timestamp: Utc::now().timestamp_millis(),
                execution_id: request.execution_id.clone(),
                from_zone: request.from_zone,
                to_zone: request.to_zone,
                data_classification: request.data_classification.clone(),
                legal_basis: request.legal_basis.clone(),
                approved: true,
            });
            
            Ok(approval)
        } else {
            Err(anyhow::anyhow!("Transfer request not found: {}", request_id))
        }
    }

    /// Reject transfer request
    pub fn reject_transfer(&self, request_id: &str) -> anyhow::Result<()> {
        let mut requests = self.transfer_requests.write();
        
        if let Some(request) = requests.get_mut(request_id) {
            request.status = TransferRequestStatus::Rejected;
            
            // Record in history
            let mut history = self.transfer_history.write();
            history.push(TransferRecord {
                timestamp: Utc::now().timestamp_millis(),
                execution_id: request.execution_id.clone(),
                from_zone: request.from_zone,
                to_zone: request.to_zone,
                data_classification: request.data_classification.clone(),
                legal_basis: request.legal_basis.clone(),
                approved: false,
            });
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transfer request not found: {}", request_id))
        }
    }

    /// Get transfer request
    pub fn get_transfer_request(&self, request_id: &str) -> Option<TransferRequest> {
        let requests = self.transfer_requests.read();
        requests.get(request_id).cloned()
    }

    /// List pending transfer requests
    pub fn list_pending_requests(&self) -> Vec<TransferRequest> {
        let requests = self.transfer_requests.read();
        requests
            .values()
            .filter(|r| r.status == TransferRequestStatus::Pending)
            .cloned()
            .collect()
    }

    /// Generate residency report
    pub fn generate_residency_report(&self) -> ResidencyReport {
        let locations = self.execution_locations.read();
        let history = self.transfer_history.read();
        
        let mut zone_counts: HashMap<GeoZone, u32> = HashMap::new();
        for zone in locations.values() {
            *zone_counts.entry(*zone).or_insert(0) += 1;
        }
        
        let total_transfers = history.len() as u32;
        let approved_transfers = history.iter().filter(|h| h.approved).count() as u32;
        let rejected_transfers = total_transfers - approved_transfers;
        
        ResidencyReport {
            generated_at: Utc::now().timestamp_millis(),
            executions_by_zone: zone_counts,
            total_transfers,
            approved_transfers,
            rejected_transfers,
            recent_transfers: history.iter().rev().take(100).cloned().collect(),
        }
    }

    /// Get transfer history for execution
    pub fn get_transfer_history(&self, execution_id: &str) -> Vec<TransferRecord> {
        let history = self.transfer_history.read();
        history
            .iter()
            .filter(|h| h.execution_id == execution_id)
            .cloned()
            .collect()
    }

    /// Add transfer rule
    pub fn add_transfer_rule(&self, rule: TransferRule) {
        let mut rules = self.transfer_rules.write();
        rules.insert(rule.rule_id.clone(), rule);
    }
}

impl Default for CrossBorderDataController {
    fn default() -> Self {
        Self::new()
    }
}

/// Residency report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidencyReport {
    pub generated_at: i64,
    pub executions_by_zone: HashMap<GeoZone, u32>,
    pub total_transfers: u32,
    pub approved_transfers: u32,
    pub rejected_transfers: u32,
    pub recent_transfers: Vec<TransferRecord>,
}
