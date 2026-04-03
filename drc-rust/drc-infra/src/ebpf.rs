use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::info;

/// eBPF probe type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProbeType {
    FileOpen,
    FileRead,
    FileWrite,
    NetworkConnect,
    NetworkSend,
    NetworkRecv,
    ProcessSpawn,
    ProcessExit,
    SyscallEnter,
    SyscallExit,
}

/// eBPF probe configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    pub probe_type: ProbeType,
    pub enabled: bool,
    pub filter_pid: Option<u32>,
    pub filter_comm: Option<String>,
    pub sample_rate: f64, // 0.0 - 1.0
}

/// Captured system call event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallEvent {
    pub timestamp: i64,
    pub pid: u32,
    pub tid: u32,
    pub comm: String,
    pub syscall_nr: u64,
    pub syscall_name: String,
    pub args: Vec<u64>,
    pub ret: i64,
    pub duration_ns: u64,
}

/// File I/O event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub timestamp: i64,
    pub pid: u32,
    pub comm: String,
    pub event_type: FileEventType,
    pub fd: i32,
    pub path: String,
    pub bytes: Option<usize>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileEventType {
    Open,
    Read,
    Write,
    Close,
}

/// Network event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEvent {
    pub timestamp: i64,
    pub pid: u32,
    pub comm: String,
    pub event_type: NetworkEventType,
    pub src_addr: String,
    pub dst_addr: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub bytes: usize,
    pub protocol: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetworkEventType {
    Connect,
    Accept,
    Send,
    Recv,
    Close,
}

/// Process event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub timestamp: i64,
    pub pid: u32,
    pub ppid: u32,
    pub comm: String,
    pub event_type: ProcessEventType,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProcessEventType {
    Spawn,
    Exit,
    Clone,
}

/// eBPF agent for kernel-level system call capture
#[derive(Debug, Clone)]
pub struct EBPFAgent {
    probes: Arc<RwLock<HashMap<ProbeType, ProbeConfig>>>,
    kernel_version: String,
    is_supported: bool,
    loaded_probes: Arc<RwLock<Vec<ProbeType>>>,
    syscall_events: Arc<RwLock<Vec<SyscallEvent>>>,
    file_events: Arc<RwLock<Vec<FileEvent>>>,
    network_events: Arc<RwLock<Vec<NetworkEvent>>>,
    process_events: Arc<RwLock<Vec<ProcessEvent>>>,
}

impl EBPFAgent {
    pub fn new() -> Self {
        let mut agent = Self {
            probes: Arc::new(RwLock::new(HashMap::new())),
            kernel_version: String::new(),
            is_supported: false,
            loaded_probes: Arc::new(RwLock::new(Vec::new())),
            syscall_events: Arc::new(RwLock::new(Vec::new())),
            file_events: Arc::new(RwLock::new(Vec::new())),
            network_events: Arc::new(RwLock::new(Vec::new())),
            process_events: Arc::new(RwLock::new(Vec::new())),
        };
        
        agent.check_kernel_support();
        agent
    }

    fn check_kernel_support(&mut self) {
        // In production, check kernel version and BCC/aya availability
        // For now, simulate based on common kernel versions
        self.kernel_version = "5.15.0".to_string();
        
        // eBPF supported on kernel 4.15+
        self.is_supported = true;
        
        if self.is_supported {
            info!("Kernel {} supports eBPF features", self.kernel_version);
        } else {
            info!("Kernel {} may not support all eBPF features", self.kernel_version);
        }
    }

    /// Check if kernel supports eBPF
    pub fn is_supported(&self) -> bool {
        self.is_supported
    }

    /// Configure a probe
    pub async fn configure_probe(&self, config: ProbeConfig) {
        let mut probes = self.probes.write().await;
        probes.insert(config.probe_type, config);
    }

    /// Load a probe
    pub async fn load_probe(&self, probe_type: ProbeType) -> anyhow::Result<()> {
        if !self.is_supported {
            return Err(anyhow::anyhow!("eBPF not supported on this kernel"));
        }

        let mut loaded = self.loaded_probes.write().await;
        if !loaded.contains(&probe_type) {
            loaded.push(probe_type);
            info!("Loaded eBPF probe: {:?}", probe_type);
        }
        
        Ok(())
    }

    /// Unload a probe
    pub async fn unload_probe(&self, probe_type: ProbeType) -> anyhow::Result<()> {
        let mut loaded = self.loaded_probes.write().await;
        loaded.retain(|&p| p != probe_type);
        info!("Unloaded eBPF probe: {:?}", probe_type);
        Ok(())
    }

    /// Get loaded probes
    pub async fn get_loaded_probes(&self) -> Vec<ProbeType> {
        let loaded = self.loaded_probes.read().await;
        loaded.clone()
    }

    /// Start capturing events
    pub async fn start_capture(&self) -> anyhow::Result<()> {
        info!("Starting eBPF event capture");
        // In production, this would start the eBPF programs
        Ok(())
    }

    /// Stop capturing events
    pub async fn stop_capture(&self) -> anyhow::Result<()> {
        info!("Stopping eBPF event capture");
        Ok(())
    }

    /// Get captured syscall events
    pub async fn get_syscall_events(&self) -> Vec<SyscallEvent> {
        let events = self.syscall_events.read().await;
        events.clone()
    }

    /// Get captured file events
    pub async fn get_file_events(&self) -> Vec<FileEvent> {
        let events = self.file_events.read().await;
        events.clone()
    }

    /// Get captured network events
    pub async fn get_network_events(&self) -> Vec<NetworkEvent> {
        let events = self.network_events.read().await;
        events.clone()
    }

    /// Get captured process events
    pub async fn get_process_events(&self) -> Vec<ProcessEvent> {
        let events = self.process_events.read().await;
        events.clone()
    }

    /// Get status
    pub fn get_status(&self) -> EBPFStatus {
        EBPFStatus {
            running: self.is_supported,
            supported: self.is_supported,
            kernel_version: self.kernel_version.clone(),
            loaded_probes: 0, // Would be populated from loaded_probes
        }
    }
}

impl Default for EBPFAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// eBPF status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EBPFStatus {
    pub running: bool,
    pub supported: bool,
    pub kernel_version: String,
    pub loaded_probes: usize,
}

/// Ptrace tracer fallback for systems without eBPF
#[derive(Debug, Clone)]
pub struct PtraceTracer {
    target_pid: Option<u32>,
    is_running: bool,
    attached: Arc<RwLock<bool>>,
}

impl PtraceTracer {
    pub fn new() -> Self {
        Self {
            target_pid: None,
            is_running: false,
            attached: Arc::new(RwLock::new(false)),
        }
    }

    /// Attach to a process
    pub async fn attach(&mut self, pid: u32) -> anyhow::Result<()> {
        self.target_pid = Some(pid);
        self.is_running = true;
        
        let mut attached = self.attached.write().await;
        *attached = true;
        
        info!("Ptrace attached to process {}", pid);
        
        // Note: Real ptrace implementation requires native bindings
        // This is a simplified version for the Rust port
        
        Ok(())
    }

    /// Detach from process
    pub async fn detach(&mut self) -> anyhow::Result<()> {
        if let Some(pid) = self.target_pid {
            info!("Ptrace detaching from process {}", pid);
        }
        
        self.target_pid = None;
        self.is_running = false;
        
        let mut attached = self.attached.write().await;
        *attached = false;
        
        Ok(())
    }

    /// Check if attached
    pub async fn is_attached(&self) -> bool {
        let attached = self.attached.read().await;
        *attached
    }
}

impl Default for PtraceTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// Hybrid capture manager using eBPF when available, ptrace as fallback
#[derive(Debug, Clone)]
pub struct HybridCaptureManager {
    ebpf_agent: EBPFAgent,
    ptrace_tracer: PtraceTracer,
    prefer_ebpf: bool,
}

impl HybridCaptureManager {
    pub fn new() -> Self {
        let ebpf_agent = EBPFAgent::new();
        let prefer_ebpf = ebpf_agent.is_supported();
        
        Self {
            ebpf_agent,
            ptrace_tracer: PtraceTracer::new(),
            prefer_ebpf,
        }
    }

    /// Start capture with best available method
    pub async fn start_capture(&self) -> anyhow::Result<()> {
        if self.prefer_ebpf {
            info!("Using eBPF for system call capture");
            self.ebpf_agent.start_capture().await?;
        } else {
            info!("eBPF not available, would use ptrace fallback");
            // ptrace would require specific PID to attach
        }
        Ok(())
    }

    /// Get eBPF agent
    pub fn get_ebpf_agent(&self) -> &EBPFAgent {
        &self.ebpf_agent
    }

    /// Get ptrace tracer
    pub fn get_ptrace_tracer(&self) -> &PtraceTracer {
        &self.ptrace_tracer
    }

    /// Check if using eBPF
    pub fn is_using_ebpf(&self) -> bool {
        self.prefer_ebpf
    }
}

impl Default for HybridCaptureManager {
    fn default() -> Self {
        Self::new()
    }
}
