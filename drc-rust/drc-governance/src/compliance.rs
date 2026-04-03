use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Compliance framework
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComplianceFramework {
    Soc2,
    Hipaa,
    Gdpr,
    PciDss,
    Iso27001,
    Fedramp,
}

/// Compliance control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceControl {
    pub control_id: String,
    pub framework: ComplianceFramework,
    pub category: String,
    pub title: String,
    pub description: String,
    pub status: ControlStatus,
    pub implementation: String,
    pub evidence_required: Vec<String>,
    pub last_tested: Option<i64>,
    pub next_test: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlStatus {
    Implemented,
    Partial,
    NotImplemented,
    NotApplicable,
}

/// Compliance evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceEvidence {
    pub evidence_id: String,
    pub control_id: String,
    pub evidence_type: String,
    pub description: String,
    pub collected_at: i64,
    pub collected_by: String,
    pub data: String,
    pub hash: String,
}

/// Compliance finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub finding_id: String,
    pub control_id: String,
    pub severity: FindingSeverity,
    pub title: String,
    pub description: String,
    pub created_at: i64,
    pub status: FindingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingStatus {
    Open,
    InProgress,
    Resolved,
    Accepted,
}

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: String,
    pub framework: ComplianceFramework,
    pub generated_at: i64,
    pub generated_by: String,
    pub period_start: i64,
    pub period_end: i64,
    pub overall_score: f64,
    pub controls: Vec<ComplianceControl>,
    pub findings: Vec<ComplianceFinding>,
    pub evidence: Vec<ComplianceEvidence>,
    pub recommendations: Vec<String>,
}

/// Compliance reporting engine
#[derive(Debug, Clone)]
pub struct ComplianceReportingEngine {
    controls: Arc<RwLock<HashMap<String, ComplianceControl>>>,
    evidence: Arc<RwLock<HashMap<String, Vec<ComplianceEvidence>>>>,
    findings: Arc<RwLock<Vec<ComplianceFinding>>>,
}

impl ComplianceReportingEngine {
    pub fn new() -> Self {
        let engine = Self {
            controls: Arc::new(RwLock::new(HashMap::new())),
            evidence: Arc::new(RwLock::new(HashMap::new())),
            findings: Arc::new(RwLock::new(Vec::new())),
        };
        
        engine.initialize_default_controls();
        engine
    }

    fn initialize_default_controls(&self) {
        let mut controls = self.controls.write();
        
        // SOC2 controls
        let soc2_controls = vec![
            ComplianceControl {
                control_id: "SOC2-CC6.1".to_string(),
                framework: ComplianceFramework::Soc2,
                category: "Logical Access Security".to_string(),
                title: "Audit Logging".to_string(),
                description: "All access to sensitive data must be logged".to_string(),
                status: ControlStatus::Implemented,
                implementation: "ImmutableAuditLog captures all access".to_string(),
                evidence_required: vec!["audit_logs".to_string()],
                last_tested: None,
                next_test: None,
            },
            ComplianceControl {
                control_id: "SOC2-CC6.2".to_string(),
                framework: ComplianceFramework::Soc2,
                category: "Logical Access Security".to_string(),
                title: "Access Controls".to_string(),
                description: "Access to systems must be restricted".to_string(),
                status: ControlStatus::Implemented,
                implementation: "RBAC with tenant isolation".to_string(),
                evidence_required: vec!["access_policies".to_string()],
                last_tested: None,
                next_test: None,
            },
            ComplianceControl {
                control_id: "SOC2-CC6.3".to_string(),
                framework: ComplianceFramework::Soc2,
                category: "System Operations".to_string(),
                title: "Data Backup".to_string(),
                description: "Data must be backed up regularly".to_string(),
                status: ControlStatus::Implemented,
                implementation: "Tiered storage with snapshots".to_string(),
                evidence_required: vec!["backup_logs".to_string()],
                last_tested: None,
                next_test: None,
            },
        ];
        
        for control in soc2_controls {
            controls.insert(control.control_id.clone(), control);
        }
        
        // GDPR controls
        let gdpr_controls = vec![
            ComplianceControl {
                control_id: "GDPR-Art17".to_string(),
                framework: ComplianceFramework::Gdpr,
                category: "Data Subject Rights".to_string(),
                title: "Right to Erasure".to_string(),
                description: "Data subjects can request deletion".to_string(),
                status: ControlStatus::Implemented,
                implementation: "RTBF Manager handles deletion requests".to_string(),
                evidence_required: vec!["deletion_certificates".to_string()],
                last_tested: None,
                next_test: None,
            },
            ComplianceControl {
                control_id: "GDPR-Art32".to_string(),
                framework: ComplianceFramework::Gdpr,
                category: "Security".to_string(),
                title: "Data Protection".to_string(),
                description: "Appropriate security measures".to_string(),
                status: ControlStatus::Implemented,
                implementation: "KMS encryption and classification".to_string(),
                evidence_required: vec!["encryption_settings".to_string()],
                last_tested: None,
                next_test: None,
            },
        ];
        
        for control in gdpr_controls {
            controls.insert(control.control_id.clone(), control);
        }
        
        // HIPAA controls
        let hipaa_controls = vec![
            ComplianceControl {
                control_id: "HIPAA-164.312".to_string(),
                framework: ComplianceFramework::Hipaa,
                category: "Technical Safeguards".to_string(),
                title: "Access Control".to_string(),
                description: "Implement technical policies for access".to_string(),
                status: ControlStatus::Implemented,
                implementation: "Role-based access control".to_string(),
                evidence_required: vec!["access_logs".to_string()],
                last_tested: None,
                next_test: None,
            },
        ];
        
        for control in hipaa_controls {
            controls.insert(control.control_id.clone(), control);
        }
    }

    /// Add a compliance control
    pub fn add_control(&self, control: ComplianceControl) {
        let mut controls = self.controls.write();
        controls.insert(control.control_id.clone(), control);
    }

    /// Get control by ID
    pub fn get_control(&self, control_id: &str) -> Option<ComplianceControl> {
        let controls = self.controls.read();
        controls.get(control_id).cloned()
    }

    /// Collect evidence for a control
    pub fn collect_evidence(
        &self,
        control_id: &str,
        evidence_type: String,
        description: String,
        collected_by: String,
        data: String,
        hash: String,
    ) -> ComplianceEvidence {
        let evidence = ComplianceEvidence {
            evidence_id: format!("evidence_{}", Utc::now().timestamp_millis()),
            control_id: control_id.to_string(),
            evidence_type,
            description,
            collected_at: Utc::now().timestamp_millis(),
            collected_by,
            data,
            hash,
        };
        
        let mut evidence_map = self.evidence.write();
        evidence_map
            .entry(control_id.to_string())
            .or_default()
            .push(evidence.clone());
        
        evidence
    }

    /// Record a finding
    pub fn record_finding(
        &self,
        control_id: String,
        severity: FindingSeverity,
        title: String,
        description: String,
        remediation: Option<String>,
    ) -> ComplianceFinding {
        let finding = ComplianceFinding {
            finding_id: format!("finding_{}", Utc::now().timestamp_millis()),
            control_id,
            severity,
            title,
            description,
            created_at: Utc::now().timestamp_millis(),
            status: FindingStatus::Open,
            remediation,
            resolved_at: None,
        };
        
        let mut findings = self.findings.write();
        findings.push(finding.clone());
        
        finding
    }

    /// Resolve a finding
    pub fn resolve_finding(&self, finding_id: &str) -> anyhow::Result<()> {
        let mut findings = self.findings.write();
        
        if let Some(finding) = findings.iter_mut().find(|f| f.finding_id == finding_id) {
            finding.status = FindingStatus::Resolved;
            finding.resolved_at = Some(Utc::now().timestamp_millis());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Finding not found: {}", finding_id))
        }
    }

    /// Generate compliance report
    pub fn generate_report(
        &self,
        framework: ComplianceFramework,
        generated_by: String,
        period_start: i64,
        period_end: i64,
    ) -> ComplianceReport {
        let controls = self.controls.read();
        let findings = self.findings.read();
        let evidence_map = self.evidence.read();
        
        // Filter controls for framework
        let framework_controls: Vec<_> = controls
            .values()
            .filter(|c| c.framework == framework)
            .cloned()
            .collect();
        
        // Get findings for these controls
        let control_ids: std::collections::HashSet<_> = framework_controls
            .iter()
            .map(|c| c.control_id.clone())
            .collect();
        
        let framework_findings: Vec<_> = findings
            .iter()
            .filter(|f| control_ids.contains(&f.control_id))
            .cloned()
            .collect();
        
        // Collect evidence
        let mut all_evidence = Vec::new();
        for (control_id, evidence_list) in evidence_map.iter() {
            if control_ids.contains(control_id) {
                all_evidence.extend(evidence_list.clone());
            }
        }
        
        // Calculate score
        let total_controls = framework_controls.len() as f64;
        let implemented = framework_controls
            .iter()
            .filter(|c| c.status == ControlStatus::Implemented)
            .count() as f64;
        let score = if total_controls > 0.0 {
            (implemented / total_controls) * 100.0
        } else {
            0.0
        };
        
        // Generate recommendations
        let recommendations = framework_findings
            .iter()
            .filter(|f| f.status == FindingStatus::Open)
            .map(|f| format!("{}: {}", f.control_id, f.title))
            .collect();
        
        ComplianceReport {
            report_id: format!("report_{}", Utc::now().timestamp_millis()),
            framework,
            generated_at: Utc::now().timestamp_millis(),
            generated_by,
            period_start,
            period_end,
            overall_score: score,
            controls: framework_controls,
            findings: framework_findings,
            evidence: all_evidence,
            recommendations,
        }
    }

    /// Get controls by framework
    pub fn get_controls_by_framework(&self, framework: ComplianceFramework) -> Vec<ComplianceControl> {
        let controls = self.controls.read();
        controls
            .values()
            .filter(|c| c.framework == framework)
            .cloned()
            .collect()
    }

    /// Get findings by severity
    pub fn get_findings_by_severity(&self, severity: FindingSeverity) -> Vec<ComplianceFinding> {
        let findings = self.findings.read();
        findings
            .iter()
            .filter(|f| f.severity == severity)
            .cloned()
            .collect()
    }

    /// Get open findings count
    pub fn get_open_findings_count(&self) -> usize {
        let findings = self.findings.read();
        findings
            .iter()
            .filter(|f| f.status == FindingStatus::Open)
            .count()
    }

    /// Update control status
    pub fn update_control_status(
        &self,
        control_id: &str,
        status: ControlStatus,
        implementation: Option<String>,
    ) -> anyhow::Result<()> {
        let mut controls = self.controls.write();
        
        if let Some(control) = controls.get_mut(control_id) {
            control.status = status;
            if let Some(impl_text) = implementation {
                control.implementation = impl_text;
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Control not found: {}", control_id))
        }
    }

    /// List all findings
    pub fn list_findings(&self) -> Vec<ComplianceFinding> {
        let findings = self.findings.read();
        findings.clone()
    }
}

impl Default for ComplianceReportingEngine {
    fn default() -> Self {
        Self::new()
    }
}
