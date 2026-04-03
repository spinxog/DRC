use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::info;
use drc_core::{DRCEvent, ExecutionId, ExecutionMetadata, Storage};
pub use drc_core::Storage as StorageTrait;
pub use drc_core::types::StorageTier;

/// File-based storage implementation
#[derive(Debug, Clone)]
pub struct FileStorage {
    base_path: PathBuf,
    cache: Arc<RwLock<HashMap<ExecutionId, Vec<DRCEvent>>>>,
}

impl FileStorage {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn ensure_dir(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.base_path).await?;
        Ok(())
    }

    fn get_event_path(&self, execution_id: &ExecutionId) -> PathBuf {
        self.base_path.join(format!("{}.events.json", execution_id))
    }

    fn get_metadata_path(&self, execution_id: &ExecutionId) -> PathBuf {
        self.base_path.join(format!("{}.meta.json", execution_id))
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn store_event(&self, event: &DRCEvent) -> anyhow::Result<()> {
        self.ensure_dir().await?;
        
        let path = self.get_event_path(&event.execution_id);
        
        // Read existing events
        let mut events: Vec<DRCEvent> = if path.exists() {
            let content = fs::read_to_string(&path).await?;
            serde_json::from_str(&content)?
        } else {
            Vec::new()
        };
        
        // Add new event
        events.push(event.clone());
        
        // Write back
        let json = serde_json::to_string_pretty(&events)?;
        fs::write(&path, json).await?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(event.execution_id.clone(), events);
        
        Ok(())
    }

    async fn get_events(&self, execution_id: &ExecutionId) -> anyhow::Result<Vec<DRCEvent>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(events) = cache.get(execution_id) {
                return Ok(events.clone());
            }
        }
        
        // Read from disk
        let path = self.get_event_path(execution_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        
        let content = fs::read_to_string(&path).await?;
        let events: Vec<DRCEvent> = serde_json::from_str(&content)?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(execution_id.clone(), events.clone());
        
        Ok(events)
    }

    async fn store_metadata(&self, metadata: &ExecutionMetadata) -> anyhow::Result<()> {
        self.ensure_dir().await?;
        
        let path = self.get_metadata_path(&metadata.execution_id);
        let json = serde_json::to_string_pretty(metadata)?;
        fs::write(&path, json).await?;
        
        Ok(())
    }

    async fn get_metadata(&self, execution_id: &ExecutionId) -> anyhow::Result<Option<ExecutionMetadata>> {
        let path = self.get_metadata_path(execution_id);
        if !path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(&path).await?;
        let metadata: ExecutionMetadata = serde_json::from_str(&content)?;
        
        Ok(Some(metadata))
    }
}

/// Tiered storage with hot/warm/cold tiers
#[derive(Debug, Clone)]
pub struct TieredStorage {
    hot: FileStorage,
    warm: FileStorage,
    cold: FileStorage,
    current_tier: Arc<RwLock<StorageTier>>,
}

impl TieredStorage {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let base = base_path.as_ref();
        Self {
            hot: FileStorage::new(base.join("hot")),
            warm: FileStorage::new(base.join("warm")),
            cold: FileStorage::new(base.join("cold")),
            current_tier: Arc::new(RwLock::new(StorageTier::Hot)),
        }
    }

    async fn get_storage_for_tier(&self, tier: StorageTier) -> &FileStorage {
        match tier {
            StorageTier::Hot => &self.hot,
            StorageTier::Warm => &self.warm,
            StorageTier::Cold => &self.cold,
            StorageTier::Expired => &self.cold,
        }
    }

    pub async fn promote(&self, execution_id: &ExecutionId, to_tier: StorageTier) -> anyhow::Result<()> {
        // Find which tier currently has the data
        let from_tier = self.find_execution_tier(execution_id).await;
        
        if from_tier == to_tier {
            return Ok(());
        }
        
        // Get events from source tier
        let from_storage = self.get_storage_for_tier(from_tier).await;
        let events = from_storage.get_events(execution_id).await?;
        let metadata = from_storage.get_metadata(execution_id).await?;
        
        // Store in new tier
        let to_storage = self.get_storage_for_tier(to_tier).await;
        if let Some(meta) = metadata {
            to_storage.store_metadata(&meta).await?;
        }
        for event in &events {
            to_storage.store_event(event).await?;
        }
        
        // Update current tier tracking
        let mut current = self.current_tier.write().await;
        *current = to_tier;
        
        info!(
            "Promoted execution {} from {:?} to {:?}",
            execution_id, from_tier, to_tier
        );
        
        Ok(())
    }
    
    /// Find which tier currently contains the execution data
    async fn find_execution_tier(&self, execution_id: &ExecutionId) -> StorageTier {
        for tier in [StorageTier::Hot, StorageTier::Warm, StorageTier::Cold] {
            let storage = self.get_storage_for_tier(tier).await;
            if let Ok(events) = storage.get_events(execution_id).await {
                if !events.is_empty() {
                    return tier;
                }
            }
        }
        // Default to current tier if not found
        *self.current_tier.read().await
    }
}

#[async_trait]
impl Storage for TieredStorage {
    async fn store_event(&self, event: &DRCEvent) -> anyhow::Result<()> {
        let tier = {
            let current = self.current_tier.read().await;
            *current
        };
        let storage = self.get_storage_for_tier(tier).await;
        storage.store_event(event).await
    }

    async fn get_events(&self, execution_id: &ExecutionId) -> anyhow::Result<Vec<DRCEvent>> {
        // Try each tier in order
        for tier in [StorageTier::Hot, StorageTier::Warm, StorageTier::Cold] {
            let storage = self.get_storage_for_tier(tier).await;
            let events = storage.get_events(execution_id).await?;
            if !events.is_empty() {
                return Ok(events);
            }
        }
        Ok(Vec::new())
    }

    async fn store_metadata(&self, metadata: &ExecutionMetadata) -> anyhow::Result<()> {
        let tier = {
            let current = self.current_tier.read().await;
            *current
        };
        let storage = self.get_storage_for_tier(tier).await;
        storage.store_metadata(metadata).await
    }

    async fn get_metadata(&self, execution_id: &ExecutionId) -> anyhow::Result<Option<ExecutionMetadata>> {
        // Try each tier in order
        for tier in [StorageTier::Hot, StorageTier::Warm, StorageTier::Cold] {
            let storage = self.get_storage_for_tier(tier).await;
            let metadata = storage.get_metadata(execution_id).await?;
            if metadata.is_some() {
                return Ok(metadata);
            }
        }
        Ok(None)
    }
}
