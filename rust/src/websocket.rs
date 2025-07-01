use tokio_tungstenite::{connect_async_with_config, tungstenite::{Message, client::IntoClientRequest}};
use futures::{SinkExt, StreamExt};

use std::time::Duration;
use tokio::time::timeout;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use uuid::Uuid;
use crate::types::{Result, ClientError};
use crate::config::Config;
use std::sync::atomic::{AtomicBool, Ordering};

/// WebSocket事件类型
#[derive(Debug, Clone)]
pub enum WebSocketEvent {
    Connected,
    AudioChannelOpened,
    AudioChannelClosed,
    IncomingAudio(Vec<u8>),
    IncomingJson(serde_json::Value),
    NetworkError(String),
}

/// WebSocket协议处理器
pub struct WebSocketProtocol {
    config: Config,
    connected: Arc<AtomicBool>,
    hello_received: Arc<Mutex<bool>>,
    session_id: Arc<Mutex<Option<String>>>,
    event_sender: Option<mpsc::UnboundedSender<WebSocketEvent>>,
    write_sender: Option<mpsc::UnboundedSender<Message>>,
}

// 为 WebSocketProtocol 手动实现 Send 和 Sync
unsafe impl Send for WebSocketProtocol {}
unsafe impl Sync for WebSocketProtocol {}

impl WebSocketProtocol {
    /// 创建新的WebSocket协议处理器
    pub fn new(config: Config) -> Self {
        Self {
            config,
            connected: Arc::new(AtomicBool::new(false)),
            hello_received: Arc::new(Mutex::new(false)),
            session_id: Arc::new(Mutex::new(None)),
            event_sender: None,
            write_sender: None,
        }
    }

    /// 连接到WebSocket服务器
    /// 
    /// # Returns
    /// * `mpsc::UnboundedReceiver<WebSocketEvent>` - 事件接收器
    pub async fn connect(&mut self) -> Result<mpsc::UnboundedReceiver<WebSocketEvent>> {
        {
            let mut hello_guard = self.hello_received.lock().await;
            *hello_guard = false;
        }
        self.connected.store(false, Ordering::Relaxed);
        {
            let mut session_guard = self.session_id.lock().await;
            *session_guard = None;
        }

        // 创建事件通道
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        self.event_sender = Some(event_sender.clone());

        tracing::info!("🌐 连接到WebSocket服务器: {}", self.config.websocket_url);

        // 创建WebSocket请求并设置自定义头部
        let mut request = self.config.websocket_url.clone().into_client_request()?;
        let headers = request.headers_mut();
        
        // 设置自定义头部
        headers.insert("Authorization", format!("Bearer {}", self.config.access_token).parse().unwrap());
        headers.insert("Protocol-Version", "1".parse().unwrap());
        headers.insert("Device-Id", self.config.device_id.parse().unwrap());
        headers.insert("Client-Id", self.config.client_id.parse().unwrap());

        tracing::info!("[调试] WebSocket请求头: Authorization=Bearer {}, Device-Id={}, Client-Id={}", 
            self.config.access_token, self.config.device_id, self.config.client_id);

        // 建立WebSocket连接
        let connection_result = timeout(
            Duration::from_millis(self.config.connect_timeout),
            connect_async_with_config(request, None, false)
        ).await;

        let (ws_stream, response) = match connection_result {
            Ok(Ok((stream, response))) => {
                tracing::debug!("WebSocket连接建立: {:?}", response.status());
                (stream, response)
            }
            Ok(Err(e)) => {
                let error_msg = format!("WebSocket连接失败: {}", e);
                tracing::error!("{}", error_msg);
                let _ = event_sender.send(WebSocketEvent::NetworkError(error_msg.clone()));
                return Err(ClientError::WebSocketError(e));
            }
            Err(_) => {
                let error_msg = "WebSocket连接超时".to_string();
                tracing::error!("{}", error_msg);
                let _ = event_sender.send(WebSocketEvent::NetworkError(error_msg));
                return Err(ClientError::ConnectionTimeout);
            }
        };

        let (mut write, read) = ws_stream.split();

        // 创建写入通道
        let (write_sender, mut write_receiver) = mpsc::unbounded_channel::<Message>();
        self.write_sender = Some(write_sender.clone());

        // 启动写入任务
        let write_event_sender = event_sender.clone();
        tokio::spawn(async move {
            while let Some(message) = write_receiver.recv().await {
                if let Err(e) = write.send(message).await {
                    tracing::error!("发送消息失败: {}", e);
                    let _ = write_event_sender.send(WebSocketEvent::NetworkError(format!("发送失败: {}", e)));
                    break;
                }
            }
        });

        // 发送Hello消息
        let hello_message = self.create_hello_message();
        let hello_text = serde_json::to_string(&hello_message)?;
        
        if let Err(e) = write_sender.send(Message::Text(hello_text)) {
            let error_msg = format!("发送Hello消息失败: {}", e);
            tracing::error!("{}", error_msg);
            let _ = event_sender.send(WebSocketEvent::NetworkError(error_msg));
            return Err(ClientError::invalid_state("发送Hello消息失败"));
        }

        tracing::debug!("已发送Hello消息");

        // 添加调试输出看看实际发送的内容
        tracing::info!("[调试] 发送的Hello消息: {}", serde_json::to_string_pretty(&hello_message)?);

        // 启动消息处理任务
        let event_sender_clone = event_sender.clone();
        let hello_received_clone = Arc::clone(&self.hello_received);
        let session_id_clone = Arc::clone(&self.session_id);
        let connected_clone = Arc::clone(&self.connected);

        let _read_task = tokio::spawn(async move {
            let mut read = read;
            
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        tracing::debug!("收到文本消息: {}", text);
                        if let Err(e) = Self::handle_text_message(&text, &event_sender_clone, &hello_received_clone, &session_id_clone).await {
                            tracing::error!("处理文本消息失败: {}", e);
                            let _ = event_sender_clone.send(WebSocketEvent::NetworkError(format!("消息处理错误: {}", e)));
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        tracing::debug!("收到二进制数据: {} bytes", data.len());
                        let _ = event_sender_clone.send(WebSocketEvent::IncomingAudio(data));
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("WebSocket连接已关闭");
                        let _ = event_sender_clone.send(WebSocketEvent::AudioChannelClosed);
                        break;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                        // 忽略ping/pong消息
                    }
                    Ok(Message::Frame(_)) => {
                        // 处理原始帧消息
                        tracing::debug!("收到原始帧消息");
                    }
                    Err(e) => {
                        tracing::error!("WebSocket读取错误: {}", e);
                        let _ = event_sender_clone.send(WebSocketEvent::NetworkError(format!("连接错误: {}", e)));
                        break;
                    }
                }
            }
            tracing::debug!("WebSocket读取任务结束，设置connected为false");
            connected_clone.store(false, Ordering::Relaxed);
        });

        // 等待Hello响应
        let hello_timeout = timeout(
            Duration::from_millis(self.config.connect_timeout),
            self.wait_for_hello()
        ).await;

        match hello_timeout {
            Ok(Ok(_session_id)) => {
                self.connected.store(true, Ordering::Relaxed);
                let _ = event_sender.send(WebSocketEvent::Connected);
                tracing::info!("✅ WebSocket连接成功");
            }
            Ok(Err(e)) => {
                let error_msg = format!("Hello握手失败: {}", e);
                tracing::error!("{}", error_msg);
                let _ = event_sender.send(WebSocketEvent::NetworkError(error_msg));
                return Err(e);
            }
            Err(_) => {
                let error_msg = "Hello握手超时".to_string();
                tracing::error!("{}", error_msg);
                let _ = event_sender.send(WebSocketEvent::NetworkError(error_msg));
                return Err(ClientError::ConnectionTimeout);
            }
        }
        
        Ok(event_receiver)
    }

    /// 等待Hello响应
    async fn wait_for_hello(&self) -> Result<String> {
        // 等待Hello响应处理完成
        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_millis(self.config.connect_timeout);
        
        while start_time.elapsed() < timeout_duration {
            let hello_received = {
                let hello_guard = self.hello_received.lock().await;
                *hello_guard
            };
            
            if hello_received {
                let session_guard = self.session_id.lock().await;
                if let Some(session_id) = session_guard.as_ref() {
                    return Ok(session_id.clone());
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        
        // 如果超时但没有获取到session_id，生成一个默认的
        let default_session_id = format!("session_{}", chrono::Utc::now().timestamp());
        tracing::warn!("⚠️ 未收到服务器session_id，使用默认值: {}", default_session_id);
        Ok(default_session_id)
    }

    /// 创建Hello消息
    fn create_hello_message(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "hello",
            "version": 1,
            "transport": "websocket",
            "audio_params": {
                "format": "opus",
                "sample_rate": self.config.audio.input_sample_rate,
                "channels": self.config.audio.channels,
                "frame_duration": self.config.audio.frame_duration
            }
        })
    }

    /// 处理文本消息
    async fn handle_text_message(
        text: &str, 
        event_sender: &mpsc::UnboundedSender<WebSocketEvent>,
        hello_received: &Arc<Mutex<bool>>,
        session_id_storage: &Arc<Mutex<Option<String>>>
    ) -> Result<()> {
        tracing::debug!("开始处理文本消息: {}", text);
        
        let json_data: serde_json::Value = match serde_json::from_str(text) {
            Ok(data) => {
                tracing::debug!("JSON解析成功: {:?}", data);
                data
            }
            Err(e) => {
                tracing::error!("JSON解析失败: {}", e);
                return Err(ClientError::JsonError(e));
            }
        };
        
        // 检查是否是Hello响应
        if let Some(msg_type) = json_data.get("type") {
            tracing::debug!("消息类型字段存在，值为: {:?}", msg_type);
            if let Some(type_str) = msg_type.as_str() {
                tracing::debug!("消息类型为字符串: {}", type_str);
                if type_str == "hello" {
                    tracing::info!("✨ 收到Hello响应，准备打开音频通道");
                    
                    // 提取session_id
                    if let Some(session_id) = json_data.get("session_id").and_then(|v| v.as_str()) {
                        let mut session_guard = session_id_storage.lock().await;
                        *session_guard = Some(session_id.to_string());
                        tracing::info!("🆔 获取到Session ID: {}", session_id);
                    }
                    
                    // 设置hello_received标志
                    {
                        let mut hello_guard = hello_received.lock().await;
                        *hello_guard = true;
                    }
                    
                    let _ = event_sender.send(WebSocketEvent::AudioChannelOpened);
                }
            } else {
                tracing::warn!("消息类型不是字符串类型");
            }
        } else {
            tracing::warn!("消息中没有type字段");
        }

        tracing::debug!("发送JSON数据到事件通道");
        let _ = event_sender.send(WebSocketEvent::IncomingJson(json_data));
        tracing::debug!("文本消息处理完成");
        Ok(())
    }

    /// 发送音频数据
    pub async fn send_audio(&mut self, audio_data: Vec<u8>) -> Result<()> {
        if !self.is_audio_channel_open().await {
            return Err(ClientError::invalid_state("音频通道未打开"));
        }

        if let Some(write_sender) = &self.write_sender {
            if let Err(_) = write_sender.send(Message::Binary(audio_data)) {
                return Err(ClientError::invalid_state("发送音频数据失败"));
            }
        } else {
            return Err(ClientError::invalid_state("写入通道未建立"));
        }
        
        Ok(())
    }

    /// 发送文本消息
    pub async fn send_text(&mut self, message: &str) -> Result<()> {
        if !self.is_connected() {
            return Err(ClientError::invalid_state("WebSocket未连接"));
        }

        if let Some(write_sender) = &self.write_sender {
            if let Err(_) = write_sender.send(Message::Text(message.to_string())) {
                return Err(ClientError::invalid_state("发送文本消息失败"));
            }
            tracing::debug!("✅ 文本消息已发送: {}", message);
        } else {
            return Err(ClientError::invalid_state("写入通道未建立"));
        }
        
        Ok(())
    }

    /// 检查音频通道是否打开
    pub async fn is_audio_channel_open(&self) -> bool {
        let hello_guard = self.hello_received.lock().await;
        self.connected.load(Ordering::Relaxed) && *hello_guard
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// 获取会话ID
    pub async fn get_session_id(&self) -> Option<String> {
        let session_guard = self.session_id.lock().await;
        session_guard.clone()
    }

    /// 设置会话ID
    pub async fn set_session_id(&self, session_id: String) {
        let mut session_guard = self.session_id.lock().await;
        *session_guard = Some(session_id);
    }

    /// 打开音频通道
    pub async fn open_audio_channel(&mut self) -> Result<()> {
        if !self.is_connected() {
            return Err(ClientError::invalid_state("WebSocket未连接"));
        }

        // 发送打开音频通道的消息
        tracing::info!("🎵 打开音频通道");
        Ok(())
    }

    /// 关闭音频通道
    pub async fn close_audio_channel(&mut self) -> Result<()> {
        if let Some(event_sender) = &self.event_sender {
            let _ = event_sender.send(WebSocketEvent::AudioChannelClosed);
        }
        
        self.connected.store(false, Ordering::Relaxed);
        {
            let mut hello_guard = self.hello_received.lock().await;
            *hello_guard = false;
        }
        {
            let mut session_guard = self.session_id.lock().await;
            *session_guard = None;
        }
        
        tracing::info!("🔇 音频通道已关闭");
        Ok(())
    }

    /// 更新配置
    pub fn update_config(&mut self, new_config: Config) {
        self.config = new_config;
    }

    /// 销毁连接
    pub fn destroy(&mut self) {
        self.connected.store(false, Ordering::Relaxed);
        self.event_sender = None;
        self.write_sender = None;
        tracing::info!("🔌 WebSocket协议已销毁");
    }
}

/// 生成设备ID
pub fn generate_device_id() -> String {
    Uuid::new_v4().to_string()
}

/// 生成客户端ID
pub fn generate_client_id() -> String {
    let uuid_str = Uuid::new_v4().simple().to_string();
    format!("client_{}_{}", 
        chrono::Utc::now().timestamp(),
        &uuid_str[..9]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ids() {
        let device_id = generate_device_id();
        let client_id = generate_client_id();
        
        assert!(!device_id.is_empty());
        assert!(!client_id.is_empty());
        assert!(client_id.starts_with("client_"));
    }

    #[tokio::test]
    async fn test_websocket_protocol_creation() {
        let config = Config::default();
        let protocol = WebSocketProtocol::new(config);
        
        assert!(!protocol.connected.load(Ordering::Relaxed));
        assert!(!protocol.is_audio_channel_open().await);
        assert!(protocol.get_session_id().await.is_none());
    }
} 