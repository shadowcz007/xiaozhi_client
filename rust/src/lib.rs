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

/// 库的版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志记录
pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
}

/// 初始化日志记录（调试模式）
pub fn init_debug_logging() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
} 