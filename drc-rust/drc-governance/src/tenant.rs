use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Tenant context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: String,
    pub tenant_name: String,
    pub department: String,
    pub created_at: i64,
    pub metadata: HashMap<String, String>,
}

/// Tenant isolation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPolicy {
    pub tenant_id: String,
    pub data_isolation: DataIsolationLevel,
    pub access_control: AccessControlModel,
    pub cross_tenant_access: Vec<CrossTenantGrant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataIsolationLevel {
    Database,
    Schema,
    RowLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccessControlModel {
    Rbac,
    Abac,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantTier {
    Free,
    Basic,
    Standard,
    Enterprise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    Read,
    Write,
    Delete,
    Admin,
}

/// Cross-tenant access grant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossTenantGrant {
    pub granted_to: String,
    pub resource_type: String,
    pub resource_id: String,
    pub permissions: Vec<String>,
    pub granted_at: i64,
    pub expires_at: Option<i64>,
    pub granted_by: String,
}

/// Manager for tenant isolation
#[derive(Debug, Clone)]
pub struct TenantIsolationManager {
    tenants: Arc<RwLock<HashMap<String, TenantContext>>>,
    policies: Arc<RwLock<HashMap<String, TenantPolicy>>>,
    execution_tenants: Arc<RwLock<HashMap<String, String>>>, // execution_id -> tenant_id
}

impl TenantIsolationManager {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            execution_tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new tenant
    pub fn register_tenant(
        &self,
        tenant_id: String,
        tenant_name: String,
        department: String,
        metadata: HashMap<String, String>,
    ) -> TenantContext {
        let context = TenantContext {
            tenant_id: tenant_id.clone(),
            tenant_name,
            department,
            created_at: Utc::now().timestamp_millis(),
            metadata,
        };
        
        let mut tenants = self.tenants.write();
        tenants.insert(tenant_id.clone(), context.clone());
        
        // Create default policy
        let policy = TenantPolicy {
            tenant_id: tenant_id.clone(),
            data_isolation: DataIsolationLevel::RowLevel,
            access_control: AccessControlModel::Rbac,
            cross_tenant_access: Vec::new(),
        };
        
        let mut policies = self.policies.write();
        policies.insert(tenant_id, policy);
        
        context
    }

    /// Get tenant by ID
    pub fn get_tenant(&self, tenant_id: &str) -> Option<TenantContext> {
        let tenants = self.tenants.read();
        tenants.get(tenant_id).cloned()
    }

    /// Assign execution to tenant
    pub fn assign_execution(&self, execution_id: &str, tenant_id: &str) -> anyhow::Result<()> {
        // Verify tenant exists
        let tenants = self.tenants.read();
        if !tenants.contains_key(tenant_id) {
            return Err(anyhow::anyhow!("Tenant not found: {}", tenant_id));
        }
        drop(tenants);
        
        let mut execution_tenants = self.execution_tenants.write();
        execution_tenants.insert(execution_id.to_string(), tenant_id.to_string());
        
        Ok(())
    }

    /// Get tenant for execution
    pub fn get_execution_tenant(&self, execution_id: &str) -> Option<String> {
        let execution_tenants = self.execution_tenants.read();
        execution_tenants.get(execution_id).cloned()
    }

    /// Check if access is allowed
    pub fn check_access(
        &self,
        accessor_tenant: &str,
        resource_tenant: &str,
        resource_id: &str,
        action: &str,
    ) -> bool {
        // Same tenant always allowed
        if accessor_tenant == resource_tenant {
            return true;
        }
        
        // Check cross-tenant grants
        let policies = self.policies.read();
        if let Some(policy) = policies.get(resource_tenant) {
            for grant in &policy.cross_tenant_access {
                if grant.granted_to == accessor_tenant
                    && grant.resource_id == resource_id
                    && grant.permissions.contains(&action.to_string())
                {
                    // Check expiration
                    if let Some(expires) = grant.expires_at {
                        if Utc::now().timestamp_millis() > expires {
                            return false; // Grant expired
                        }
                    }
                    return true;
                }
            }
        }
        
        false
    }

    /// Grant cross-tenant access
    pub fn grant_cross_tenant_access(
        &self,
        from_tenant: &str,
        to_tenant: &str,
        resource_type: String,
        resource_id: String,
        permissions: Vec<String>,
        expires_at: Option<i64>,
        granted_by: String,
    ) -> anyhow::Result<CrossTenantGrant> {
        let grant = CrossTenantGrant {
            granted_to: to_tenant.to_string(),
            resource_type,
            resource_id,
            permissions,
            granted_at: Utc::now().timestamp_millis(),
            expires_at,
            granted_by,
        };
        
        let mut policies = self.policies.write();
        if let Some(policy) = policies.get_mut(from_tenant) {
            // Check for duplicate grants
            if !policy.cross_tenant_access.iter().any(|g| {
                g.granted_to == to_tenant && g.resource_id == grant.resource_id
            }) {
                policy.cross_tenant_access.push(grant.clone());
            }
        } else {
            return Err(anyhow::anyhow!("Tenant not found: {}", from_tenant));
        }
        
        Ok(grant)
    }

    /// Revoke cross-tenant access
    pub fn revoke_cross_tenant_access(
        &self,
        from_tenant: &str,
        to_tenant: &str,
        resource_id: &str,
    ) -> anyhow::Result<()> {
        let mut policies = self.policies.write();
        if let Some(policy) = policies.get_mut(from_tenant) {
            policy.cross_tenant_access.retain(|g| {
                !(g.granted_to == to_tenant && g.resource_id == resource_id)
            });
            Ok(())
        } else {
            Err(anyhow::anyhow!("Tenant not found: {}", from_tenant))
        }
    }

    /// Update tenant policy
    pub fn update_policy(
        &self,
        tenant_id: &str,
        data_isolation: Option<DataIsolationLevel>,
        access_control: Option<AccessControlModel>,
    ) -> anyhow::Result<TenantPolicy> {
        let mut policies = self.policies.write();
        if let Some(policy) = policies.get_mut(tenant_id) {
            if let Some(level) = data_isolation {
                policy.data_isolation = level;
            }
            if let Some(model) = access_control {
                policy.access_control = model;
            }
            Ok(policy.clone())
        } else {
            Err(anyhow::anyhow!("Tenant not found: {}", tenant_id))
        }
    }

    /// Get tenant policy
    pub fn get_policy(&self, tenant_id: &str) -> Option<TenantPolicy> {
        let policies = self.policies.read();
        policies.get(tenant_id).cloned()
    }

    /// List all tenants
    pub fn list_tenants(&self) -> Vec<TenantContext> {
        let tenants = self.tenants.read();
        tenants.values().cloned().collect()
    }

    /// List executions for tenant
    pub fn list_tenant_executions(&self, tenant_id: &str) -> Vec<String> {
        let execution_tenants = self.execution_tenants.read();
        execution_tenants
            .iter()
            .filter(|(_, tid)| *tid == tenant_id)
            .map(|(eid, _)| eid.clone())
            .collect()
    }
}

impl Default for TenantIsolationManager {
    fn default() -> Self {
        Self::new()
    }
}
