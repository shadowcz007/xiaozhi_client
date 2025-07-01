use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// 客户端操作结果类型
pub type Result<T> = std::result::Result<T, ClientError>;

/// 设备状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceState {
    /// 空闲状态
    Idle,
    /// 连接中
    Connecting,
    /// 监听中
    Listening,
    /// 播放中
    Speaking,
}

impl fmt::Display for DeviceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceState::Idle => write!(f, "空闲"),
            DeviceState::Connecting => write!(f, "连接中"),
            DeviceState::Listening => write!(f, "监听中"),
            DeviceState::Speaking => write!(f, "播放中"),
        }
    }
}

/// 监听模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListeningMode {
    /// 持续监听
    AlwaysOn,
    /// 自动停止
    AutoStop,
    /// 手动控制
    Manual,
}

impl ListeningMode {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            ListeningMode::AlwaysOn => "always_on",
            ListeningMode::AutoStop => "auto_stop",
            ListeningMode::Manual => "manual",
        }
    }
}

/// 音频参数配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioParams {
    /// 音频格式
    pub format: String,
    /// 采样率
    pub sample_rate: u32,
    /// 声道数
    pub channels: u16,
    /// 帧持续时间（毫秒）
    pub frame_duration: u32,
}

/// 音频配置
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// 输入采样率
    pub input_sample_rate: u32,
    /// 输出采样率
    pub output_sample_rate: u32,
    /// 声道数
    pub channels: u16,
    /// 帧持续时间（毫秒）
    pub frame_duration: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            input_sample_rate: 16000,
            output_sample_rate: 24000,
            channels: 1,
            frame_duration: 20,
        }
    }
}

/// WebSocket消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "hello")]
    Hello {
        version: u32,
        transport: String,
        audio_params: AudioParams,
    },
    #[serde(rename = "text")]
    Text {
        content: String,
        timestamp: i64,
    },
    #[serde(rename = "audio")]
    Audio {
        data: Vec<u8>,
        timestamp: i64,
    },
    #[serde(rename = "interrupt")]
    Interrupt {
        timestamp: i64,
    },
}

/// 设备状态响应
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceStatusResponse {
    #[serde(default)]
    pub code: Option<i32>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub data: Option<DeviceData>,
    pub firmware: FirmwareConfig,
    pub mqtt: MqttConfig,
    pub server_time: ServerTimeConfig,
    pub websocket: WebSocketConfig,
}

/// 固件配置
#[derive(Debug, Clone, Deserialize)]
pub struct FirmwareConfig {
    pub url: String,
    pub version: String,
}

/// 服务器时间配置
#[derive(Debug, Clone, Deserialize)]
pub struct ServerTimeConfig {
    pub timestamp: i64,
    pub timezone_offset: i32,
}

/// WebSocket配置
#[derive(Debug, Clone, Deserialize)]
pub struct WebSocketConfig {
    pub url: String,
    pub token: String,
    #[serde(default)]
    pub protocols: Option<Vec<String>>,
}

/// MQTT配置
#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub client_id: String,
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub publish_topic: String,
    pub subscribe_topic: String,
}

/// 设备数据
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceData {
    pub device_id: String,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub activated: bool,
    pub activation_time: Option<String>,
}

/// 客户端错误类型
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("网络错误: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("音频错误: {0}")]
    AudioError(String),
    
    #[error("Opus编解码错误: {0}")]
    OpusError(#[from] opus::Error),
    
    #[error("WebSocket错误: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    
    #[error("JSON错误: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("设备状态无效: {0}")]
    InvalidDeviceStatus(String),

    #[error("连接超时")]
    ConnectionTimeout,

    #[error("设备未激活")]
    DeviceNotActivated,
}

impl From<String> for ClientError {
    fn from(err: String) -> Self {
        ClientError::AudioError(err)
    }
}

impl From<&str> for ClientError {
    fn from(err: &str) -> Self {
        ClientError::AudioError(err.to_string())
    }
}

impl ClientError {
    /// 创建音频错误
    pub fn audio_error<T: Into<String>>(msg: T) -> Self {
        ClientError::AudioError(msg.into())
    }

    /// 创建设备状态错误
    pub fn invalid_state<T: Into<String>>(msg: T) -> Self {
        ClientError::InvalidDeviceStatus(msg.into())
    }

    /// 检查是否为网络相关错误
    pub fn is_network_error(&self) -> bool {
        matches!(self, 
            ClientError::NetworkError(_) | 
            ClientError::WebSocketError(_) | 
            ClientError::ConnectionTimeout
        )
    }

    /// 检查是否为音频相关错误
    pub fn is_audio_error(&self) -> bool {
        matches!(self, 
            ClientError::AudioError(_) | 
            ClientError::OpusError(_)
        )
    }
} 