use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use regex::Regex;

/// Data classification level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClassificationLevel {
    Public,
    Internal,
    Confidential,
    Restricted,
    Critical,
}

/// Sensitive data type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SensitiveDataType {
    Pii,
    Pci,
    Phi,
    Financial,
    Credentials,
    Secrets,
    IntellectualProperty,
    Personal,
}

/// Classification rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRule {
    pub name: String,
    pub pattern: String,
    pub level: ClassificationLevel,
    pub data_types: Vec<SensitiveDataType>,
    pub auto_redact: bool,
    pub auto_encrypt: bool,
}

/// Classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub level: ClassificationLevel,
    pub sensitive_types: Vec<SensitiveDataType>,
    pub fields_identified: Vec<String>,
    pub compliance_flags: Vec<String>,
    pub confidence_score: f64,
    pub recommendation: String,
}

/// Data classification engine
#[derive(Debug, Clone)]
pub struct DataClassificationEngine {
    rules: Arc<RwLock<Vec<ClassificationRule>>>,
    compiled_patterns: Arc<RwLock<HashMap<String, Regex>>>,
}

impl DataClassificationEngine {
    pub fn new() -> Self {
        let engine = Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            compiled_patterns: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Initialize with default rules
        engine.initialize_default_rules();
        
        engine
    }

    fn initialize_default_rules(&self) {
        let default_rules = vec![
            ClassificationRule {
                name: "PII Detection".to_string(),
                pattern: r"\b[A-Z]{2}\d{6,10}\b".to_string(), // Passport pattern
                level: ClassificationLevel::Confidential,
                data_types: vec![SensitiveDataType::Pii],
                auto_redact: true,
                auto_encrypt: false,
            },
            ClassificationRule {
                name: "Credit Card".to_string(),
                pattern: r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b".to_string(),
                level: ClassificationLevel::Restricted,
                data_types: vec![SensitiveDataType::Pci, SensitiveDataType::Financial],
                auto_redact: true,
                auto_encrypt: true,
            },
            ClassificationRule {
                name: "SSN".to_string(),
                pattern: r"\b\d{3}-\d{2}-\d{4}\b".to_string(),
                level: ClassificationLevel::Restricted,
                data_types: vec![SensitiveDataType::Pii],
                auto_redact: true,
                auto_encrypt: true,
            },
            ClassificationRule {
                name: "Email".to_string(),
                pattern: r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b".to_string(),
                level: ClassificationLevel::Confidential,
                data_types: vec![SensitiveDataType::Pii, SensitiveDataType::Personal],
                auto_redact: true,
                auto_encrypt: false,
            },
            ClassificationRule {
                name: "API Key".to_string(),
                pattern: "(?i)(api[_-]?key|apikey)\\s*[:=]\\s*['\"]?[a-zA-Z0-9]{16,}['\"]?".to_string(),
                level: ClassificationLevel::Critical,
                data_types: vec![SensitiveDataType::Credentials, SensitiveDataType::Secrets],
                auto_redact: true,
                auto_encrypt: true,
            },
            ClassificationRule {
                name: "Password".to_string(),
                pattern: "(?i)(password|passwd|pwd)\\s*[:=]\\s*['\"]?[^\\s'\"]+['\"]?".to_string(),
                level: ClassificationLevel::Critical,
                data_types: vec![SensitiveDataType::Credentials, SensitiveDataType::Secrets],
                auto_redact: true,
                auto_encrypt: true,
            },
            ClassificationRule {
                name: "JWT Token".to_string(),
                pattern: r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*".to_string(),
                level: ClassificationLevel::Critical,
                data_types: vec![SensitiveDataType::Credentials, SensitiveDataType::Secrets],
                auto_redact: true,
                auto_encrypt: true,
            },
        ];
        
        let mut rules = self.rules.write();
        *rules = default_rules.clone();
        
        // Compile patterns
        let mut patterns = self.compiled_patterns.write();
        for rule in default_rules {
            if let Ok(regex) = Regex::new(&rule.pattern) {
                patterns.insert(rule.name.clone(), regex);
            }
        }
    }

    /// Add a custom classification rule
    pub fn add_rule(&self, rule: ClassificationRule) -> anyhow::Result<()> {
        // Compile and validate pattern
        let regex = Regex::new(&rule.pattern)
            .map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))?;
        
        let mut rules = self.rules.write();
        rules.push(rule.clone());
        
        let mut patterns = self.compiled_patterns.write();
        patterns.insert(rule.name, regex);
        
        Ok(())
    }

    /// Classify data
    pub fn classify(&self, data: &str) -> ClassificationResult {
        let rules = self.rules.read();
        let patterns = self.compiled_patterns.read();
        
        let mut detected_types = HashSet::new();
        let mut fields = Vec::new();
        let mut max_level = ClassificationLevel::Public;
        let mut compliance_flags = Vec::new();
        
        for rule in rules.iter() {
            if let Some(regex) = patterns.get(&rule.name) {
                for mat in regex.find_iter(data) {
                    // Update classification level
                    if self.level_value(&rule.level) > self.level_value(&max_level) {
                        max_level = rule.level;
                    }
                    
                    // Add sensitive types
                    for data_type in &rule.data_types {
                        detected_types.insert(*data_type);
                    }
                    
                    // Add field
                    let field_snippet = mat.as_str().chars().take(20).collect::<String>();
                    fields.push(format!("{}: {}", rule.name, field_snippet));
                    
                    // Add compliance flags
                    if rule.auto_encrypt {
                        compliance_flags.push(format!("ENCRYPTION_REQUIRED: {}", rule.name));
                    }
                    if rule.auto_redact {
                        compliance_flags.push(format!("REDACTION_RECOMMENDED: {}", rule.name));
                    }
                }
            }
        }
        
        // Calculate confidence score
        let confidence = if detected_types.is_empty() {
            1.0
        } else {
            0.5 + (detected_types.len() as f64 * 0.1).min(0.5)
        };
        
        // Generate recommendation
        let recommendation = if detected_types.is_empty() {
            "No sensitive data detected".to_string()
        } else {
            format!(
                "Sensitive data detected: {}. Apply appropriate protection measures.",
                detected_types.iter()
                    .map(|t| format!("{:?}", t))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        
        ClassificationResult {
            level: max_level,
            sensitive_types: detected_types.into_iter().collect(),
            fields_identified: fields,
            compliance_flags,
            confidence_score: confidence,
            recommendation,
        }
    }

    /// Classify an event payload
    pub fn classify_payload(&self, payload: &serde_json::Value) -> ClassificationResult {
        let data = payload.to_string();
        self.classify(&data)
    }

    /// Get classification level value for ordering
    fn level_value(&self, level: &ClassificationLevel) -> u8 {
        match level {
            ClassificationLevel::Public => 0,
            ClassificationLevel::Internal => 1,
            ClassificationLevel::Confidential => 2,
            ClassificationLevel::Restricted => 3,
            ClassificationLevel::Critical => 4,
        }
    }

    /// Check if redaction is required
    pub fn requires_redaction(&self, result: &ClassificationResult) -> bool {
        matches!(
            result.level,
            ClassificationLevel::Restricted | ClassificationLevel::Critical
        ) || !result.sensitive_types.is_empty()
    }

    /// Check if encryption is required
    pub fn requires_encryption(&self, result: &ClassificationResult) -> bool {
        matches!(
            result.level,
            ClassificationLevel::Confidential | ClassificationLevel::Restricted | ClassificationLevel::Critical
        ) || result.sensitive_types.iter().any(|t| {
            matches!(
                t,
                SensitiveDataType::Pci | SensitiveDataType::Credentials | SensitiveDataType::Secrets
            )
        })
    }

    /// Get all rules
    pub fn get_rules(&self) -> Vec<ClassificationRule> {
        let rules = self.rules.read();
        rules.clone()
    }

    /// Remove a rule
    pub fn remove_rule(&self, rule_name: &str) -> anyhow::Result<()> {
        let mut rules = self.rules.write();
        let initial_len = rules.len();
        rules.retain(|r| r.name != rule_name);
        
        if rules.len() == initial_len {
            return Err(anyhow::anyhow!("Rule not found: {}", rule_name));
        }
        
        let mut patterns = self.compiled_patterns.write();
        patterns.remove(rule_name);
        
        Ok(())
    }
}

impl Default for DataClassificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Auto-classification policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoClassificationPolicy {
    pub policy_id: String,
    pub name: String,
    pub rules: Vec<ClassificationRule>,
    pub auto_redact_threshold: ClassificationLevel,
    pub auto_encrypt_threshold: ClassificationLevel,
    pub retention_policy: String,
}

impl AutoClassificationPolicy {
    pub fn new(name: String) -> Self {
        Self {
            policy_id: format!("policy_{}", uuid::Uuid::new_v4()),
            name,
            rules: Vec::new(),
            auto_redact_threshold: ClassificationLevel::Restricted,
            auto_encrypt_threshold: ClassificationLevel::Confidential,
            retention_policy: "standard".to_string(),
        }
    }

    pub fn add_rule(&mut self, rule: ClassificationRule) {
        self.rules.push(rule);
    }
}
