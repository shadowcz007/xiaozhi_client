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

/// 音频播放器
pub struct NodeAudioPlayer {
    decoder: Arc<Mutex<Decoder>>,
    audio_buffer: Arc<Mutex<VecDeque<Vec<f32>>>>, // 改为f32类型
    _stream: Option<Stream>,
    is_playing: Arc<AtomicBool>,
    stop_receiving: Arc<AtomicBool>, // 新增：停止接收新数据标志
    sample_rate: u32,
    channels: u16,
    frame_size: usize,
    buffer_size: usize,
    max_buffer_size: usize,
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

        Ok(Self {
            decoder: Arc::new(Mutex::new(decoder)),
            audio_buffer: Arc::new(Mutex::new(VecDeque::new())),
            _stream: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            stop_receiving: Arc::new(AtomicBool::new(false)), // 初始化新标志
            sample_rate,
            channels,
            frame_size,
            buffer_size: 10,        // 初始缓冲帧数
            max_buffer_size: 100,   // 最大缓冲区限制
            last_data_time: Arc::new(Mutex::new(Instant::now())),
            playback_finished_callback: None,
            debug_counter: Arc::new(Mutex::new(0)),
            device_channels: 2, // 默认值，会在配置时更新
            device_sample_rate: 48000, // 默认值，会在配置时更新
            resampler: Arc::new(Mutex::new(None)),
        })
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
        // 如果设置了停止接收标志，直接返回
        if self.stop_receiving.load(Ordering::Relaxed) {
            tracing::debug!("🔊 已停止接收新音频数据");
            return Ok(());
        }

        if opus_data.is_empty() {
            tracing::warn!("🔊 收到空的Opus数据");
            return Ok(());
        }

        tracing::debug!("🔊 接收到Opus数据: 长度={}", opus_data.len());

        // 解码Opus数据
        let pcm_data = {
            let mut decoder_guard = self.decoder.lock().map_err(|_| {
                ClientError::AudioError("获取解码器锁失败".to_string())
            })?;

            let mut output_buffer = vec![0i16; self.frame_size];
            
            match decoder_guard.decode(&opus_data, &mut output_buffer, false) {
                Ok(len) => {
                    output_buffer.truncate(len);
                    tracing::debug!("🔊 Opus解码成功: PCM长度={}", len);
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

        // 将i16转换为f32
        let pcm_f32: Vec<f32> = pcm_data.iter()
            .map(|&x| x as f32 / 32768.0)
            .collect();

        // 重采样处理
        let resampled_data = {
            let mut resampler_guard = self.resampler.lock().map_err(|_| {
                ClientError::AudioError("获取重采样器锁失败".to_string())
            })?;

            // 如果重采样器还没有初始化，创建一个新的
            if resampler_guard.is_none() {
                let params = InterpolationParameters {
                    sinc_len: 256,
                    f_cutoff: 0.95,
                    interpolation: InterpolationType::Linear,
                    oversampling_factor: 256,
                    window: WindowFunction::BlackmanHarris2,
                };
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

            waves_out.into_iter().next().unwrap_or_default()
        };

        tracing::debug!(
            "🔊 重采样: 输入长度={}, 输出长度={}, 比率={:.2}",
            pcm_f32.len(),
            resampled_data.len(),
            self.device_sample_rate as f32 / self.sample_rate as f32
        );

        // 声道转换（单声道到多声道）
        let mut multi_channel_data = Vec::with_capacity(resampled_data.len() * self.device_channels as usize);
        for sample in resampled_data.iter() {
            // 复制同一个样本到所有声道
            for _ in 0..self.device_channels {
                multi_channel_data.push(*sample);
            }
        }

        // 添加到缓冲区
        {
            let mut buffer_guard = self.audio_buffer.lock().map_err(|_| {
                ClientError::AudioError("获取音频缓冲区锁失败".to_string())
            })?;

            let buffer_len = buffer_guard.len();
            tracing::debug!("🔊 当前缓冲区状态: 已使用={}/{}", buffer_len, self.max_buffer_size);

            // 检查缓冲区是否过满
            if buffer_len >= self.max_buffer_size {
                tracing::warn!("🔊 音频缓冲区过满，丢弃旧数据");
                // 丢弃一些旧数据，但不要一次丢弃太多
                let drop_count = std::cmp::min(5, buffer_len - self.max_buffer_size + 1);
                for _ in 0..drop_count {
                    buffer_guard.pop_front();
                }
            }

            buffer_guard.push_back(multi_channel_data);
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
        let default_config = device.default_output_config()
            .map_err(|e| ClientError::AudioError(format!("获取默认配置失败: {}", e)))?;
        
        // 更新设备配置
        if let Some(self_mut) = unsafe { (self as *const Self as *mut Self).as_mut() } {
            self_mut.device_channels = default_config.channels();
            self_mut.device_sample_rate = default_config.sample_rate().0;
        }

        tracing::info!("🔊 使用设备配置:");
        tracing::info!("  - 采样率: {}Hz", default_config.sample_rate().0);
        tracing::info!("  - 声道数: {}", default_config.channels());
        tracing::info!("  - 采样格式: {:?}", default_config.sample_format());

        Ok(StreamConfig {
            channels: default_config.channels(),
            sample_rate: default_config.sample_rate(),
            buffer_size: cpal::BufferSize::Fixed(self.frame_size as u32),
        })
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
        audio_buffer: Arc<Mutex<VecDeque<Vec<f32>>>>,
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
                        tracing::debug!("🔊 缓冲区状态: 数据帧数={}", buffer_guard.len());
                    }

                    let mut output_index = 0;
                    let mut samples_copied = 0;
                    
                    while output_index < data.len() {
                        if buffer_guard.is_empty() {
                            break;
                        }

                        let frame = buffer_guard.front_mut().unwrap(); // safe due to check
                        let remaining_output = data.len() - output_index;
                        let samples_to_copy = std::cmp::min(frame.len(), remaining_output);

                        for i in 0..samples_to_copy {
                            data[output_index + i] = cpal::Sample::from_sample(frame[i]);
                        }

                        output_index += samples_to_copy;
                        samples_copied += samples_to_copy;

                        // Remove the copied part
                        if samples_to_copy == frame.len() {
                            // Frame fully consumed
                            buffer_guard.pop_front();
                        } else {
                            // Frame partially consumed, remove the copied part from the front
                            // This is not super efficient for a Vec, but it's correct.
                            // A VecDeque for the frame itself would be better.
                            frame.drain(0..samples_to_copy);
                        }
                    }

                    if *counter % 100 == 0 {
                        tracing::debug!("🔊 已复制采样点: {}/{}", samples_copied, data.len());
                    }

                    // 填充剩余空间为静音
                    for i in output_index..data.len() {
                        data[i] = cpal::Sample::from_sample(0.0f32);
                    }

                    if buffer_guard.is_empty() {
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