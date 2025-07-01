pub mod types;
pub mod device_status;
pub mod websocket;
pub mod voice;
pub mod client;
pub mod config;

pub use types::*;
pub use device_status::DeviceStatusChecker;
pub use websocket::WebSocketProtocol;
pub use voice::{MicrophoneOpusRecorder, NodeAudioPlayer};
pub use client::Client;
pub use config::Config;

use tracing_subscriber::{fmt, EnvFilter};

/// 库的版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志记录
pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
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