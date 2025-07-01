use cpal::{Device, Stream, StreamConfig, SampleRate};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use opus::{Decoder, Channels};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tokio::time;
use crate::types::{Result, ClientError};

/// 音频播放器
pub struct NodeAudioPlayer {
    decoder: Arc<Mutex<Decoder>>,
    audio_buffer: Arc<Mutex<VecDeque<Vec<i16>>>>,
    _stream: Option<Stream>,
    is_playing: Arc<AtomicBool>,
    sample_rate: u32,
    channels: u16,
    frame_size: usize,
    buffer_size: usize,
    max_buffer_size: usize,
    last_data_time: Arc<Mutex<Instant>>,
    playback_finished_callback: Option<Box<dyn Fn() + Send + Sync>>,
}

unsafe impl Send for NodeAudioPlayer {}
unsafe impl Sync for NodeAudioPlayer {}

impl NodeAudioPlayer {
    /// 创建新的音频播放器
    /// 
    /// # Arguments
    /// * `sample_rate` - 采样率（Hz）
    /// * `channels` - 声道数
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self> {
        let opus_channels = match channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => return Err(ClientError::AudioError("不支持的声道数".to_string())),
        };

        let decoder = Decoder::new(sample_rate, opus_channels)?;
        let frame_size = (sample_rate as f64 * 0.02) as usize; // 20ms frames

        Ok(Self {
            decoder: Arc::new(Mutex::new(decoder)),
            audio_buffer: Arc::new(Mutex::new(VecDeque::new())),
            _stream: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            sample_rate,
            channels,
            frame_size,
            buffer_size: 10,        // 初始缓冲帧数
            max_buffer_size: 100,   // 最大缓冲区限制
            last_data_time: Arc::new(Mutex::new(Instant::now())),
            playback_finished_callback: None,
        })
    }

    /// 获取默认输出设备
    pub fn get_default_output_device() -> Result<Device> {
        let host = cpal::default_host();
        host.default_output_device()
            .ok_or_else(|| ClientError::AudioError("未找到默认输出设备".to_string()))
    }

    /// 设置播放完成回调
    pub fn set_playback_finished_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.playback_finished_callback = Some(Box::new(callback));
    }

    /// 处理音频数据
    pub fn process_audio_data(&mut self, opus_data: Vec<u8>) -> Result<()> {
        if opus_data.is_empty() {
            tracing::warn!("🔊 收到空的Opus数据");
            return Ok(());
        }

        // 解码Opus数据
        let pcm_data = {
            let mut decoder_guard = self.decoder.lock().map_err(|_| {
                ClientError::AudioError("获取解码器锁失败".to_string())
            })?;

            let mut output_buffer = vec![0i16; self.frame_size];
            
            // 尝试解码，如果失败则生成静音
            match decoder_guard.decode(&opus_data, &mut output_buffer, false) {
                Ok(len) => {
                    output_buffer.truncate(len);
                    if len == 0 {
                        tracing::warn!("🔊 解码返回0长度，生成静音帧");
                        // 生成静音帧保持连续性
                        vec![0i16; self.frame_size]
                    } else {
                        output_buffer
                    }
                }
                Err(e) => {
                    tracing::error!("🔊 Opus解码失败: {}, 生成静音帧", e);
                    // 解码失败时生成静音帧，避免音频中断
                    vec![0i16; self.frame_size]
                }
            }
        };

        // 添加到缓冲区
        {
            let mut buffer_guard = self.audio_buffer.lock().map_err(|_| {
                ClientError::AudioError("获取音频缓冲区锁失败".to_string())
            })?;

            // 检查缓冲区是否过满
            if buffer_guard.len() >= self.max_buffer_size {
                tracing::warn!("🔊 音频缓冲区过满，丢弃旧数据");
                // 丢弃一些旧数据，但不要一次丢弃太多
                let drop_count = std::cmp::min(5, buffer_guard.len() - self.max_buffer_size + 1);
                for _ in 0..drop_count {
                    buffer_guard.pop_front();
                }
            }

            buffer_guard.push_back(pcm_data);
        }

        // 更新最后接收数据时间
        {
            let mut last_time_guard = self.last_data_time.lock().map_err(|_| {
                ClientError::AudioError("获取时间锁失败".to_string())
            })?;
            *last_time_guard = Instant::now();
        }

        // 如果还没开始播放且缓冲区足够大，开始播放
        if !self.is_playing.load(Ordering::Relaxed) {
            let buffer_len = {
                let buffer_guard = self.audio_buffer.lock().map_err(|_| {
                    ClientError::AudioError("获取音频缓冲区锁失败".to_string())
                })?;
                buffer_guard.len()
            };

            if buffer_len >= self.buffer_size {
                self.start_playback()?;
            }
        }

        Ok(())
    }

    /// 开始播放
    pub fn start_playback(&mut self) -> Result<()> {
        if self.is_playing.load(Ordering::Relaxed) {
            return Ok(());
        }

        let device = Self::get_default_output_device()?;
        let config = self.get_optimal_config(&device)?;

        tracing::info!(
            "🔊 开始音频播放: 设备={:?}, 采样率={}Hz, 声道={}, 缓冲区大小={:?}",
            device.name().unwrap_or_default(),
            config.sample_rate.0,
            config.channels,
            config.buffer_size
        );

        let audio_buffer = Arc::clone(&self.audio_buffer);
        let is_playing = Arc::clone(&self.is_playing);
        let last_data_time = Arc::clone(&self.last_data_time);

        let stream = self.create_stream::<f32>(&device, &config, audio_buffer, is_playing, last_data_time)?;

        stream.play().map_err(|e| ClientError::AudioError(format!("启动音频流失败: {}", e)))?;
        
        self._stream = Some(stream);
        self.is_playing.store(true, Ordering::Relaxed);

        // 启动播放监控
        self.start_playback_monitor();

        tracing::info!("✅ 音频播放已开始");
        Ok(())
    }

    /// 获取最佳配置
    fn get_optimal_config(&self, device: &Device) -> Result<StreamConfig> {
        let supported_configs = device.supported_output_configs()
            .map_err(|e| ClientError::AudioError(format!("获取支持的配置失败: {}", e)))?;

        // 尝试找到匹配的配置
        for supported_config in supported_configs {
            let sample_rate_range = supported_config.min_sample_rate().0..=supported_config.max_sample_rate().0;
            if sample_rate_range.contains(&self.sample_rate) {
                let config = StreamConfig {
                    channels: self.channels,
                    sample_rate: SampleRate(self.sample_rate),
                    buffer_size: cpal::BufferSize::Fixed(self.frame_size as u32),
                };
                return Ok(config);
            }
        }

        // 使用默认配置
        let _default_config = device.default_output_config()
            .map_err(|e| ClientError::AudioError(format!("获取默认配置失败: {}", e)))?;

        Ok(StreamConfig {
            channels: self.channels,
            sample_rate: SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.frame_size as u32),
        })
    }

    /// 创建音频流
    fn create_stream<T>(
        &self,
        device: &Device,
        config: &StreamConfig,
        audio_buffer: Arc<Mutex<VecDeque<Vec<i16>>>>,
        is_playing: Arc<AtomicBool>,
        last_data_time: Arc<Mutex<Instant>>,
    ) -> Result<Stream>
    where
        T: cpal::Sample + cpal::SizedSample,
        T: cpal::FromSample<i16>,
    {
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    if !is_playing.load(Ordering::Relaxed) {
                        // 如果没有播放，填充静音
                        for sample in data.iter_mut() {
                            *sample = cpal::Sample::from_sample(0i16);
                        }
                        return;
                    }

                    let mut buffer_guard = match audio_buffer.lock() {
                        Ok(guard) => guard,
                        Err(_) => {
                            // 锁失败，填充静音
                            for sample in data.iter_mut() {
                                *sample = cpal::Sample::from_sample(0i16);
                            }
                            return;
                        }
                    };

                    let mut output_index = 0;
                    
                    // 从缓冲区取出数据并播放
                    while output_index < data.len() && !buffer_guard.is_empty() {
                        if let Some(pcm_frame) = buffer_guard.front_mut() {
                            let remaining_output = data.len() - output_index;
                            let samples_to_copy = std::cmp::min(pcm_frame.len(), remaining_output);

                            // 复制数据到输出缓冲区
                            for i in 0..samples_to_copy {
                                data[output_index + i] = cpal::Sample::from_sample(pcm_frame[i]);
                            }

                            output_index += samples_to_copy;

                            if samples_to_copy == pcm_frame.len() {
                                // 整个帧都被消费了，移除它
                                buffer_guard.pop_front();
                            } else {
                                // 只消费了部分数据，从帧头部移除已消费的部分
                                // 保留未消费的数据供下次使用
                                pcm_frame.drain(0..samples_to_copy);
                                break; // 输出缓冲区已满，保留剩余数据
                            }
                        } else {
                            break;
                        }
                    }

                    // 如果还有剩余空间，填充静音
                    for i in output_index..data.len() {
                        data[i] = cpal::Sample::from_sample(0i16);
                    }

                    // 如果缓冲区为空，检查是否应该停止播放
                    if buffer_guard.is_empty() {
                        if let Ok(last_time_guard) = last_data_time.lock() {
                            if last_time_guard.elapsed() > Duration::from_millis(500) {
                                is_playing.store(false, Ordering::Relaxed);
                            }
                        }
                    }
                },
                |err| {
                    tracing::error!("音频播放流错误: {}", err);
                },
                None,
            )
            .map_err(|e| ClientError::AudioError(format!("创建音频播放流失败: {}", e)))?;

        Ok(stream)
    }

    /// 启动播放监控
    fn start_playback_monitor(&self) {
        let is_playing = Arc::clone(&self.is_playing);
        let last_data_time = Arc::clone(&self.last_data_time);
        let audio_buffer = Arc::clone(&self.audio_buffer);
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(100));
            
            loop {
                interval.tick().await;
                
                if !is_playing.load(Ordering::Relaxed) {
                    break;
                }

                // 检查是否长时间没有数据
                let should_stop = {
                    if let Ok(last_time_guard) = last_data_time.lock() {
                        if let Ok(buffer_guard) = audio_buffer.lock() {
                            buffer_guard.is_empty() && last_time_guard.elapsed() > Duration::from_millis(1000)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                if should_stop {
                    is_playing.store(false, Ordering::Relaxed);
                    tracing::info!("🔇 播放监控检测到播放结束");
                    break;
                }
            }
        });
    }

    /// 停止播放
    pub fn stop(&mut self) {
        if self.is_playing.load(Ordering::Relaxed) {
            self.is_playing.store(false, Ordering::Relaxed);
            self._stream = None;
            
            // 清空缓冲区
            if let Ok(mut buffer_guard) = self.audio_buffer.lock() {
                buffer_guard.clear();
            }

            // 调用完成回调
            if let Some(callback) = &self.playback_finished_callback {
                callback();
            }
            
            tracing::info!("🛑 音频播放已停止");
        }
    }

    /// 检查是否正在播放
    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    /// 获取缓冲区状态
    pub fn get_buffer_status(&self) -> (usize, usize) {
        if let Ok(buffer_guard) = self.audio_buffer.lock() {
            (buffer_guard.len(), self.max_buffer_size)
        } else {
            (0, self.max_buffer_size)
        }
    }

    /// 获取音频配置
    pub fn get_audio_config(&self) -> (u32, u16, usize) {
        (self.sample_rate, self.channels, self.frame_size)
    }
}

impl Drop for NodeAudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_creation() {
        let player = NodeAudioPlayer::new(24000, 1);
        assert!(player.is_ok());
    }

    #[test]
    fn test_invalid_channels() {
        let player = NodeAudioPlayer::new(24000, 5);
        assert!(player.is_err());
    }
} 