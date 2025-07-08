use std::sync::mpsc;
use std::time::Duration;
use std::io::{self, Write};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tracing::{info, error, debug};
use opus::{Encoder as OpusEncoder, Decoder as OpusDecoder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const OPUS_FRAME_SIZE_MS: u32 = 20; // 20ms 的帧大小
const MAX_PACKET_SIZE: usize = 1500; // 最大包大小

/// 计算音频数据的统计信息
fn calculate_audio_stats(data: &[f32]) -> (f32, f32, f32) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    
    let sum: f32 = data.iter().sum();
    let mean = sum / data.len() as f32;
    
    let max = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let min = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    
    // 计算RMS（均方根）音量
    let rms = (data.iter()
        .map(|&x| x * x)
        .sum::<f32>() / data.len() as f32)
        .sqrt();
        
    (rms, min, max)
}

/// 列出所有输入设备并让用户选择
fn select_input_device(host: &cpal::Host) -> Result<cpal::Device, Box<dyn std::error::Error>> {
    let input_devices = host.input_devices()?;
    let devices: Vec<cpal::Device> = input_devices.collect();
    
    if devices.is_empty() {
        return Err("没有找到任何输入设备".into());
    }

    // 如果只有一个设备,直接返回
    if devices.len() == 1 {
        let device = &devices[0];
        let name = device.name().unwrap_or_else(|_| "未知设备名".to_string());
        info!("✅ 只有一个输入设备,自动选择: {}", name);
        return Ok(device.clone());
    }
    
    println!("\n=== 可用的输入设备 ===");
    for (i, device) in devices.iter().enumerate() {
        let name = device.name().unwrap_or_else(|_| "未知设备名".to_string());
        println!("{}. {}", i + 1, name);
    }
    
    // 添加默认设备选项
    if let Some(default_device) = host.default_input_device() {
        let default_name = default_device.name().unwrap_or_else(|_| "未知设备名".to_string());
        println!("\n默认输入设备: {}", default_name);
    }
    
    print!("\n请选择输入设备 (1-{}, 直接回车使用默认设备): ", devices.len());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let selected_device = if input.trim().is_empty() {
        host.default_input_device()
            .ok_or("未找到默认输入设备")?
    } else {
        let index: usize = input.trim().parse::<usize>()
            .map_err(|_| "无效的设备编号")?
            .checked_sub(1)
            .ok_or("设备编号必须大于0")?;
        
        devices.get(index)
            .ok_or("设备编号超出范围")?
            .clone()
    };
    
    let name = selected_device.name().unwrap_or_else(|_| "未知设备名".to_string());
    info!("✅ 已选择设备: {}", name);
    
    Ok(selected_device)
}

/// 列出所有输出设备并让用户选择
fn select_output_device(host: &cpal::Host) -> Result<cpal::Device, Box<dyn std::error::Error>> {
    let output_devices = host.output_devices()?;
    let devices: Vec<cpal::Device> = output_devices.collect();
    
    if devices.is_empty() {
        return Err("没有找到任何输出设备".into());
    }

    // 如果只有一个设备,直接返回
    if devices.len() == 1 {
        let device = &devices[0];
        let name = device.name().unwrap_or_else(|_| "未知设备名".to_string());
        info!("✅ 只有一个输出设备,自动选择: {}", name);
        return Ok(device.clone());
    }
    
    println!("\n=== 可用的输出设备 ===");
    for (i, device) in devices.iter().enumerate() {
        let name = device.name().unwrap_or_else(|_| "未知设备名".to_string());
        println!("{}. {}", i + 1, name);
    }
    
    // 添加默认设备选项
    if let Some(default_device) = host.default_output_device() {
        let default_name = default_device.name().unwrap_or_else(|_| "未知设备名".to_string());
        println!("\n默认输出设备: {}", default_name);
    }
    
    print!("\n请选择输出设备 (1-{}, 直接回车使用默认设备): ", devices.len());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let selected_device = if input.trim().is_empty() {
        host.default_output_device()
            .ok_or("未找到默认输出设备")?
    } else {
        let index: usize = input.trim().parse::<usize>()
            .map_err(|_| "无效的设备编号")?
            .checked_sub(1)
            .ok_or("设备编号必须大于0")?;
        
        devices.get(index)
            .ok_or("设备编号超出范围")?
            .clone()
    };
    
    let name = selected_device.name().unwrap_or_else(|_| "未知设备名".to_string());
    info!("✅ 已选择设备: {}", name);
    
    Ok(selected_device)
}

/// 自动检测最佳音频配置
fn detect_optimal_config(device: &cpal::Device) -> Result<(u32, u16), Box<dyn std::error::Error>> {
    info!("🔍 正在检测最佳音频配置...");
    
    let supported_configs = device.supported_input_configs()
        .map_err(|e| format!("获取支持的配置失败: {}", e))?
        .collect::<Vec<_>>();
    
    if supported_configs.is_empty() {
        return Err("设备不支持任何音频配置".into());
    }

    // 打印所有支持的配置
    info!("发现 {} 个支持的音频配置:", supported_configs.len());
    for (i, config) in supported_configs.iter().enumerate() {
        info!("配置 {}: 采样率 {}-{} Hz, 声道数: {}", 
            i + 1,
            config.min_sample_rate().0,
            config.max_sample_rate().0,
            config.channels());
    }
    
    // 首选配置：48kHz + 单声道
    if let Some(config) = supported_configs.iter().find(|config| {
        config.min_sample_rate().0 <= 48000 && 
        config.max_sample_rate().0 >= 48000 &&
        config.channels() == 1
    }) {
        info!("✅ 找到理想配置：48kHz + 单声道");
        return Ok((48000, 1));
    }
    
    // 备选配置：44.1kHz + 单声道
    if let Some(config) = supported_configs.iter().find(|config| {
        config.min_sample_rate().0 <= 44100 && 
        config.max_sample_rate().0 >= 44100 &&
        config.channels() == 1
    }) {
        info!("✅ 使用备选配置：44.1kHz + 单声道");
        return Ok((44100, 1));
    }
    
    // 最后选择：使用第一个可用的配置
    let first_config = supported_configs.first()
        .ok_or("未找到任何音频配置")?;
    
    let sample_rate = first_config.min_sample_rate().0;
    let channels = first_config.channels();
    
    info!("⚠️ 使用默认配置：{}Hz + {} 声道", sample_rate, channels);
    Ok((sample_rate, channels))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    info!("🎤 开始麦克风录音测试");

    // 获取主机
    let host = cpal::default_host();
    
    // 选择输入和输出设备
    info!("第一步：选择录音设备");
    let input_device = select_input_device(&host)?;
    
    info!("第二步：选择播放设备");
    let output_device = select_output_device(&host)?;
    
    // 获取所选设备的音频配置
    let (sample_rate, channels) = detect_optimal_config(&input_device)?;
    let frame_size = (sample_rate as f64 * OPUS_FRAME_SIZE_MS as f64 / 1000.0) as usize;
    
    // 创建 Opus 编码器和解码器
    let mut encoder = OpusEncoder::new(
        sample_rate,
        if channels == 1 { opus::Channels::Mono } else { opus::Channels::Stereo },
        opus::Application::Voip
    )?;
    
    let mut decoder = OpusDecoder::new(
        sample_rate,
        if channels == 1 { opus::Channels::Mono } else { opus::Channels::Stereo }
    )?;

    // 设置编码器参数
    encoder.set_bitrate(opus::Bitrate::Bits(64000))?; // 64kbps
    
    // 创建通道用于传输编码后的数据
    let (tx, rx) = mpsc::channel();
    
    // 创建计数器来跟踪处理的帧数
    let frame_counter = Arc::new(AtomicUsize::new(0));
    let frame_counter_clone = frame_counter.clone();
    
    // ===== 第一阶段：录音 =====
    info!("📝 准备开始录音...");
    let mut all_encoded_data = Vec::<Vec<u8>>::new();
    
    let config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Fixed(frame_size as u32),
    };

    // 创建输入流
    let mut input_data = Vec::with_capacity(frame_size * channels as usize);
    let mut opus_output = vec![0u8; MAX_PACKET_SIZE];
    
    let input_stream = input_device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let frame_num = frame_counter_clone.fetch_add(1, Ordering::SeqCst);
            let (rms, min, max) = calculate_audio_stats(data);
            
            debug!(
                "Frame #{}: RMS音量: {:.4}, 最小值: {:.4}, 最大值: {:.4}, 样本数: {}",
                frame_num, rms, min, max, data.len()
            );
            
            input_data.extend_from_slice(data);
            
            // 当收集够一帧数据时进行编码
            if input_data.len() >= frame_size * channels as usize {
                let mut pcm = vec![0f32; frame_size * channels as usize];
                pcm.copy_from_slice(&input_data[..frame_size * channels as usize]);
                input_data.clear();
                
                // 编码
                if let Ok(encoded_len) = encoder.encode_float(&pcm, &mut opus_output) {
                    let encoded_data = opus_output[..encoded_len].to_vec();
                    debug!("编码后数据大小: {} 字节", encoded_len);
                    let _ = tx.send(encoded_data);
                }
            }
        },
        |err| error!("录音错误: {}", err),
        None,
    )?;

    // 开始录音
    input_stream.play()?;
    info!("🎙️ 开始录音（5秒）...");
    
    // 录制5秒钟
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // 停止录音并收集所有编码数据
    drop(input_stream);
    info!("⏹️ 录音结束，共处理 {} 帧音频", frame_counter.load(Ordering::SeqCst));
    
    // 从通道接收所有编码数据
    let mut total_encoded_size = 0;
    while let Ok(data) = rx.try_recv() {
        total_encoded_size += data.len();
        all_encoded_data.push(data);
    }
    info!("📊 录音统计:");
    info!("   - 总编码数据大小: {} 字节", total_encoded_size);
    info!("   - 平均每帧大小: {:.2} 字节", total_encoded_size as f32 / all_encoded_data.len() as f32);
    info!("   - 总帧数: {}", all_encoded_data.len());

    // ===== 第二阶段：播放 =====
    info!("🔊 准备开始播放录音...");
    
    // 使用选择的输出设备
    let mut encoded_data_iter = all_encoded_data.into_iter();
    
    // 创建一个可重用的缓冲区
    let buffer_size = frame_size * channels as usize;
    let mut last_good_frame = vec![0f32; buffer_size];
    let mut consecutive_errors = 0;
    
    let output_stream = output_device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if let Some(encoded_data) = encoded_data_iter.next() {
                // 确保解码缓冲区大小与播放缓冲区匹配
                let mut decoded = vec![0f32; buffer_size];
                
                match decoder.decode_float(&encoded_data, &mut decoded, false) {
                    Ok(_) => {
                        // 应用简单的平滑处理，确保不会越界
                        let len = data.len().min(decoded.len()).min(last_good_frame.len());
                        for i in 1..len {
                            decoded[i] = decoded[i] * 0.9 + last_good_frame[i] * 0.1;
                        }
                        
                        // 更新最后一个好帧并重置错误计数
                        last_good_frame[..len].copy_from_slice(&decoded[..len]);
                        consecutive_errors = 0;
                        
                        // 复制处理后的数据到输出缓冲区
                        let copy_len = data.len().min(decoded.len());
                        data[..copy_len].copy_from_slice(&decoded[..copy_len]);
                        
                        // 如果还有剩余的输出缓冲区，填充静音
                        if copy_len < data.len() {
                            data[copy_len..].fill(0.0);
                        }
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        error!("解码错误: {}, 连续错误次数: {}", e, consecutive_errors);
                        
                        if consecutive_errors < 3 {
                            // 使用上一帧的数据进行平滑过渡
                            let len = data.len().min(last_good_frame.len());
                            for i in 0..len {
                                data[i] = last_good_frame[i] * (1.0 - (consecutive_errors as f32 * 0.3));
                            }
                            // 如果还有剩余的输出缓冲区，填充静音
                            if len < data.len() {
                                data[len..].fill(0.0);
                            }
                        } else {
                            // 太多连续错误，使用静音
                            data.fill(0.0);
                        }
                    }
                }
            } else {
                // 没有更多数据时，渐变至静音
                let len = data.len().min(last_good_frame.len());
                for i in 0..len {
                    data[i] = last_good_frame[i] * 0.5;
                }
                // 如果还有剩余的输出缓冲区，填充静音
                if len < data.len() {
                    data[len..].fill(0.0);
                }
                // 更新last_good_frame
                for x in last_good_frame.iter_mut() {
                    *x *= 0.5;
                }
            }
        },
        |err| error!("播放错误: {}", err),
        None,
    )?;

    // 开始播放
    info!("▶️ 开始播放录音...");
    output_stream.play()?;
    
    // 等待播放完成（等待5秒，与录音时间相同）
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // 确保完全停止播放
    info!("⏹️ 播放完成，正在关闭播放设备...");
    drop(output_stream);
    
    info!("✅ 测试完成!");
    Ok(())
} 