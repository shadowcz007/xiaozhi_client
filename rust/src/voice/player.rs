use cpal::{Device, Stream, StreamConfig, SampleRate};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use opus::{Decoder, Channels};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tokio::time;
use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};
use crate::types::{Result, ClientError};

struct PlayerBuffer {
    deque: VecDeque<f32>,
}

/// 音频播放器
pub struct NodeAudioPlayer {
    decoder: Arc<Mutex<Decoder>>,
    audio_buffer: Arc<Mutex<PlayerBuffer>>,
    _stream: Option<Stream>,
    is_playing: Arc<AtomicBool>,
    stop_receiving: Arc<AtomicBool>, // 新增：停止接收新数据标志
    sample_rate: u32,
    channels: u16,
    frame_size: usize,
    buffer_duration_ms: usize,
    max_buffer_duration_ms: usize,
    last_data_time: Arc<Mutex<Instant>>,
    playback_finished_callback: Option<Box<dyn Fn() + Send + Sync>>,
    debug_counter: Arc<Mutex<usize>>,
    device_channels: u16,
    device_sample_rate: u32,
    resampler: Arc<Mutex<Option<SincFixedIn<f32>>>>,
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

        tracing::info!("🔊 创建新的音频播放器: 采样率={}Hz, 声道={}, 帧大小={}", sample_rate, channels, frame_size);

        let mut player = Self {
            decoder: Arc::new(Mutex::new(decoder)),
            audio_buffer: Arc::new(Mutex::new(PlayerBuffer {
                deque: VecDeque::new(),
            })),
            _stream: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            stop_receiving: Arc::new(AtomicBool::new(false)), // 初始化新标志
            sample_rate,
            channels,
            frame_size,
            buffer_duration_ms: 200,   // 初始缓冲时长（毫秒）
            max_buffer_duration_ms: 2000,  // 最大缓冲时长（毫秒）
            last_data_time: Arc::new(Mutex::new(Instant::now())),
            playback_finished_callback: None,
            debug_counter: Arc::new(Mutex::new(0)),
            device_channels: 2, // 默认值，会在配置时更新
            device_sample_rate: 48000, // 默认值，会在配置时更新
            resampler: Arc::new(Mutex::new(None)),
        };

        // 记录设备状态
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| ClientError::AudioError("未找到输出设备".to_string()))?;
        player.log_device_status(&device)?;

        // 获取设备配置
        let config = player.get_optimal_config(&device)?;
        
        Ok(player)
    }

    /// 获取默认输出设备
    pub fn get_default_output_device() -> Result<Device> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| ClientError::AudioError("未找到默认输出设备".to_string()))?;

        // 打印设备信息
        tracing::info!("🔊 默认输出设备名称: {:?}", device.name());
        
        // 打印支持的配置
        if let Ok(supported_configs) = device.supported_output_configs() {
            tracing::info!("🔊 设备支持的配置:");
            for config in supported_configs {
                tracing::info!("  - 采样率范围: {}Hz - {}Hz", 
                    config.min_sample_rate().0,
                    config.max_sample_rate().0
                );
                tracing::info!("  - 声道数: {}", config.channels());
                tracing::info!("  - 采样格式: {:?}", config.sample_format());
            }
        }

        // 打印默认配置
        if let Ok(default_config) = device.default_output_config() {
            tracing::info!("🔊 设备默认配置:");
            tracing::info!("  - 采样率: {}Hz", default_config.sample_rate().0);
            tracing::info!("  - 声道数: {}", default_config.channels());
            tracing::info!("  - 采样格式: {:?}", default_config.sample_format());
        }

        Ok(device)
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
        // 步骤 1: 记录输入的 Opus 数据信息
        tracing::info!("🎵 [步骤1] 接收到 Opus 数据: 大小={} 字节, 目标采样率={}Hz, 目标声道={}", 
            opus_data.len(), self.sample_rate, self.channels);

        // 如果设置了停止接收标志，直接返回
        if self.stop_receiving.load(Ordering::Relaxed) {
            tracing::debug!("🔊 已停止接收新音频数据");
            return Ok(());
        }

        if opus_data.is_empty() {
            tracing::warn!("🔊 收到空的Opus数据");
            return Ok(());
        }

        // 步骤 2: Opus 解码为 PCM
        let pcm_data = {
            let mut decoder_guard = self.decoder.lock().map_err(|_| {
                ClientError::AudioError("获取解码器锁失败".to_string())
            })?;

            let mut output_buffer = vec![0i16; self.frame_size];
            
            match decoder_guard.decode(&opus_data, &mut output_buffer, false) {
                Ok(len) => {
                    output_buffer.truncate(len);
                    tracing::info!("🎵 [步骤2] Opus解码完成: PCM长度={}, 采样率={}Hz", 
                        len, self.sample_rate);
                    if len == 0 {
                        tracing::warn!("🔊 解码返回0长度，生成静音帧");
                        vec![0i16; self.frame_size]
                    } else {
                        output_buffer
                    }
                }
                Err(e) => {
                    tracing::error!("🔊 Opus解码失败: {}, 生成静音帧", e);
                    vec![0i16; self.frame_size]
                }
            }
        };

        // 步骤 3: PCM (i16) 转换为 f32
        let pcm_f32: Vec<f32> = pcm_data.iter()
            .map(|&x| x as f32 / 32768.0)
            .collect();
        
        tracing::info!("🎵 [步骤3] PCM转换为f32: 数据点数={}, 最大值={:.3}, 最小值={:.3}", 
            pcm_f32.len(),
            pcm_f32.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)),
            pcm_f32.iter().fold(f32::INFINITY, |a, &b| a.min(b)));

        // 步骤 4: 重采样处理
        let resampled_data = {
            let mut resampler_guard = self.resampler.lock().map_err(|_| {
                ClientError::AudioError("获取重采样器锁失败".to_string())
            })?;

            // 如果重采样器还没有初始化，创建一个新的
            if resampler_guard.is_none() {
                let params = InterpolationParameters {
                    sinc_len: 512,
                    f_cutoff: 0.95,
                    interpolation: InterpolationType::Cubic,
                    oversampling_factor: 512,
                    window: WindowFunction::Blackman,
                };
                
                tracing::info!("🎵 创建重采样器: {}Hz -> {}Hz, sinc_len={}, oversampling={}", 
                    self.sample_rate, self.device_sample_rate, 
                    params.sinc_len, params.oversampling_factor);

                *resampler_guard = Some(SincFixedIn::<f32>::new(
                    self.device_sample_rate as f64 / self.sample_rate as f64,
                    2.0,
                    params,
                    self.frame_size,
                    1, // 单声道处理
                ).map_err(|e| ClientError::AudioError(format!("创建重采样器失败: {}", e)))?);
            }

            let resampler = resampler_guard.as_mut().unwrap();
            let waves_in = vec![pcm_f32.clone()];
            let waves_out = resampler.process(&waves_in, None).map_err(|e| {
                ClientError::AudioError(format!("重采样处理失败: {}", e))
            })?;

            let resampled = waves_out.into_iter().next().unwrap_or_default();
            tracing::info!("🎵 [步骤4] 重采样完成: 输入={} ({}Hz) -> 输出={} ({}Hz), 比率={:.2}", 
                pcm_f32.len(), self.sample_rate,
                resampled.len(), self.device_sample_rate,
                self.device_sample_rate as f32 / self.sample_rate as f32);
            
            resampled
        };

        // 步骤 5: 声道转换（单声道到多声道）
        let mut multi_channel_data = Vec::with_capacity(resampled_data.len() * self.device_channels as usize);
        for sample in resampled_data.iter() {
            // 复制同一个样本到所有声道
            for _ in 0..self.device_channels {
                multi_channel_data.push(*sample);
            }
        }

        tracing::info!("🎵 [步骤5] 声道转换完成: {} -> {} 声道, 最终数据点数={}", 
            1, self.device_channels, multi_channel_data.len());

        // 添加到缓冲区
        {
            let mut buffer_guard = self.audio_buffer.lock().map_err(|_| {
                ClientError::AudioError("获取音频缓冲区锁失败".to_string())
            })?;

            let max_buffer_samples = (self.device_sample_rate as usize * self.device_channels as usize * self.max_buffer_duration_ms) / 1000;
            let current_buffer_samples = buffer_guard.deque.len();

            tracing::info!("📊 缓冲区状态: 已缓冲 {} / {} 样本 ({} / {} ms)",
                current_buffer_samples, max_buffer_samples,
                (current_buffer_samples * 1000) / (self.device_sample_rate as usize * self.device_channels as usize).max(1),
                self.max_buffer_duration_ms
            );

            // 检查缓冲区是否过满
            if current_buffer_samples >= max_buffer_samples {
                tracing::warn!("⚠️ 音频缓冲区过满，丢弃旧数据以腾出空间");
                let drain_count = current_buffer_samples - max_buffer_samples + multi_channel_data.len();
                let len = buffer_guard.deque.len();
                buffer_guard.deque.drain(..std::cmp::min(drain_count, len));
            }

            buffer_guard.deque.extend(multi_channel_data);
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
            let buffer_len_ms = {
                let buffer_guard = self.audio_buffer.lock().map_err(|_| {
                    ClientError::AudioError("获取音频缓冲区锁失败".to_string())
                })?;
                (buffer_guard.deque.len() * 1000) / (self.device_sample_rate as usize * self.device_channels as usize).max(1)
            };

            if buffer_len_ms >= self.buffer_duration_ms {
                tracing::info!("▶️ 缓冲区已满足播放条件 ({} / {} ms), 开始播放",
                    buffer_len_ms, self.buffer_duration_ms);
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
    fn get_optimal_config(&mut self, device: &Device) -> Result<StreamConfig> {
        let default_config = device.default_output_config()
            .map_err(|e| ClientError::AudioError(format!("获取默认配置失败: {}", e)))?;
        
        tracing::info!("🔊 音频设备信息:");
        tracing::info!("  设备名称: {}", device.name()
            .map_err(|e| ClientError::AudioError(format!("获取设备名称失败: {}", e)))?);
        tracing::info!("  默认配置: 采样率={}Hz, 声道数={}, 格式={:?}", 
            default_config.sample_rate().0,
            default_config.channels(),
            default_config.sample_format());

        // 获取所有支持的配置
        if let Ok(supported_configs) = device.supported_output_configs() {
            tracing::info!("  支持的配置:");
            for config in supported_configs {
                tracing::info!("    - 采样率: {}Hz-{}Hz, 声道={}, 格式={:?}",
                    config.min_sample_rate().0,
                    config.max_sample_rate().0,
                    config.channels(),
                    config.sample_format());
            }
        }

        // 更新设备配置
        self.device_channels = default_config.channels();
        self.device_sample_rate = default_config.sample_rate().0;

        Ok(StreamConfig {
            channels: default_config.channels(),
            sample_rate: default_config.sample_rate(),
            buffer_size: cpal::BufferSize::Fixed(self.frame_size as u32),
        })
    }

    fn log_device_status(&self, device: &Device) -> Result<()> {
        tracing::info!("🎵 音频设备状态:");
        tracing::info!("  名称: {}", device.name()
            .map_err(|e| ClientError::AudioError(format!("获取设备名称失败: {}", e)))?);
        tracing::info!("  当前采样率: {}Hz", self.device_sample_rate);
        tracing::info!("  当前声道数: {}", self.device_channels);
        
        if let Ok(supported_configs) = device.supported_output_configs() {
            tracing::info!("  支持的配置:");
            for config in supported_configs {
                tracing::info!("    - 采样率: {}Hz-{}Hz, 声道={}, 格式={:?}",
                    config.min_sample_rate().0,
                    config.max_sample_rate().0,
                    config.channels(),
                    config.sample_format());
            }
        }
        Ok(())
    }

    /// 开始优雅停止
    pub fn start_graceful_stop(&mut self) {
        tracing::info!("🔊 开始优雅停止播放器：停止接收新数据，等待缓冲区播放完毕");
        self.stop_receiving.store(true, Ordering::Relaxed);
    }

    /// 创建音频流
    fn create_stream<T>(
        &self,
        device: &Device,
        config: &StreamConfig,
        audio_buffer: Arc<Mutex<PlayerBuffer>>,
        is_playing: Arc<AtomicBool>,
        last_data_time: Arc<Mutex<Instant>>,
    ) -> Result<Stream>
    where
        T: cpal::Sample + cpal::SizedSample,
        T: cpal::FromSample<f32>,
    {
        let debug_counter = Arc::clone(&self.debug_counter);
        let stop_receiving = Arc::clone(&self.stop_receiving);
        
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    let mut counter = debug_counter.lock().unwrap();
                    *counter += 1;
                    
                    if *counter % 100 == 0 {
                        tracing::debug!("🔊 音频回调 #{}: 请求数据长度={}", counter, data.len());
                    }

                    if !is_playing.load(Ordering::Relaxed) {
                        tracing::debug!("🔊 播放器未激活，输出静音");
                        for sample in data.iter_mut() {
                            *sample = cpal::Sample::from_sample(0.0f32);
                        }
                        return;
                    }

                    let mut buffer_guard = match audio_buffer.lock() {
                        Ok(guard) => guard,
                        Err(_) => {
                            tracing::error!("🔊 获取音频缓冲区锁失败");
                            for sample in data.iter_mut() {
                                *sample = cpal::Sample::from_sample(0.0f32);
                            }
                            return;
                        }
                    };

                    if *counter % 100 == 0 {
                        tracing::debug!("🔊 缓冲区状态: {} 样本", buffer_guard.deque.len());
                    }

                    let samples_to_write = std::cmp::min(data.len(), buffer_guard.deque.len());

                    // 从缓冲区复制数据
                    for (i, sample) in buffer_guard.deque.drain(..samples_to_write).enumerate() {
                        data[i] = cpal::Sample::from_sample(sample);
                    }

                    // 填充剩余空间为静音
                    for sample in data.iter_mut().skip(samples_to_write) {
                        *sample = cpal::Sample::from_sample(0.0f32);
                    }

                    if buffer_guard.deque.is_empty() {
                        if let Ok(last_time_guard) = last_data_time.lock() {
                            // 如果停止接收新数据，缓冲区为空时立即停止
                            if stop_receiving.load(Ordering::Relaxed) {
                                tracing::info!("🔊 缓冲区已清空，停止播放");
                                is_playing.store(false, Ordering::Relaxed);
                            } else if last_time_guard.elapsed() > Duration::from_millis(500) {
                                // 原有逻辑：超时停止
                                tracing::info!("🔊 缓冲区为空且超时，停止播放");
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
                            buffer_guard.deque.is_empty() && last_time_guard.elapsed() > Duration::from_millis(1000)
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
        // 🔧 移除了错误的 `if is_playing` 判断。
        // 无论当前状态如何，调用 stop() 都必须彻底清理资源。
        self.is_playing.store(false, Ordering::Relaxed);

        // 使用 take() 来消耗并丢弃音频流，确保回调停止
        if let Some(stream) = self._stream.take() {
            if let Err(e) = stream.pause() {
                tracing::warn!("暂停音频流失败: {}", e);
            }
            tracing::info!("🔊 音频流已暂停和释放");
        }

        // 清空缓冲区
        if let Ok(mut buffer_guard) = self.audio_buffer.lock() {
            buffer_guard.deque.clear();
        }

        // 重置停止接收标志，为下次播放做准备
        self.stop_receiving.store(false, Ordering::Relaxed);

        // 调用完成回调
        if let Some(callback) = &self.playback_finished_callback {
            callback();
        }

        tracing::info!("🛑 音频播放已完全停止");
    }

    /// 检查是否正在播放
    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    /// 获取缓冲区状态
    pub fn get_buffer_status(&self) -> (usize, usize) {
        if let Ok(buffer_guard) = self.audio_buffer.lock() {
            let current_samples = buffer_guard.deque.len();
            let max_samples = (self.device_sample_rate as usize * self.device_channels as usize * self.max_buffer_duration_ms) / 1000;
            (current_samples, max_samples)
        } else {
            (0, 0)
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