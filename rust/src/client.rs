use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

use crate::types::{DeviceState, ListeningMode, Result, ClientError};
use crate::config::Config;
use crate::websocket::{WebSocketProtocol, WebSocketEvent};
use crate::voice::{MicrophoneOpusRecorder, NodeAudioPlayer};

/// 客户端状态变化回调类型
pub type StateChangeCallback = Arc<dyn Fn(DeviceState) + Send + Sync>;

/// 小智客户端
pub struct Client {
    config: Config,
    protocol: Arc<Mutex<WebSocketProtocol>>,
    recorder: Arc<Mutex<Option<MicrophoneOpusRecorder>>>,
    player: Arc<Mutex<NodeAudioPlayer>>,
    device_state: Arc<Mutex<DeviceState>>,
    keep_listening: Arc<AtomicBool>,
    aborted: Arc<AtomicBool>,
    is_recording_from_mic: Arc<AtomicBool>,
    
    // 回调函数
    pub on_state_changed: Option<StateChangeCallback>,
}

unsafe impl Send for Client {}
unsafe impl Sync for Client {}

impl Client {
    /// 创建新的客户端实例
    pub fn new(config: Config) -> Result<Self> {
        let protocol = WebSocketProtocol::new(config.clone());
        let mut player = NodeAudioPlayer::new(
            config.audio.output_sample_rate,
            config.audio.channels
        )?;

        // 设置播放完成回调
        player.set_playback_finished_callback(|| {
            tracing::debug!("🔇 音频播放完成回调被触发");
        });

        Ok(Self {
            config: config.clone(),
            protocol: Arc::new(Mutex::new(protocol)),
            recorder: Arc::new(Mutex::new(None)),
            player: Arc::new(Mutex::new(player)),
            device_state: Arc::new(Mutex::new(DeviceState::Idle)),
            keep_listening: Arc::new(AtomicBool::new(true)),
            aborted: Arc::new(AtomicBool::new(false)),
            is_recording_from_mic: Arc::new(AtomicBool::new(false)),
            on_state_changed: None,
        })
    }

    /// 从配置参数创建客户端
    pub fn from_params(
        websocket_url: String,
        access_token: String,
        device_id: String,
        client_id: String,
    ) -> Result<Self> {
        let config = Config::new(websocket_url, access_token, device_id, client_id);
        Self::new(config)
    }

    /// 设置状态变化回调
    pub fn set_state_change_callback<F>(&mut self, callback: F)
    where
        F: Fn(DeviceState) + Send + Sync + 'static,
    {
        self.on_state_changed = Some(Arc::new(callback));
    }

    /// 开始语音聊天
    pub async fn start_voice_chat(&self) -> Result<()> {
        tracing::info!("🚀 开始语音聊天...");
        self.set_device_state(DeviceState::Connecting);

        // 连接WebSocket
        let event_receiver = {
            let mut protocol_guard = self.protocol.lock().await;
            protocol_guard.connect().await?
        };

        // 启动事件处理
        self.start_event_handling(event_receiver);

        // 设置持续监听
        self.keep_listening.store(true, Ordering::Relaxed);

        // 发送欢迎消息开始对话
        self.send_text_message("hello").await?;

        Ok(())
    }

    /// 启动事件处理
    fn start_event_handling(&self, mut event_receiver: mpsc::UnboundedReceiver<WebSocketEvent>) {
        let player = Arc::clone(&self.player);
        let device_state = Arc::clone(&self.device_state);
        let keep_listening = Arc::clone(&self.keep_listening);
        let aborted = Arc::clone(&self.aborted);
        let callback = self.on_state_changed.clone();
        let client = Arc::new(self.clone());

        tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                // 增加调试日志
                match &event {
                    WebSocketEvent::IncomingAudio(_) => {},
                    e => tracing::info!("[调试] 接收到WebSocket事件: {:?}", e),
                }

                match event {
                    WebSocketEvent::Connected => {
                        tracing::info!("✅ WebSocket 连接成功");
                        let mut state_guard = device_state.lock().await;
                        *state_guard = DeviceState::Idle;
                        if let Some(cb) = &callback {
                            cb(DeviceState::Idle);
                        }
                    }
                    WebSocketEvent::AudioChannelOpened => {
                        tracing::info!("🎵 音频通道已打开");
                    }
                    WebSocketEvent::AudioChannelClosed => {
                        tracing::info!("🔇 音频通道已关闭");
                        let mut state_guard = device_state.lock().await;
                        *state_guard = DeviceState::Idle;
                        if let Some(cb) = &callback {
                            cb(DeviceState::Idle);
                        }
                    }
                    WebSocketEvent::IncomingAudio(audio_data) => {
                        // 处理接收到的音频数据
                        let device_state_guard = device_state.lock().await;
                        if *device_state_guard == DeviceState::Speaking && !audio_data.is_empty() {
                            drop(device_state_guard); // 释放锁
                            let mut player_guard = player.lock().await;
                            let _ = player_guard.process_audio_data(audio_data);
                        }
                    }
                    WebSocketEvent::IncomingJson(json_data) => {
                        Self::handle_incoming_json(json_data, &device_state, &player, &keep_listening, &aborted, &callback, &client).await;
                    }
                    WebSocketEvent::NetworkError(error) => {
                        tracing::error!("❌ 网络错误: {}", error);
                        let mut state_guard = device_state.lock().await;
                        *state_guard = DeviceState::Idle;
                        if let Some(cb) = &callback {
                            cb(DeviceState::Idle);
                        }
                    }
                }
            }
        });
    }

    /// 处理接收到的JSON消息
    async fn handle_incoming_json(
        json_data: serde_json::Value,
        device_state: &Arc<Mutex<DeviceState>>,
        player: &Arc<Mutex<NodeAudioPlayer>>,
        keep_listening: &Arc<AtomicBool>,
        aborted: &Arc<AtomicBool>,
        callback: &Option<StateChangeCallback>,
        client: &Arc<Client>,
    ) {
        tracing::info!("📨 接收到消息: {:?}", json_data);

        let msg_type = json_data.get("type").and_then(|v| v.as_str());

        match msg_type {
            Some("tts") => {
                Self::handle_tts_message(json_data, device_state, player, keep_listening, aborted, callback, client).await;
            }
            Some("stt") => {
                Self::handle_stt_message(json_data);
            }
            Some("llm") => {
                Self::handle_llm_message(json_data);
                
                // 在收到LLM消息后，自动开始监听
                let mut state_guard = device_state.lock().await;
                if *state_guard == DeviceState::Idle && keep_listening.load(Ordering::Relaxed) {
                    *state_guard = DeviceState::Listening;
                    if let Some(cb) = callback {
                        cb(DeviceState::Listening);
                    }
                    tracing::info!("🎤 收到LLM消息后自动开始监听");
                    
                    // 不再需要启动新任务，因为状态变化会在start_event_handling中处理
                }
            }
            Some("error") => {
                if let Some(message) = json_data.get("message").and_then(|v| v.as_str()) {
                    tracing::warn!("⚠️ 服务器错误: {}", message);
                }
            }
            _ => {
                tracing::info!("📋 其他消息: {:?}", json_data);
            }
        }
    }

    /// 处理TTS消息
    async fn handle_tts_message(
        data: serde_json::Value,
        device_state: &Arc<Mutex<DeviceState>>,
        player: &Arc<Mutex<NodeAudioPlayer>>,
        keep_listening: &Arc<AtomicBool>,
        aborted: &Arc<AtomicBool>,
        callback: &Option<StateChangeCallback>,
        client: &Arc<Client>,
    ) {
        let state = data.get("state").and_then(|v| v.as_str());

        match state {
            Some("start") => {
                tracing::info!("🗣️ 开始播放AI回复");
                
                // 在TTS开始时强制停止录音，确保状态一致性
                let was_recording = client.is_recording_from_mic.load(Ordering::Relaxed);
                tracing::debug!("🔍 TTS开始时录音状态检查: is_recording={}", was_recording);
                
                // 不管当前状态如何，都尝试停止录音以确保状态一致性
                tracing::info!("🛑 TTS开始，强制停止录音以确保状态一致性");
                client.stop_microphone_recording().await;
                
                let mut state_guard = device_state.lock().await;
                *state_guard = DeviceState::Speaking;
                if let Some(cb) = callback {
                    cb(DeviceState::Speaking);
                }
                drop(state_guard);
                aborted.store(false, Ordering::Relaxed);
            }
            Some("stop") => {
                tracing::info!("🔇 AI回复播放完成");

                Self::handle_tts_stop(
                    device_state,
                    player,
                    keep_listening,
                    aborted,
                    callback,
                    client,
                )
                .await;
            }
            Some("interrupted") => {
                tracing::info!("⚡ AI回复被打断");
                aborted.store(true, Ordering::Relaxed);
                let mut player_guard = player.lock().await;
                player_guard.stop();
                drop(player_guard);
                
                // AI回复被打断后，如果启用持续监听，则重新开始监听
                if keep_listening.load(Ordering::Relaxed) {
                    let mut state_guard = device_state.lock().await;
                    *state_guard = DeviceState::Listening;
                    if let Some(cb) = callback {
                        cb(DeviceState::Listening);
                    }
                    drop(state_guard);
                    
                    tracing::info!("🎤 AI回复被打断，重新开始监听");
                    if let Err(e) = client.start_listening(ListeningMode::AlwaysOn).await {
                        tracing::error!("AI回复被打断后重新启动监听失败: {}", e);
                    }
                }
            }
            _ => {
                tracing::debug!("TTS其他状态: {:?}", state);
            }
        }
    }

    /// 处理TTS停止
    async fn handle_tts_stop(
        device_state: &Arc<Mutex<DeviceState>>,
        player: &Arc<Mutex<NodeAudioPlayer>>,
        keep_listening: &Arc<AtomicBool>,
        aborted: &Arc<AtomicBool>,
        callback: &Option<StateChangeCallback>,
        client: &Arc<Client>,
    ) {
        // 等待音频播放完成
        Self::wait_for_audio_playback_complete(player).await;

        let mut state_guard = device_state.lock().await;
        if !aborted.load(Ordering::Relaxed) {
            // 根据是否启用持续监听来决定下一个状态
            let should_start_listening = keep_listening.load(Ordering::Relaxed);
            tracing::debug!("🔍 TTS停止检查: keep_listening={}", should_start_listening);
            
            let next_state = if should_start_listening {
                DeviceState::Listening  // TTS停止后直接切换到监听状态
            } else {
                DeviceState::Idle       // 如果没有启用持续监听，则切换到空闲状态
            };

            *state_guard = next_state;
            if let Some(cb) = callback {
                cb(next_state);
            }

            if next_state == DeviceState::Listening {
                tracing::info!("🎤 TTS播放完成，自动切换到监听状态");
                drop(state_guard); // 释放锁
                
                // 实际启动监听功能，使用 AlwaysOn 模式保持持续监听
                if let Err(e) = client.start_listening(ListeningMode::AlwaysOn).await {
                    tracing::error!("TTS停止后自动启动监听失败: {}", e);
                } else {
                    tracing::info!("✅ TTS停止后成功启动监听");
                }
            } else {
                tracing::info!("💤 TTS播放完成，切换到空闲状态");
            }
        }
    }

    /// 等待音频播放完成
    async fn wait_for_audio_playback_complete(player: &Arc<Mutex<NodeAudioPlayer>>) {
        let check_interval = Duration::from_millis(50);
        let max_wait_time = Duration::from_secs(30);
        let start_time = std::time::Instant::now();

        loop {
                    let is_playing = {
            let player_guard = player.lock().await;
            player_guard.is_playing()
        };

            if !is_playing || start_time.elapsed() > max_wait_time {
                break;
            }

            tokio::time::sleep(check_interval).await;
        }

        tracing::debug!("🔇 音频播放完成检查结束");
    }

    /// 处理STT消息
    fn handle_stt_message(data: serde_json::Value) {
        if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
            tracing::info!("🎤 语音识别结果: {}", text);
        }
    }

    /// 处理LLM消息
    fn handle_llm_message(data: serde_json::Value) {
        if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
            tracing::info!("💬 AI回复内容: {}", content);
        }
    }

    /// 设置设备状态
    fn set_device_state(&self, new_state: DeviceState) {
        let device_state = Arc::clone(&self.device_state);
        let callback = self.on_state_changed.clone();
        
        tokio::spawn(async move {
            let mut state_guard = device_state.lock().await;
            let old_state = *state_guard;
            if old_state != new_state {
                *state_guard = new_state;
                
                tracing::debug!("📱 设备状态变化: {:?} -> {:?}", old_state, new_state);
                
                // 调用状态变化回调
                if let Some(callback) = &callback {
                    callback(new_state);
                }
            }
        });
    }

    /// 发送文本消息
    pub async fn send_text_message(&self, text: &str) -> Result<()> {
        tracing::info!("✉️  发送文本消息: {}", text);
        let session_id = {
            let protocol_guard = self.protocol.lock().await;
            protocol_guard.get_session_id().await
        };

        if let Some(session_id) = session_id {
            let message = serde_json::json!({
                "session_id": session_id,
                "type": "listen",
                "state": "detect",
                "text": text
            });

            // 发送消息
            {
                let mut protocol_guard = self.protocol.lock().await;

                if !protocol_guard.is_connected() {
                    // 如果未连接，则先连接
                    tracing::info!("WebSocket未连接，正在重新连接...");
                    protocol_guard.connect().await?;
                }

                let message_text = serde_json::to_string(&message)?;
                protocol_guard.send_text(&message_text).await?;
                tracing::info!("[调试] 文本消息已发送: {}", message_text);
            }

            // 设置为持续监听模式并开始监听
            if let Err(e) = self.start_listening(ListeningMode::AlwaysOn).await {
                tracing::error!("自动开始监听失败: {}", e);
            }
        } else {
            tracing::warn!("⚠️ 未获取到Session ID，无法发送消息");
        }

        Ok(())
    }

    /// 开始监听
    pub async fn start_listening(&self, mode: ListeningMode) -> Result<()> {
        // 检查当前录音状态
        let is_currently_recording = self.is_recording_from_mic.load(Ordering::Relaxed);
        let current_state = {
            let state_guard = self.device_state.lock().await;
            *state_guard
        };
        
        tracing::debug!("🔍 开始监听检查: is_recording={}, current_state={:?}, mode={:?}", 
                       is_currently_recording, current_state, mode);
        
        // 如果已有录音在运行，则先停止，确保状态干净
        if is_currently_recording {
            tracing::info!("🛑 检测到已有录音，先停止旧的录音以确保状态干净 (state={:?})", current_state);
            self.stop_microphone_recording().await;
            
            // 停止后再次检查状态
            let new_recording_state = self.is_recording_from_mic.load(Ordering::Relaxed);
            tracing::debug!("🔍 停止录音后状态检查: is_recording={}", new_recording_state);
        }

        tracing::info!("🎤 开始监听，模式: {:?}", mode);

        // 发送消息前，确保WebSocket是连接状态
        let mut protocol_guard = self.protocol.lock().await;
        if !protocol_guard.is_connected() {
            tracing::info!("WebSocket未连接，正在重新连接...");
            let new_event_receiver = protocol_guard.connect().await?;
            // 重新连接后，需要重新启动事件处理循环
            self.start_event_handling(new_event_receiver);
        }
        
        // 获取 Session ID，如果不存在则使用空字符串
        let session_id = protocol_guard.get_session_id().await.unwrap_or_else(|| {
            tracing::warn!("⚠️ 未获取到Session ID，将使用空字符串");
            String::new()
        });

        // 构造 listen:start 消息
        let mode_str = match mode {
            ListeningMode::AlwaysOn => "realtime",
            ListeningMode::AutoStop => "auto",
            ListeningMode::Manual => "manual",
        };
        let message = serde_json::json!({
            "session_id": session_id,
            "type": "listen",
            "state": "start",
            "mode": mode_str
        });

        // 发送消息
        let message_text = serde_json::to_string(&message)?;
        protocol_guard.send_text(&message_text).await?;
        tracing::debug!("🎤 发送 listen:start 消息: {}", message_text);

        self.set_device_state(DeviceState::Listening);

        // 设置监听模式
        match mode {
            ListeningMode::AlwaysOn => {
                self.keep_listening.store(true, Ordering::Relaxed);
            }
            ListeningMode::AutoStop => {
                self.keep_listening.store(false, Ordering::Relaxed);
            }
            ListeningMode::Manual => {
                self.keep_listening.store(false, Ordering::Relaxed);
            }
        }

        // 开始麦克风录音
        self.start_microphone_recording().await?;

        Ok(())
    }

    /// 停止监听
    pub async fn stop_listening(&self) -> Result<()> {
        if !self.is_recording_from_mic.load(Ordering::Relaxed) {
            return Ok(());
        }

        tracing::info!("🛑 停止监听");
        
        // 停止麦克风录音
        self.stop_microphone_recording().await;
        
        // 设置状态
        self.set_device_state(DeviceState::Idle);
        
        // 关闭音频通道
        {
            let mut protocol_guard = self.protocol.lock().await;
            protocol_guard.close_audio_channel().await?;
        }

        Ok(())
    }

    /// 开始麦克风录音
    async fn start_microphone_recording(&self) -> Result<()> {
        // 创建录音器
        let mut recorder = MicrophoneOpusRecorder::new(
            self.config.audio.input_sample_rate,
            self.config.audio.channels,
            ((self.config.audio.frame_duration as usize) * (self.config.audio.input_sample_rate as usize)) / 1000,
        )?;

        // 开始录音
        let opus_receiver = recorder.start_recording()?;

        // 保存录音器实例
        {
            let mut recorder_guard = self.recorder.lock().await;
            *recorder_guard = Some(recorder);
        }

        self.is_recording_from_mic.store(true, Ordering::Relaxed);

        tracing::info!("🎤 麦克风录音已启动，状态标志已设置为true");

        // 启动音频数据处理任务
        let protocol = Arc::clone(&self.protocol);
        let is_recording = Arc::clone(&self.is_recording_from_mic);
        
        tokio::spawn(async move {
            let mut receiver = opus_receiver;
            
            // 添加任务开始日志
            tracing::debug!("🎤 音频数据处理任务开始");
            
            while let Some(opus_data) = receiver.recv().await {
                if !is_recording.load(Ordering::Relaxed) {
                    tracing::debug!("🎤 检测到录音停止标志，退出音频处理任务");
                    break;
                }

                // 发送音频数据到服务器
                let should_send = {
                    let protocol_guard = protocol.lock().await;
                    protocol_guard.is_audio_channel_open().await
                };

                if should_send {
                    let mut protocol_guard = protocol.lock().await;
                    if let Err(e) = protocol_guard.send_audio(opus_data).await {
                        tracing::error!("发送音频数据失败: {}", e);
                        break;
                    }
                } else {
                    tracing::debug!("🎤 音频通道未开启，跳过发送音频数据");
                }
            }
            
            // 任务结束时确保录音状态被重置
            is_recording.store(false, Ordering::Relaxed);
            tracing::debug!("🎤 音频数据处理任务结束，录音状态已重置为false");
        });

        Ok(())
    }

    /// 停止麦克风录音
    async fn stop_microphone_recording(&self) {
        let was_recording = self.is_recording_from_mic.load(Ordering::Relaxed);
        
        // 立即设置录音状态为false，防止新的音频数据被处理
        self.is_recording_from_mic.store(false, Ordering::Relaxed);
        tracing::debug!("🎤 录音状态已立即设置为false");
        
        let mut recorder_guard = self.recorder.lock().await;
        if let Some(ref mut recorder) = recorder_guard.as_mut() {
            tracing::debug!("🎤 正在停止录音器...");
            recorder.stop_recording();
            tracing::debug!("🎤 已调用recorder.stop_recording()");
        } else {
            tracing::debug!("🎤 录音器已为None，无需停止");
        }
        *recorder_guard = None;
        
        // 给音频数据处理任务一些时间来响应停止信号
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        tracing::info!("🎤 麦克风录音已停止，状态标志从{}变为false", was_recording);
    }

    /// 停止语音聊天
    pub async fn stop_voice_chat(&self) -> Result<()> {
        tracing::info!("🛑 停止语音聊天...");
        
        self.keep_listening.store(false, Ordering::Relaxed);
        
        if let Err(e) = self.stop_listening().await {
            tracing::warn!("停止监听时出错: {}", e);
        }
        
        Ok(())
    }

    /// 打断当前对话
    pub async fn interrupt_conversation(&self) -> Result<()> {
        tracing::info!("⚡ 打断当前对话");
        
        let current_state = self.get_device_state().await;
        
        match current_state {
                         DeviceState::Speaking => {
                 // 停止音频播放
                 {
                     let mut player_guard = self.player.lock().await;
                     player_guard.stop();
                 }
                
                // 设置打断标志
                self.aborted.store(true, Ordering::Relaxed);
                
                // 发送打断消息
                let interrupt_message = serde_json::json!({
                    "type": "interrupt",
                    "timestamp": chrono::Utc::now().timestamp_millis()
                });
                
                                 {
                     let mut protocol_guard = self.protocol.lock().await;
                     let message_text = serde_json::to_string(&interrupt_message)?;
                     protocol_guard.send_text(&message_text).await?;
                 }
                
                // 开始新的监听
                if self.keep_listening.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    self.start_listening(ListeningMode::AlwaysOn).await?;
                }
            }
            DeviceState::Listening => {
                // 如果正在监听，重新开始监听
                if let Err(e) = self.stop_listening().await {
                    tracing::warn!("停止监听失败: {}", e);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                self.start_listening(ListeningMode::AlwaysOn).await?;
            }
            _ => {
                tracing::debug!("当前状态不需要打断: {:?}", current_state);
            }
        }
        
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&self) -> Result<()> {
        tracing::info!("🔌 断开连接...");
        
        // 停止语音聊天
        if let Err(e) = self.stop_voice_chat().await {
            tracing::warn!("停止语音聊天失败: {}", e);
        }
        
        // 关闭协议连接
        {
            let mut protocol_guard = self.protocol.lock().await;
            protocol_guard.destroy();
        }
        
        // 设置空闲状态
        self.set_device_state(DeviceState::Idle);
        
        tracing::info!("✅ 连接已断开");
        Ok(())
    }

    /// 获取当前设备状态
    pub async fn get_device_state(&self) -> DeviceState {
        let state_guard = self.device_state.lock().await;
        *state_guard
    }

    /// 检查是否正在录音
    pub fn is_recording(&self) -> bool {
        self.is_recording_from_mic.load(Ordering::Relaxed)
    }

    /// 检查是否持续监听
    pub fn is_keep_listening(&self) -> bool {
        self.keep_listening.load(Ordering::Relaxed)
    }
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            protocol: Arc::clone(&self.protocol),
            recorder: Arc::clone(&self.recorder),
            player: Arc::clone(&self.player),
            device_state: Arc::clone(&self.device_state),
            keep_listening: Arc::clone(&self.keep_listening),
            aborted: Arc::clone(&self.aborted),
            is_recording_from_mic: Arc::clone(&self.is_recording_from_mic),
            on_state_changed: self.on_state_changed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = Config::default();
        let client = Client::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_client_from_params() {
        let client = Client::from_params(
            "ws://localhost:8080".to_string(),
            "test-token".to_string(),
            "test-device".to_string(),
            "test-client".to_string(),
        );
        assert!(client.is_ok());
    }
}