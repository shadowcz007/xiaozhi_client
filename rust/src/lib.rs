pub mod types;
pub mod device_status;
pub mod websocket;
pub mod voice;
pub mod client;
pub mod config;
pub mod crypto;
pub mod stdio_controller;
pub mod mcp;

pub use types::*;
pub use device_status::DeviceStatusChecker;
pub use websocket::WebSocketProtocol;
pub use voice::{MicrophoneOpusRecorder, NodeAudioPlayer};
pub use client::Client;
pub use config::Config;
pub use stdio_controller::StdioController;
pub use mcp::{MCPProtocol, types::MCPMessage, types::Tool, types::Resource, types::Content};

use tracing_subscriber::{fmt, EnvFilter};
use std::sync::Arc;

/// 库的版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志记录
pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("error"));
    
    fmt()
        .with_target(false)
        .with_env_filter(env_filter)
        .init();
}

/// 初始化日志记录（调试模式）
pub fn init_debug_logging() {
    let env_filter = EnvFilter::try_from_env("RUST_LOG")
        .unwrap_or_else(|_| EnvFilter::new("debug"));
    
    fmt()
        .with_target(false)
        .with_env_filter(env_filter)
        .init();
}

// 导出主要功能
pub async fn create_client(
    websocket_url: String,
    access_token: String,
    device_id: String,
    client_id: String,
) -> Result<Arc<Client>> {
    let client = Arc::new(Client::from_params(websocket_url, access_token, device_id, client_id)?);
    let client_ref = Arc::clone(&client);
    client_ref.setup_mcp_client_ref().await?;
    Ok(client)
}

// 导出WebSocket ID生成函数
pub use websocket::{generate_device_id, generate_client_id};

// 导出MCP协议相关
// 注：所有MCP类型已在上面导入 