use std::collections::HashMap;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::sleep;
use serde::{Deserialize, Serialize};
use tracing::info;
use std::sync::Arc;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub sandbox_type: SandboxType,
    pub image: String,
    pub cpu_limit: String,       // e.g., "1.0" for 1 core
    pub memory_limit: String,    // e.g., "512m" for 512MB
    pub disk_limit: String,      // e.g., "1g" for 1GB
    pub network_mode: NetworkMode,
    pub egress_policy: EgressPolicy,
    pub volume_mounts: Vec<VolumeMount>,
    pub environment: HashMap<String, String>,
    pub timeout_seconds: u64,
    pub cleanup_on_exit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SandboxType {
    Docker,
    Kubernetes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetworkMode {
    None,
    Host,
    Bridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EgressPolicy {
    Block,
    Restricted,
    Allow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

/// Sandbox status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxStatus {
    pub id: String,
    pub execution_id: String,
    pub state: SandboxState,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub exit_code: Option<i32>,
    pub metrics: SandboxMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SandboxState {
    Creating,
    Running,
    Completed,
    Failed,
    Timeout,
    Destroyed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub disk_usage_mb: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_recv: u64,
}

/// Docker sandbox
#[derive(Debug)]
pub struct DockerSandbox {
    container_id: Option<String>,
    _config: SandboxConfig,
    status: Arc<RwLock<SandboxStatus>>,
    demo_mode: bool,
}

impl DockerSandbox {
    pub fn new(execution_id: &str, config: SandboxConfig, demo_mode: bool) -> Self {
        let status = SandboxStatus {
            id: format!("sandbox_{}", execution_id),
            execution_id: execution_id.to_string(),
            state: SandboxState::Creating,
            created_at: chrono::Utc::now().timestamp_millis(),
            started_at: None,
            completed_at: None,
            exit_code: None,
            metrics: SandboxMetrics::default(),
        };

        Self {
            container_id: None,
            _config: config,
            status: Arc::new(RwLock::new(status)),
            demo_mode,
        }
    }

    /// Create and start the container
    pub async fn create(&mut self, working_dir: &str) -> anyhow::Result<SandboxStatus> {
        // In demo mode, simulate container creation without Docker
        if self.demo_mode {
            let sandbox_id = format!("drc-demo-{}", self.status.read().await.execution_id);
            
            // Simulate container ID
            self.container_id = Some(format!("demo_{}", chrono::Utc::now().timestamp_millis()));
            
            // Update status to running
            {
                let mut status = self.status.write().await;
                status.state = SandboxState::Running;
                status.started_at = Some(chrono::Utc::now().timestamp_millis());
                status.metrics.cpu_usage_percent = 0.5;
                status.metrics.memory_usage_mb = 100; // 100MB
            }

            info!("Docker sandbox {} created (demo mode)", sandbox_id);
            return Ok(self.status.read().await.clone());
        }

        // Real Docker mode - validate docker binary exists
        match Command::new("docker").arg("--version").output().await {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("Docker is not installed or not in PATH")),
        }

        let sandbox_id = format!("drc-{}", self.status.read().await.execution_id);
        
        // Build docker run command
        let mut args = vec![
            "run".to_string(),
            "-d".to_string(), // detached
            "--name".to_string(),
            sandbox_id.clone(),
            "--cpus".to_string(),
            self._config.cpu_limit.clone(),
            "--memory".to_string(),
            self._config.memory_limit.clone(),
        ];

        // Network configuration
        match self._config.network_mode {
            NetworkMode::None => args.push("--network=none".to_string()),
            NetworkMode::Host => args.push("--network=host".to_string()),
            NetworkMode::Bridge => args.push("--network=bridge".to_string()),
        }

        // Volume mounts
        for mount in &self._config.volume_mounts {
            let mount_str = format!(
                "{}:{}:{}",
                mount.host_path,
                mount.container_path,
                if mount.read_only { "ro" } else { "rw" }
            );
            args.push("-v".to_string());
            args.push(mount_str);
        }

        // Environment variables
        for (key, value) in &self._config.environment {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Add image
        args.push(self._config.image.clone());

        // Execute docker run
        let output = Command::new("docker")
            .args(&args)
            .current_dir(working_dir)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to create container: {}", stderr));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        self.container_id = Some(container_id);

        // Update status
        {
            let mut status = self.status.write().await;
            status.state = SandboxState::Running;
            status.started_at = Some(chrono::Utc::now().timestamp_millis());
        }

        info!("Docker sandbox {} created and running", sandbox_id);

        // Start monitoring
        let status_clone = self.status.clone();
        let container_id_clone = self.container_id.clone();
        let timeout_seconds = self._config.timeout_seconds;

        tokio::spawn(async move {
            DockerSandbox::monitor_container(status_clone, container_id_clone, timeout_seconds).await;
        });

        Ok(self.status.read().await.clone())
    }

    /// Execute a command in the container
    pub async fn execute(&self, command: &[String]) -> anyhow::Result<SandboxResult> {
        let container_id = self
            .container_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Container not created"))?;

        let mut args = vec!["exec".to_string(), container_id.clone()];
        args.extend(command.iter().cloned());

        let output = Command::new("docker")
            .args(&args)
            .output()
            .await?;

        Ok(SandboxResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms: 0, // Would track actual duration
        })
    }

    /// Stop and remove the container
    pub async fn destroy(&mut self) -> anyhow::Result<()> {
        if let Some(ref container_id) = self.container_id {
            // Stop container
            let _ = Command::new("docker")
                .args(&["stop", "-t", "10", container_id])
                .output()
                .await;

            // Remove container
            let _ = Command::new("docker")
                .args(&["rm", "-f", container_id])
                .output()
                .await;

            let mut status = self.status.write().await;
            status.state = SandboxState::Destroyed;
            status.completed_at = Some(chrono::Utc::now().timestamp_millis());

            info!("Docker sandbox {} destroyed", container_id);
        }

        Ok(())
    }

    /// Monitor container status
    async fn monitor_container(
        status: Arc<RwLock<SandboxStatus>>,
        container_id: Option<String>,
        timeout_seconds: u64,
    ) {
        let container_id = match container_id {
            Some(id) => id,
            None => return,
        };
        let start_time = tokio::time::Instant::now();
        let timeout_duration = Duration::from_secs(timeout_seconds);

        loop {
            // Check timeout
            if start_time.elapsed() > timeout_duration {
                let mut s = status.write().await;
                s.state = SandboxState::Timeout;
                break;
            }

            // Check container status
            let output = Command::new("docker")
                .args(&["inspect", &container_id, "--format", "{{.State.Status}}"])
                .output()
                .await;

            match output {
                Ok(output) => {
                    let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    
                    if state == "exited" || state == "dead" {
                        // Get exit code
                        let exit_output = Command::new("docker")
                            .args(&["inspect", &container_id, "--format", "{{.State.ExitCode}}"])
                            .output()
                            .await;

                        let mut s = status.write().await;
                        s.state = SandboxState::Completed;
                        s.completed_at = Some(chrono::Utc::now().timestamp_millis());
                        
                        if let Ok(exit_output) = exit_output {
                            let exit_code = String::from_utf8_lossy(&exit_output.stdout)
                                .trim()
                                .parse::<i32>()
                                .unwrap_or(-1);
                            s.exit_code = Some(exit_code);
                            
                            if exit_code != 0 {
                                s.state = SandboxState::Failed;
                            }
                        }
                        
                        break;
                    }
                }
                Err(_) => {
                    // Container may have been removed
                    let mut s = status.write().await;
                    s.state = SandboxState::Completed;
                    break;
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Get current status
    pub async fn status(&self) -> SandboxStatus {
        self.status.read().await.clone()
    }
}

/// Sandbox execution result
#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Kubernetes sandbox
#[derive(Debug)]
pub struct KubernetesSandbox {
    pod_name: Option<String>,
    namespace: String,
    _config: SandboxConfig,
    status: Arc<RwLock<SandboxStatus>>,
}

impl KubernetesSandbox {
    pub fn new(execution_id: &str, config: SandboxConfig, namespace: String) -> Self {
        let status = SandboxStatus {
            id: format!("k8s-sandbox-{}", execution_id),
            execution_id: execution_id.to_string(),
            state: SandboxState::Creating,
            created_at: chrono::Utc::now().timestamp_millis(),
            started_at: None,
            completed_at: None,
            exit_code: None,
            metrics: SandboxMetrics::default(),
        };

        Self {
            pod_name: None,
            namespace,
            _config: config,
            status: Arc::new(RwLock::new(status)),
        }
    }

    /// Create a pod
    pub async fn create(&mut self, _working_dir: &str) -> anyhow::Result<SandboxStatus> {
        // Validate kubectl
        match Command::new("kubectl").args(&["version", "--client"]).output().await {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("kubectl is not installed or not in PATH")),
        }

        let pod_name = format!("drc-{}", self.status.read().await.execution_id);
        self.pod_name = Some(pod_name.clone());

        // In production, generate and apply pod YAML
        // For now, this is a simplified placeholder
        
        {
            let mut status = self.status.write().await;
            status.state = SandboxState::Running;
            status.started_at = Some(chrono::Utc::now().timestamp_millis());
        }

        info!("Kubernetes sandbox {} created in namespace {}", pod_name, self.namespace);

        Ok(self.status.read().await.clone())
    }

    /// Execute command in pod
    pub async fn execute(&self, command: &[String]) -> anyhow::Result<SandboxResult> {
        let pod_name = self
            .pod_name
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Pod not created"))?;

        let mut args = vec![
            "exec".to_string(),
            pod_name.clone(),
            "-n".to_string(),
            self.namespace.clone(),
            "--".to_string(),
        ];
        args.extend(command.iter().cloned());

        let output = Command::new("kubectl")
            .args(&args)
            .output()
            .await?;

        Ok(SandboxResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms: 0,
        })
    }

    /// Destroy the pod
    pub async fn destroy(&mut self) -> anyhow::Result<()> {
        if let Some(ref pod_name) = self.pod_name {
            let _ = Command::new("kubectl")
                .args(&["delete", "pod", pod_name, "-n", &self.namespace, "--force"])
                .output()
                .await;

            let mut status = self.status.write().await;
            status.state = SandboxState::Destroyed;
            status.completed_at = Some(chrono::Utc::now().timestamp_millis());

            info!("Kubernetes sandbox {} destroyed", pod_name);
        }

        Ok(())
    }

    /// Get status
    pub async fn status(&self) -> SandboxStatus {
        self.status.read().await.clone()
    }
}

/// Sandbox factory
pub struct SandboxFactory;

impl SandboxFactory {
    /// Create appropriate sandbox based on type
    pub fn create(
        execution_id: &str,
        config: SandboxConfig,
        demo_mode: bool,
    ) -> Box<dyn Sandbox> {
        match config.sandbox_type {
            SandboxType::Docker => {
                Box::new(DockerSandbox::new(execution_id, config, demo_mode))
            }
            SandboxType::Kubernetes => {
                Box::new(KubernetesSandbox::new(
                    execution_id,
                    config,
                    "default".to_string(),
                ))
            }
        }
    }
}

/// Sandbox trait
#[async_trait::async_trait]
pub trait Sandbox: Send + Sync {
    async fn create(&mut self, working_dir: &str) -> anyhow::Result<SandboxStatus>;
    async fn execute(&self, command: &[String]) -> anyhow::Result<SandboxResult>;
    async fn destroy(&mut self) -> anyhow::Result<()>;
    async fn status(&self) -> SandboxStatus;
}

#[async_trait::async_trait]
impl Sandbox for DockerSandbox {
    async fn create(&mut self, working_dir: &str) -> anyhow::Result<SandboxStatus> {
        self.create(working_dir).await
    }

    async fn execute(&self, command: &[String]) -> anyhow::Result<SandboxResult> {
        self.execute(command).await
    }

    async fn destroy(&mut self) -> anyhow::Result<()> {
        self.destroy().await
    }

    async fn status(&self) -> SandboxStatus {
        self.status().await
    }
}

#[async_trait::async_trait]
impl Sandbox for KubernetesSandbox {
    async fn create(&mut self, working_dir: &str) -> anyhow::Result<SandboxStatus> {
        self.create(working_dir).await
    }

    async fn execute(&self, command: &[String]) -> anyhow::Result<SandboxResult> {
        self.execute(command).await
    }

    async fn destroy(&mut self) -> anyhow::Result<()> {
        self.destroy().await
    }

    async fn status(&self) -> SandboxStatus {
        self.status().await
    }
}
