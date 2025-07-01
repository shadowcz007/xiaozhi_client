use cpal::{Device, Stream, StreamConfig, SampleRate};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use opus::{Encoder, Application, Channels};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use crate::types::{Result, ClientError};

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
    /// * `sample_rate` - 采样率（Hz）
    /// * `channels` - 声道数
    /// * `frame_size` - 帧大小（样本数）
    pub fn new(sample_rate: u32, channels: u16, _frame_size: usize) -> Result<Self> {
        // 确保采样率是 Opus 支持的值
        let opus_sample_rate = match sample_rate {
            rate if rate <= 8000 => 8000,
            rate if rate <= 12000 => 12000,
            rate if rate <= 16000 => 16000,
            rate if rate <= 24000 => 24000,
            _ => 48000,
        };

        let device = Self::get_default_input_device()?;
        
        // 获取支持的配置
        let supported_configs = device.supported_input_configs()
            .map_err(|e| ClientError::AudioError(format!("获取支持的配置失败: {}", e)))?;
        
        // 查找最接近请求配置的支持配置
        let mut selected_config = None;
        let mut min_rate_diff = u32::MAX;
        
        for config in supported_configs {
            if config.channels() == channels {
                let min_rate = config.min_sample_rate().0;
                let max_rate = config.max_sample_rate().0;
                
                // 检查采样率是否在支持范围内
                if sample_rate >= min_rate && sample_rate <= max_rate {
                    selected_config = Some(config);
                    break;
                }
                
                // 找到最接近的采样率
                let diff_min = sample_rate.abs_diff(min_rate);
                let diff_max = sample_rate.abs_diff(max_rate);
                let diff = diff_min.min(diff_max);
                
                if diff < min_rate_diff {
                    min_rate_diff = diff;
                    selected_config = Some(config);
                }
            }
        }
        
        let config = selected_config
            .ok_or_else(|| ClientError::AudioError("未找到支持的音频配置".to_string()))?;
        
        // 调整采样率到支持的范围
        let actual_sample_rate = if sample_rate < config.min_sample_rate().0 {
            config.min_sample_rate().0
        } else if sample_rate > config.max_sample_rate().0 {
            config.max_sample_rate().0
        } else {
            sample_rate
        };
        
        tracing::info!("使用音频配置: 采样率={}Hz (Opus采样率={}Hz), 声道数={}, 格式={:?}", 
            actual_sample_rate, opus_sample_rate, channels, config.sample_format());
        
        let opus_channels = match channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => return Err(ClientError::AudioError("不支持的声道数".to_string())),
        };

        let encoder = Encoder::new(opus_sample_rate, opus_channels, Application::Audio)?;
        let frame_size = (opus_sample_rate as f64 * 0.02) as usize; // 20ms frames

        Ok(Self {
            _stream: None,
            encoder: Arc::new(Mutex::new(encoder)),
            sample_rate: actual_sample_rate,
            channels,
            frame_size,
            is_recording: Arc::new(AtomicBool::new(false)),
            opus_sender: None,
        })
    }

    /// 获取可用的音频设备列表
    pub fn get_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let devices = host.input_devices()
            .map_err(|e| ClientError::AudioError(format!("获取输入设备失败: {}", e)))?;
        
        let device_names: Result<Vec<String>> = devices
            .map(|device| device.name().map_err(|e| ClientError::AudioError(format!("获取设备名称失败: {}", e))))
            .collect();
            
        device_names
    }

    /// 获取默认输入设备
    pub fn get_default_input_device() -> Result<Device> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| ClientError::AudioError("未找到默认输入设备".to_string()))?;
        
        // 打印设备信息
        tracing::info!("🎤 默认输入设备: {}", device.name().unwrap_or_default());
        
        // 获取支持的配置
        if let Ok(supported_configs) = device.supported_input_configs() {
            tracing::info!("支持的输入配置:");
            for config in supported_configs {
                tracing::info!("- 采样率范围: {:?}-{:?}Hz", 
                    config.min_sample_rate().0,
                    config.max_sample_rate().0);
                tracing::info!("  声道数: {}", config.channels());
                tracing::info!("  采样格式: {:?}", config.sample_format());
            }
        }
        
        Ok(device)
    }

    /// 开始录音
    /// 
    /// # Returns
    /// * `mpsc::UnboundedReceiver<Vec<u8>>` - Opus数据接收器
    pub fn start_recording(&mut self) -> Result<mpsc::UnboundedReceiver<Vec<u8>>> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Err(ClientError::AudioError("录音已在进行中".to_string()));
        }

        let device = Self::get_default_input_device()?;
        let config = self.get_optimal_config(&device)?;

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

        let stream = self.create_stream::<f32>(&device, &config, encoder, is_recording, opus_sender, frame_size, expected_sample_rate)?;

        stream.play().map_err(|e| ClientError::AudioError(format!("启动音频流失败: {}", e)))?;
        
        self._stream = Some(stream);
        self.is_recording.store(true, Ordering::Relaxed);
        
        tracing::info!("✅ 录音已开始");
        Ok(opus_receiver)
    }

    /// 获取最佳配置
    fn get_optimal_config(&self, device: &Device) -> Result<StreamConfig> {
        let supported_configs = device.supported_input_configs()
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

        // 如果没有匹配的，使用默认配置并修改参数
        let _default_config = device.default_input_config()
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
            tracing::error!("音频流错误: {}", err);
        };

        let mut sample_buffer = Vec::with_capacity(frame_size * config.channels as usize * 2);
        let channels = config.channels;
        let actual_sample_rate = config.sample_rate.0;

        // Opus编码器总是以特定的采样率工作，如 8, 12, 16, 24, 48 kHz
        // 我们在 `new` 方法中已经将编码器设置为 16000 Hz
        let opus_sample_rate = 16000;

        // 根据Opus的采样率和20ms帧时长计算正确的帧大小
        let opus_frame_size = (opus_sample_rate as usize * 20 / 1000) * channels as usize;
        tracing::debug!("🎤 Opus frame size calculated: {}", opus_frame_size);

        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::Relaxed) {
                    return;
                }

                // 将输入数据转换为f32
                let input: Vec<f32> = data.iter().map(|&sample| sample.to_sample()).collect();

                // 如果需要，进行采样率转换
                let processed_samples = if actual_sample_rate != opus_sample_rate {
                    Self::resample(&input, actual_sample_rate, opus_sample_rate)
                } else {
                    input
                };

                // 将样本添加到缓冲区
                sample_buffer.extend(processed_samples);

                // 当缓冲区达到Opus帧大小时进行编码
                while sample_buffer.len() >= opus_frame_size {
                    let frame: Vec<f32> = sample_buffer.drain(..opus_frame_size).collect();
                    
                    // 编码音频帧
                    if let Ok(mut encoder) = encoder.lock() {
                        // 创建足够大的输出缓冲区
                        let mut output_buffer = vec![0u8; opus_frame_size * 4]; // 넉넉하게 할당
                        if let Ok(encoded_len) = encoder.encode_float(&frame, &mut output_buffer) {
                            let encoded_data = output_buffer[..encoded_len].to_vec();
                            if opus_sender.send(encoded_data).is_err() {
                                // 如果发送失败，可能接收端已经关闭，停止录音
                                tracing::warn!("🎤 Opus数据发送失败，可能接收端已关闭，停止录音。");
                                is_recording.store(false, Ordering::Relaxed);
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
            let fract = pos - pos_floor as f32;

            if pos_ceil >= input.len() {
                break;
            }

            // 线性插值
            let sample = input[pos_floor] * (1.0 - fract) + input[pos_ceil] * fract;
            output.push(sample);
        }

        output
    }

    /// 停止录音
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
    fn test_invalid_channels() {
        let recorder = MicrophoneOpusRecorder::new(16000, 5, 320);
        assert!(recorder.is_err());
    }
} 