use crate::types::{ClientError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleRate, Stream, StreamConfig};
use opus::{Application, Channels, Encoder};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// 音频配置信息
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub min_sample_rate: u32,
    pub max_sample_rate: u32,
    pub channels: u16,
    pub format: String,
}

/// 设备状态信息
#[derive(Debug)]
pub struct DeviceStatus {
    pub name: String,
    pub is_available: bool,
    pub supported_configs: Vec<AudioConfig>,
    pub current_config: AudioConfig,
}

/// 麦克风Opus录音器
pub struct MicrophoneOpusRecorder {
    _stream: Option<Stream>,
    encoder: Arc<Mutex<Encoder>>,
    sample_rate: u32,
    channels: u16,
    frame_size: usize,
    is_recording: Arc<AtomicBool>,
    opus_sender: Option<mpsc::UnboundedSender<Vec<u8>>>,
}

unsafe impl Send for MicrophoneOpusRecorder {}
unsafe impl Sync for MicrophoneOpusRecorder {}

impl MicrophoneOpusRecorder {
    /// 创建新的录音器
    ///
    /// # Arguments
    /// * `target_sample_rate` - 目标采样率（Hz）
    /// * `target_channels` - 目标声道数
    /// * `_frame_size` - 帧大小（样本数）
    pub fn new(target_sample_rate: u32, target_channels: u16, _frame_size: usize) -> Result<Self> {
        let device = Self::get_default_input_device()?;

        // 获取最佳配置
        let (config, actual_sample_rate, actual_channels) =
            Self::get_optimal_config(&device, target_sample_rate, target_channels)?;

        tracing::info!(
            "🎤 创建录音器: 目标配置({}Hz, {}声道) -> 实际配置({}Hz, {}声道)",
            target_sample_rate,
            target_channels,
            actual_sample_rate,
            actual_channels
        );

        // 创建Opus编码器（固定使用16kHz采样率和单声道）
        let encoder = Encoder::new(16000, Channels::Mono, Application::Voip)?;

        Ok(Self {
            _stream: None,
            encoder: Arc::new(Mutex::new(encoder)),
            sample_rate: actual_sample_rate,
            channels: actual_channels,
            frame_size: (actual_sample_rate as f64 * 0.02) as usize, // 20ms frames
            is_recording: Arc::new(AtomicBool::new(false)),
            opus_sender: None,
        })
    }

    /// 获取可用的音频设备列表
    pub fn get_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| ClientError::AudioError(format!("获取输入设备失败: {}", e)))?;

        let device_names: Result<Vec<String>> = devices
            .map(|device| {
                device
                    .name()
                    .map_err(|e| ClientError::AudioError(format!("获取设备名称失败: {}", e)))
            })
            .collect();

        device_names
    }

    /// 获取默认输入设备
    pub fn get_default_input_device() -> Result<Device> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| ClientError::AudioError("未找到默认输入设备".to_string()))?;

        // 打印设备信息
        tracing::info!("🎤 默认输入设备: {}", device.name().unwrap_or_default());

        // 获取支持的配置
        if let Ok(supported_configs) = device.supported_input_configs() {
            tracing::info!("支持的输入配置:");
            for config in supported_configs {
                tracing::info!(
                    "- 采样率范围: {:?}-{:?}Hz",
                    config.min_sample_rate().0,
                    config.max_sample_rate().0
                );
                tracing::info!("  声道数: {}", config.channels());
                tracing::info!("  采样格式: {:?}", config.sample_format());
            }
        }

        Ok(device)
    }

    /// 获取最佳设备配置
    fn get_optimal_config(
        device: &Device,
        target_sample_rate: u32,
        target_channels: u16,
    ) -> Result<(StreamConfig, u32, u16)> {
        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| ClientError::AudioError(format!("获取支持的配置失败: {}", e)))?;

        // 存储所有可用配置
        let mut available_configs = Vec::new();
        for config in supported_configs {
            available_configs.push((
                config.min_sample_rate().0,
                config.max_sample_rate().0,
                config.channels(),
                config.sample_format(),
            ));
        }

        // 如果没有可用配置，返回错误
        if available_configs.is_empty() {
            return Err(ClientError::AudioError(
                "设备没有可用的音频配置".to_string(),
            ));
        }

        // 记录设备支持的配置
        tracing::info!("📊 设备支持的配置:");
        for (min_rate, max_rate, channels, format) in &available_configs {
            tracing::info!(
                "- 采样率: {}-{}Hz, 声道: {}, 格式: {:?}",
                min_rate,
                max_rate,
                channels,
                format
            );
        }

        // 优先级排序：采样率优先级
        let preferred_rates = [48000, 44100, 16000, 8000];
        let mut selected_rate = None;
        let mut selected_channels = None;

        // 1. 首先尝试找到最接近目标配置的组合
        for &(min_rate, max_rate, channels, _) in &available_configs {
            if target_sample_rate >= min_rate && target_sample_rate <= max_rate {
                selected_rate = Some(target_sample_rate);
                selected_channels = Some(channels);
                break;
            }
        }

        // 2. 如果没找到完全匹配的，查找最接近的配置
        if selected_rate.is_none() {
            for &rate in &preferred_rates {
                for &(min_rate, max_rate, channels, _) in &available_configs {
                    if rate >= min_rate && rate <= max_rate {
                        selected_rate = Some(rate);
                        selected_channels = Some(channels);
                        break;
                    }
                }
                if selected_rate.is_some() {
                    break;
                }
            }
        }

        // 3. 如果还是没找到，使用设备支持的最接近的配置
        if selected_rate.is_none() {
            let (min_rate, max_rate, channels, _) = available_configs[0];
            selected_rate = Some(if target_sample_rate <= min_rate {
                min_rate
            } else if target_sample_rate >= max_rate {
                max_rate
            } else {
                target_sample_rate
            });
            selected_channels = Some(channels);
        }

        let actual_rate = selected_rate.unwrap();
        let actual_channels = selected_channels.unwrap();

        // 计算最佳的缓冲区大小
        let buffer_size = (actual_rate as f32 * 0.02) as usize; // 20ms帧

        tracing::info!(
            "✅ 选择的音频配置: 采样率={}Hz, 声道={}, 缓冲区大小={}",
            actual_rate,
            actual_channels,
            buffer_size
        );

        let config = StreamConfig {
            channels: actual_channels,
            sample_rate: SampleRate(actual_rate),
            buffer_size: cpal::BufferSize::Fixed(buffer_size as u32),
        };

        Ok((config, actual_rate, actual_channels))
    }

    /// 检查设备状态
    pub fn check_device_status(&self) -> Result<DeviceStatus> {
        let device = Self::get_default_input_device()?;

        let status = DeviceStatus {
            name: device
                .name()
                .map_err(|e| ClientError::AudioError(format!("获取设备名称失败: {}", e)))?,
            is_available: true,
            supported_configs: self.get_supported_configs(&device)?,
            current_config: self.get_current_config(),
        };

        Ok(status)
    }

    /// 获取支持的配置列表
    fn get_supported_configs(&self, device: &Device) -> Result<Vec<AudioConfig>> {
        let mut configs = Vec::new();

        for config in device
            .supported_input_configs()
            .map_err(|e| ClientError::AudioError(format!("获取支持的输入配置失败: {}", e)))?
        {
            configs.push(AudioConfig {
                min_sample_rate: config.min_sample_rate().0,
                max_sample_rate: config.max_sample_rate().0,
                channels: config.channels(),
                format: format!("{:?}", config.sample_format()),
            });
        }

        Ok(configs)
    }

    /// 获取当前配置
    fn get_current_config(&self) -> AudioConfig {
        AudioConfig {
            min_sample_rate: self.sample_rate,
            max_sample_rate: self.sample_rate,
            channels: self.channels,
            format: "f32".to_string(),
        }
    }

    /// 开始录音
    ///
    /// # Returns
    /// * `mpsc::UnboundedReceiver<Vec<u8>>` - Opus数据接收器
    pub fn start_recording(&mut self) -> Result<mpsc::UnboundedReceiver<Vec<u8>>> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Err(ClientError::AudioError("录音已在进行中".to_string()));
        }

        let device = match Self::get_default_input_device() {
            Ok(device) => device,
            Err(e) => {
                tracing::error!("获取默认输入设备失败: {}", e);
                return Err(e);
            }
        };

        let config = match MicrophoneOpusRecorder::get_optimal_config(
            &device,
            self.sample_rate,
            self.channels,
        ) {
            Ok((config, _, _)) => config,
            Err(e) => {
                tracing::error!("获取音频配置失败: {}", e);
                return Err(e);
            }
        };

        tracing::info!(
            "🎤 开始录音: 设备={:?}, 采样率={}Hz, 声道={}, 缓冲区大小={:?}",
            device.name().unwrap_or_default(),
            config.sample_rate.0,
            config.channels,
            config.buffer_size
        );

        let (opus_sender, opus_receiver) = mpsc::unbounded_channel();
        self.opus_sender = Some(opus_sender.clone());

        let encoder = Arc::clone(&self.encoder);
        let is_recording = Arc::clone(&self.is_recording);
        let frame_size = self.frame_size;
        let expected_sample_rate = self.sample_rate;

        let stream = match self.create_stream::<f32>(
            &device,
            &config,
            encoder,
            is_recording.clone(),
            opus_sender,
            frame_size,
            expected_sample_rate,
        ) {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("创建音频流失败: {}", e);
                return Err(e);
            }
        };

        if let Err(e) = stream.play() {
            tracing::error!("启动音频流失败: {}", e);
            return Err(ClientError::AudioError(format!("启动音频流失败: {}", e)));
        }

        self._stream = Some(stream);
        self.is_recording.store(true, Ordering::Relaxed);

        tracing::info!("✅ 录音已开始");
        Ok(opus_receiver)
    }

    /// 尝试重新连接设备
    pub async fn try_reconnect(&mut self) -> Result<bool> {
        let max_attempts = 3;
        let mut attempt = 0;

        while attempt < max_attempts {
            attempt += 1;
            tracing::info!("尝试重新连接音频设备 (尝试 {}/{})", attempt, max_attempts);

            match self.start_recording() {
                Ok(_) => {
                    tracing::info!("✅ 设备重新连接成功");
                    return Ok(true);
                }
                Err(e) => {
                    tracing::warn!("❌ 重连尝试失败: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        Err(ClientError::AudioError("设备重连失败".to_string()))
    }

    /// 创建音频流
    fn create_stream<T>(
        &self,
        device: &Device,
        config: &StreamConfig,
        encoder: Arc<Mutex<Encoder>>,
        is_recording: Arc<AtomicBool>,
        opus_sender: mpsc::UnboundedSender<Vec<u8>>,
        frame_size: usize,
        _expected_sample_rate: u32,
    ) -> Result<Stream>
    where
        T: cpal::Sample + cpal::SizedSample,
        f32: cpal::FromSample<T>,
    {
        let err_fn = |err| {
            tracing::error!("❌ 音频流错误: {}", err);
        };

        let channels = config.channels;
        let actual_sample_rate = config.sample_rate.0;

        // Opus编码器固定使用16kHz采样率和单声道
        let opus_sample_rate = 16000;
        let opus_channels = 1;

        // 根据Opus的采样率和20ms帧时长计算正确的帧大小
        let opus_frame_size = (opus_sample_rate as usize * 20 / 1000) * opus_channels;
        tracing::debug!("🎤 计算Opus帧大小: {}", opus_frame_size);

        // 创建一个线程安全的采样缓冲区
        let sample_buffer = Arc::new(Mutex::new(Vec::with_capacity(
            frame_size * channels as usize * 2,
        )));
        let sample_buffer_clone = Arc::clone(&sample_buffer);

        let stream = device
            .build_input_stream(
                config,
                move |data: &[T], _: &cpal::InputCallbackInfo| {
                    if !is_recording.load(Ordering::Relaxed) {
                        return;
                    }

                    // 将输入数据转换为f32
                    let input: Vec<f32> = data.iter().map(|&sample| sample.to_sample()).collect();

                    // 如果需要，进行采样率转换
                    let resampled = if actual_sample_rate != opus_sample_rate {
                        Self::resample(&input, actual_sample_rate, opus_sample_rate)
                    } else {
                        input
                    };

                    // 如果是双声道，转换为单声道
                    let mono_samples = if channels == 2 {
                        let mut mono = Vec::with_capacity(resampled.len() / 2);
                        for chunk in resampled.chunks(2) {
                            if chunk.len() == 2 {
                                mono.push((chunk[0] + chunk[1]) * 0.5);
                            }
                        }
                        mono
                    } else {
                        resampled
                    };

                    // 将处理后的样本添加到缓冲区
                    if let Ok(mut buffer) = sample_buffer_clone.lock() {
                        buffer.extend(mono_samples);

                        // 当缓冲区达到Opus帧大小时进行编码
                        while buffer.len() >= opus_frame_size {
                            let frame: Vec<f32> = buffer.drain(..opus_frame_size).collect();

                            // 编码音频帧
                            if let Ok(mut encoder) = encoder.lock() {
                                // 创建足够大的输出缓冲区
                                let mut output_buffer = vec![0u8; opus_frame_size * 4];
                                match encoder.encode_float(&frame, &mut output_buffer) {
                                    Ok(encoded_len) => {
                                        let encoded_data = output_buffer[..encoded_len].to_vec();
                                        if opus_sender.send(encoded_data).is_err() {
                                            tracing::warn!("❌ Opus数据发送失败，可能接收端已关闭");
                                            is_recording.store(false, Ordering::Relaxed);
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("❌ Opus编码失败: {}", e);
                                    }
                                }
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| ClientError::AudioError(format!("创建音频流失败: {}", e)))?;

        Ok(stream)
    }

    /// 重采样音频数据
    fn resample(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
        if input_rate == output_rate {
            return input.to_vec();
        }

        let ratio = output_rate as f32 / input_rate as f32;
        let output_len = (input.len() as f32 * ratio) as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let pos = i as f32 / ratio;
            let pos_floor = pos.floor() as usize;
            let pos_ceil = pos.ceil() as usize;

            if pos_ceil >= input.len() {
                break;
            }

            let fract = pos - pos_floor as f32;

            // 线性插值
            let sample = input[pos_floor] * (1.0 - fract) + input[pos_ceil] * fract;

            // 确保采样值在有效范围内
            let clamped_sample = sample.max(-1.0).min(1.0);
            output.push(clamped_sample);
        }

        output
    }

    /// 停止录音并清理资源
    pub fn stop_recording(&mut self) {
        if self.is_recording.load(Ordering::Relaxed) {
            self.is_recording.store(false, Ordering::Relaxed);
            self._stream = None;
            self.opus_sender = None;
            tracing::info!("🛑 录音已停止");
        }
    }

    /// 检查是否正在录音
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    /// 获取音频配置信息
    pub fn get_audio_config(&self) -> (u32, u16, usize) {
        (self.sample_rate, self.channels, self.frame_size)
    }
}

impl Drop for MicrophoneOpusRecorder {
    fn drop(&mut self) {
        self.stop_recording();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_creation() {
        let recorder = MicrophoneOpusRecorder::new(16000, 1, 320);
        assert!(recorder.is_ok());
    }

    #[test]
    fn test_device_config_adaptation() {
        let recorder = MicrophoneOpusRecorder::new(48000, 2, 960).unwrap();
        let status = recorder.check_device_status().unwrap();

        // 验证设备状态
        assert!(status.is_available);
        assert!(!status.name.is_empty());

        // 验证配置
        let (sample_rate, channels, frame_size) = recorder.get_audio_config();
        assert!(sample_rate > 0);
        assert!(channels > 0);
        assert_eq!(frame_size, (sample_rate as f64 * 0.02) as usize);
    }

    #[test]
    fn test_resample() {
        // 测试采样率转换
        let input = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let resampled = MicrophoneOpusRecorder::resample(&input, 44100, 16000);

        // 验证输出长度近似正确
        let expected_len = (input.len() as f32 * (16000.0 / 44100.0)) as usize;
        assert!((resampled.len() as i32 - expected_len as i32).abs() <= 1);

        // 验证采样值范围
        for sample in resampled {
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_invalid_channels() {
        let recorder = MicrophoneOpusRecorder::new(16000, 5, 320);
        assert!(recorder.is_err());
    }

    #[tokio::test]
    async fn test_recording_lifecycle() {
        let mut recorder = MicrophoneOpusRecorder::new(16000, 1, 320).unwrap();

        // 测试开始录音
        let receiver = recorder.start_recording();
        assert!(receiver.is_ok());
        assert!(recorder.is_recording());

        // 等待一小段时间
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 测试停止录音
        recorder.stop_recording();
        assert!(!recorder.is_recording());
    }
}
