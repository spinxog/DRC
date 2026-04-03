use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Database adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DBAdapterConfig {
    pub capture_queries: bool,
    pub capture_results: bool,
    pub capture_transactions: bool,
    pub max_result_size: usize,
    pub redact_sensitive_fields: Vec<String>,
}

impl Default for DBAdapterConfig {
    fn default() -> Self {
        Self {
            capture_queries: true,
            capture_results: true,
            capture_transactions: true,
            max_result_size: 1000,
            redact_sensitive_fields: vec![
                "password".to_string(),
                "secret".to_string(),
                "token".to_string(),
                "credit_card".to_string(),
            ],
        }
    }
}

/// Captured database query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedQuery {
    pub query_id: String,
    pub timestamp: i64,
    pub execution_id: String,
    pub database: String,
    pub query: String,
    pub parameters: Vec<String>,
    pub duration_ms: u64,
    pub row_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Captured query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedResult {
    pub query_id: String,
    pub timestamp: i64,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    pub fields: Vec<String>,
    pub truncated: bool,
}

/// Transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub transaction_id: String,
    pub execution_id: String,
    pub status: TransactionStatus,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committed_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolled_back_at: Option<i64>,
    pub queries: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatus {
    Active,
    Committed,
    RolledBack,
}

/// Base database adapter
#[derive(Debug, Clone)]
pub struct DatabaseAdapter {
    config: DBAdapterConfig,
    query_sequence: Arc<RwLock<u64>>,
    active_transactions: Arc<RwLock<HashMap<String, TransactionRecord>>>,
}

impl DatabaseAdapter {
    pub fn new(config: Option<DBAdapterConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            query_sequence: Arc::new(RwLock::new(0)),
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate next query ID
    pub async fn next_query_id(&self) -> String {
        let mut seq = self.query_sequence.write().await;
        *seq += 1;
        format!("query_{}", *seq)
    }

    /// Redact sensitive parameters
    pub fn redact_parameters(&self, params: &[String]) -> Vec<String> {
        params
            .iter()
            .map(|param| {
                let lower = param.to_lowercase();
                for field in &self.config.redact_sensitive_fields {
                    if lower.contains(field) {
                        return "[REDACTED]".to_string();
                    }
                }
                param.clone()
            })
            .collect()
    }

    /// Truncate results
    pub fn truncate_results(&self, rows: Vec<HashMap<String, serde_json::Value>>) -> (Vec<HashMap<String, serde_json::Value>>, bool) {
        if rows.len() <= self.config.max_result_size {
            (rows, false)
        } else {
            (rows.into_iter().take(self.config.max_result_size).collect(), true)
        }
    }

    /// Start a transaction
    pub async fn start_transaction(&self, execution_id: &str) -> TransactionRecord {
        let tx_id = format!("tx_{}", chrono::Utc::now().timestamp_millis());
        let record = TransactionRecord {
            transaction_id: tx_id.clone(),
            execution_id: execution_id.to_string(),
            status: TransactionStatus::Active,
            started_at: chrono::Utc::now().timestamp_millis(),
            committed_at: None,
            rolled_back_at: None,
            queries: Vec::new(),
        };

        let mut txs = self.active_transactions.write().await;
        txs.insert(tx_id, record.clone());

        record
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, tx_id: &str) -> anyhow::Result<()> {
        let mut txs = self.active_transactions.write().await;
        if let Some(tx) = txs.get_mut(tx_id) {
            tx.status = TransactionStatus::Committed;
            tx.committed_at = Some(chrono::Utc::now().timestamp_millis());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction not found: {}", tx_id))
        }
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(&self, tx_id: &str) -> anyhow::Result<()> {
        let mut txs = self.active_transactions.write().await;
        if let Some(tx) = txs.get_mut(tx_id) {
            tx.status = TransactionStatus::RolledBack;
            tx.rolled_back_at = Some(chrono::Utc::now().timestamp_millis());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction not found: {}", tx_id))
        }
    }
}

/// PostgreSQL adapter
#[derive(Debug, Clone)]
pub struct PostgreSQLAdapter {
    base: DatabaseAdapter,
    client_id: String,
}

impl PostgreSQLAdapter {
    pub fn new(config: Option<DBAdapterConfig>) -> Self {
        Self {
            base: DatabaseAdapter::new(config),
            client_id: format!("pg_{}", uuid::Uuid::new_v4()),
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn base(&self) -> &DatabaseAdapter {
        &self.base
    }
}

/// MySQL adapter
#[derive(Debug, Clone)]
pub struct MySQLAdapter {
    base: DatabaseAdapter,
    connection_id: String,
}

impl MySQLAdapter {
    pub fn new(config: Option<DBAdapterConfig>) -> Self {
        Self {
            base: DatabaseAdapter::new(config),
            connection_id: format!("mysql_{}", uuid::Uuid::new_v4()),
        }
    }

    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    pub fn base(&self) -> &DatabaseAdapter {
        &self.base
    }
}

/// Redis adapter
#[derive(Debug, Clone)]
pub struct RedisAdapter {
    base: DatabaseAdapter,
    command_sequence: Arc<RwLock<u64>>,
}

impl RedisAdapter {
    pub fn new(config: Option<DBAdapterConfig>) -> Self {
        Self {
            base: DatabaseAdapter::new(config),
            command_sequence: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn next_command_id(&self) -> String {
        let mut seq = self.command_sequence.write().await;
        *seq += 1;
        format!("redis_cmd_{}", *seq)
    }

    pub fn base(&self) -> &DatabaseAdapter {
        &self.base
    }
}

/// MongoDB adapter
#[derive(Debug, Clone)]
pub struct MongoDBAdapter {
    base: DatabaseAdapter,
    operation_sequence: Arc<RwLock<u64>>,
}

impl MongoDBAdapter {
    pub fn new(config: Option<DBAdapterConfig>) -> Self {
        Self {
            base: DatabaseAdapter::new(config),
            operation_sequence: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn next_operation_id(&self) -> String {
        let mut seq = self.operation_sequence.write().await;
        *seq += 1;
        format!("mongo_op_{}", *seq)
    }

    pub fn base(&self) -> &DatabaseAdapter {
        &self.base
    }
}

/// Database adapter manager
#[derive(Debug, Clone)]
pub struct DBAdapterManager {
    adapters: Arc<RwLock<HashMap<String, AdapterWrapper>>>,
}

#[derive(Debug, Clone)]
pub enum AdapterWrapper {
    PostgreSQL(PostgreSQLAdapter),
    MySQL(MySQLAdapter),
    Redis(RedisAdapter),
    MongoDB(MongoDBAdapter),
}

impl DBAdapterManager {
    pub fn new() -> Self {
        Self {
            adapters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_postgres_adapter(
        &self,
        name: &str,
        config: Option<DBAdapterConfig>,
    ) -> PostgreSQLAdapter {
        let adapter = PostgreSQLAdapter::new(config);
        let mut adapters = self.adapters.write().await;
        adapters.insert(
            name.to_string(),
            AdapterWrapper::PostgreSQL(adapter.clone()),
        );
        adapter
    }

    pub async fn create_mysql_adapter(
        &self,
        name: &str,
        config: Option<DBAdapterConfig>,
    ) -> MySQLAdapter {
        let adapter = MySQLAdapter::new(config);
        let mut adapters = self.adapters.write().await;
        adapters.insert(
            name.to_string(),
            AdapterWrapper::MySQL(adapter.clone()),
        );
        adapter
    }

    pub async fn create_redis_adapter(
        &self,
        name: &str,
        config: Option<DBAdapterConfig>,
    ) -> RedisAdapter {
        let adapter = RedisAdapter::new(config);
        let mut adapters = self.adapters.write().await;
        adapters.insert(
            name.to_string(),
            AdapterWrapper::Redis(adapter.clone()),
        );
        adapter
    }

    pub async fn create_mongodb_adapter(
        &self,
        name: &str,
        config: Option<DBAdapterConfig>,
    ) -> MongoDBAdapter {
        let adapter = MongoDBAdapter::new(config);
        let mut adapters = self.adapters.write().await;
        adapters.insert(
            name.to_string(),
            AdapterWrapper::MongoDB(adapter.clone()),
        );
        adapter
    }

    pub async fn get_adapter(&self, name: &str) -> Option<AdapterWrapper> {
        let adapters = self.adapters.read().await;
        adapters.get(name).cloned()
    }

    pub async fn remove_adapter(&self, name: &str) {
        let mut adapters = self.adapters.write().await;
        adapters.remove(name);
    }
}

impl Default for DBAdapterManager {
    fn default() -> Self {
        Self::new()
    }
}
