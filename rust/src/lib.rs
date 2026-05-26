pub mod client;
pub mod config;
pub mod crypto;
pub mod device_manager;
pub mod device_status;
pub mod fingerprint;
pub mod mcp;
pub mod stdio_controller;
pub mod types;
pub mod ui;
pub mod voice;
pub mod websocket;

pub use client::Client;
pub use config::Config;
pub use device_manager::{Device, DeviceManager};
pub use device_status::{
    ActivationInfo, ActivationResult, DeviceStatusChecker, DeviceStatusResult,
};
pub use fingerprint::DeviceFingerprint;
pub use mcp::{types::Content, types::MCPMessage, types::Resource, types::Tool, MCPProtocol};
pub use stdio_controller::StdioController;
pub use types::*;
pub use voice::{MicrophoneOpusRecorder, NodeAudioPlayer};
pub use websocket::WebSocketProtocol;

use std::sync::Arc;
use tracing_subscriber::{fmt, EnvFilter};

/// 库的版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志记录
pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error"));

    fmt().with_target(false).with_env_filter(env_filter).init();
}

/// 初始化日志记录（调试模式）
pub fn init_debug_logging() {
    let env_filter =
        EnvFilter::try_from_env("RUST_LOG").unwrap_or_else(|_| EnvFilter::new("debug"));

    fmt().with_target(false).with_env_filter(env_filter).init();
}

// 导出主要功能
pub async fn create_client(
    websocket_url: String,
    access_token: String,
    device_id: String,
    client_id: String,
) -> Result<Arc<Client>> {
    let client = Arc::new(Client::from_params(
        websocket_url,
        access_token,
        device_id,
        client_id,
    )?);
    let client_ref = Arc::clone(&client);
    client_ref.setup_mcp_client_ref().await?;
    Ok(client)
}

// 导出WebSocket ID生成函数
pub use websocket::{generate_client_id, generate_device_id};

// 导出MCP协议相关
// 注：所有MCP类型已在上面导入
