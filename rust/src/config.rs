use crate::types::AudioConfig;

/// 客户端配置
#[derive(Debug, Clone)]
pub struct Config {
    /// WebSocket URL
    pub websocket_url: String,
    /// 访问令牌
    pub access_token: String,
    /// 设备ID
    pub device_id: String,
    /// 客户端ID
    pub client_id: String,
    /// OTA 服务URL
    pub ota_url: String,
    /// 音频配置
    pub audio: AudioConfig,
    /// 连接超时时间（毫秒）
    pub connect_timeout: u64,
    /// 重连最大次数
    pub max_reconnect_attempts: u32,
}

impl Config {
    pub fn new(websocket_url: String, access_token: String, device_id: String, client_id: String) -> Self {
        Self {
            websocket_url,
            access_token,
            device_id,
            client_id,
            ota_url: "https://api.tenclass.net/xiaozhi/ota/".to_string(),
            audio: AudioConfig::default(),
            connect_timeout: 15000,
            max_reconnect_attempts: 3,
        }
    }

    pub fn with_ota_url(mut self, ota_url: String) -> Self {
        self.ota_url = ota_url;
        self
    }

    pub fn with_audio_config(mut self, audio: AudioConfig) -> Self {
        self.audio = audio;
        self
    }

    pub fn with_connect_timeout(mut self, timeout: u64) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn with_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = attempts;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            websocket_url: String::new(),
            access_token: String::new(),
            device_id: String::new(),
            client_id: String::new(),
            ota_url: "https://api.tenclass.net/xiaozhi/ota/".to_string(),
            audio: AudioConfig::default(),
            connect_timeout: 15000,
            max_reconnect_attempts: 3,
        }
    }
} 