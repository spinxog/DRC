use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{header::{HeaderName, HeaderValue}, Request, Response, StatusCode},
    routing::any,
    Router,
    Server,
};
use hyper::{body::to_bytes, Client};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, error};

/// Proxy configuration
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub port: u16,
    pub host: String,
    pub target_host: String,
    pub target_port: u16,
    pub protocol: ProxyProtocol,
    pub capture_request_body: bool,
    pub capture_response_body: bool,
    pub max_body_size: usize,
    pub correlation_header: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyProtocol {
    Http,
    Https,
}

/// Captured HTTP request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedRequest {
    pub id: String,
    pub timestamp: i64,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub correlation_id: String,
}

/// Captured HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedResponse {
    pub request_id: String,
    pub timestamp: i64,
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub duration_ms: u64,
}

/// HTTP Proxy server
#[derive(Debug, Clone)]
pub struct HTTPProxy {
    config: ProxyConfig,
    captured_requests: Arc<RwLock<Vec<CapturedRequest>>>,
    captured_responses: Arc<RwLock<Vec<CapturedResponse>>>,
}

impl HTTPProxy {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            captured_requests: Arc::new(RwLock::new(Vec::new())),
            captured_responses: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start the proxy server
    pub async fn start(&self) -> anyhow::Result<()> {
        let app = Router::new()
            .route("/*path", any(proxy_handler))
            .route("/", any(proxy_handler))
            .with_state(self.clone());

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        info!("HTTP Proxy listening on {}", addr);

        Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;

        Ok(())
    }

    /// Get captured requests
    pub async fn get_captured_requests(&self) -> Vec<CapturedRequest> {
        let requests = self.captured_requests.read().await;
        requests.clone()
    }

    /// Get captured responses
    pub async fn get_captured_responses(&self) -> Vec<CapturedResponse> {
        let responses = self.captured_responses.read().await;
        responses.clone()
    }

    /// Clear captured data
    pub async fn clear_captured(&self) {
        let mut requests = self.captured_requests.write().await;
        requests.clear();
        let mut responses = self.captured_responses.write().await;
        responses.clear();
    }
}

/// Proxy request handler
async fn proxy_handler(
    State(proxy): State<HTTPProxy>,
    request: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    let start = std::time::Instant::now();
    let request_id = format!("req_{}", chrono::Utc::now().timestamp_millis());
    let correlation_id = request
        .headers()
        .get(&proxy.config.correlation_header)
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&request_id)
        .to_string();

    // Capture request
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let headers: HashMap<String, String> = request
        .headers()
        .iter()
        .filter_map(|(k, v): (&HeaderName, &HeaderValue)| {
            v.to_str().ok().map(|val| (k.to_string(), val.to_string()))
        })
        .collect();

    let captured_req = CapturedRequest {
        id: request_id.clone(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        method: method.clone(),
        url: uri.clone(),
        headers: headers.clone(),
        body: None,
        correlation_id: correlation_id.clone(),
    };

    {
        let mut requests = proxy.captured_requests.write().await;
        requests.push(captured_req);
    }

    // Forward to target
    let client = Client::new();
    let target_uri = format!(
        "http://{}:{}{}",
        proxy.config.target_host,
        proxy.config.target_port,
        uri
    );

    let mut target_request = Request::builder()
        .method(method.as_str())
        .uri(&target_uri);

    // Copy headers
    for (key, value) in &headers {
        target_request = target_request.header(key, value);
    }

    let body_bytes: Bytes = match to_bytes(request.into_body()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let target_request = match target_request.body(Body::from(body_bytes.clone())) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to build target request: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let response: hyper::Response<hyper::Body> = match client.request(target_request).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Proxy request failed: {}", e);
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    // Capture response
    let status = response.status().as_u16();
    let response_headers: HashMap<String, String> = response
        .headers()
        .iter()
        .filter_map(|(k, v): (&HeaderName, &axum::http::header::HeaderValue)| {
            v.to_str().ok().map(|val| (k.to_string(), val.to_string()))
        })
        .collect();

    let captured_resp = CapturedResponse {
        request_id: request_id.clone(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        status_code: status,
        headers: response_headers,
        body: None,
        duration_ms: start.elapsed().as_millis() as u64,
    };

    {
        let mut responses = proxy.captured_responses.write().await;
        responses.push(captured_resp);
    }

    // Build response
    let (parts, body) = response.into_parts();
    let body_bytes: Bytes = match to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read response body: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut builder = Response::builder().status(parts.status);
    for (key, value) in &parts.headers {
        builder = builder.header(key, value);
    }

    builder
        .body(Body::from(body_bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// gRPC Proxy
#[derive(Debug, Clone)]
pub struct GRPCProxy {
    config: ProxyConfig,
}

impl GRPCProxy {
    pub fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        info!("gRPC Proxy starting on port {}", self.config.port);
        // Simplified implementation - full gRPC would require tonic
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        info!("gRPC Proxy listening on {}", addr);
        
        // For now, just return OK as placeholder
        // Real implementation would use tonic for gRPC
        Ok(())
    }
}

/// PostgreSQL Proxy
#[derive(Debug, Clone)]
pub struct PostgreSQLProxy {
    port: u16,
    target_host: String,
    target_port: u16,
}

impl PostgreSQLProxy {
    pub fn new(port: u16, target_host: String, target_port: u16) -> Self {
        Self {
            port,
            target_host,
            target_port,
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        info!(
            "PostgreSQL Proxy starting on port {} -> {}:{}",
            self.port, self.target_host, self.target_port
        );

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;

        loop {
            let (client_socket, _) = listener.accept().await?;
            let target_host = self.target_host.clone();
            let target_port = self.target_port;

            tokio::spawn(async move {
                if let Err(e) = handle_pg_connection(client_socket, target_host, target_port).await {
                    error!("PostgreSQL proxy connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_pg_connection(
    mut client: tokio::net::TcpStream,
    target_host: String,
    target_port: u16,
) -> anyhow::Result<()> {
    let mut target = tokio::net::TcpStream::connect(format!("{}:{}", target_host, target_port)).await?;

    let (mut client_read, mut client_write) = client.split();
    let (mut target_read, mut target_write) = target.split();

    let client_to_target = tokio::io::copy(&mut client_read, &mut target_write);
    let target_to_client = tokio::io::copy(&mut target_read, &mut client_write);

    tokio::try_join!(client_to_target, target_to_client)?;

    Ok(())
}

/// Redis Proxy
#[derive(Debug, Clone)]
pub struct RedisProxy {
    port: u16,
    target_host: String,
    target_port: u16,
}

impl RedisProxy {
    pub fn new(port: u16, target_host: String, target_port: u16) -> Self {
        Self {
            port,
            target_host,
            target_port,
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        info!(
            "Redis Proxy starting on port {} -> {}:{}",
            self.port, self.target_host, self.target_port
        );

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;

        loop {
            let (client_socket, _) = listener.accept().await?;
            let target_host = self.target_host.clone();
            let target_port = self.target_port;

            tokio::spawn(async move {
                if let Err(e) = handle_redis_connection(client_socket, target_host, target_port).await {
                    error!("Redis proxy connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_redis_connection(
    mut client: tokio::net::TcpStream,
    target_host: String,
    target_port: u16,
) -> anyhow::Result<()> {
    let mut target = tokio::net::TcpStream::connect(format!("{}:{}", target_host, target_port)).await?;

    let (mut client_read, mut client_write) = client.split();
    let (mut target_read, mut target_write) = target.split();

    let client_to_target = tokio::io::copy(&mut client_read, &mut target_write);
    let target_to_client = tokio::io::copy(&mut target_read, &mut client_write);

    tokio::try_join!(client_to_target, target_to_client)?;

    Ok(())
}
