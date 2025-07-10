use xiaozhi_client::voice::player::NodeAudioPlayer;
use opus::{Encoder as OpusEncoder, Channels};
use std::time::Duration;
use tracing::info;
use tokio::time;

const TEST_DURATION_SECS: u64 = 10; // 测试时长
const SAMPLE_RATE: u32 = 48000;     // 采样率
const CHANNELS: u16 = 1;            // 单声道
const FRAME_SIZE_MS: u32 = 20;      // 20ms 的帧大小

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("🔊 开始音频播放器测试");
    info!("===================");

    // 创建音频播放器
    let mut player = NodeAudioPlayer::new(SAMPLE_RATE, CHANNELS)?;
    
    // 获取音频配置
    let (actual_sample_rate, actual_channels, frame_size) = player.get_audio_config();
    info!("\n📊 音频配置:");
    info!("   采样率: {}Hz", actual_sample_rate);
    info!("   声道数: {}", actual_channels);
    info!("   帧大小: {} 样本", frame_size);
    info!("   帧时长: {:.1}ms", (frame_size as f32 / actual_sample_rate as f32) * 1000.0);

    // 创建 Opus 编码器用于生成测试数据
    let mut encoder = OpusEncoder::new(
        SAMPLE_RATE,
        if CHANNELS == 1 { Channels::Mono } else { Channels::Stereo },
        opus::Application::Voip
    )?;

    // 设置编码器参数
    encoder.set_bitrate(opus::Bitrate::Bits(64000))?; // 64kbps

    // 生成测试音频数据（正弦波）
    let frequency: f32 = 440.0; // A4音符的频率
    let frame_size = (SAMPLE_RATE as f32 * FRAME_SIZE_MS as f32 / 1000.0) as usize;
    let mut phase: f32 = 0.0;
    let mut opus_output = vec![0u8; 1500]; // 最大Opus包大小

    info!("\n🎵 开始播放测试音频...");
    
    // 设置播放完成回调
    player.set_playback_finished_callback(|| {
        info!("✨ 播放完成回调被触发");
    });

    // 生成并播放音频
    let start_time = std::time::Instant::now();
    let mut total_frames = 0;
    let mut total_bytes = 0;

    while start_time.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
        // 生成正弦波
        let mut pcm_data = Vec::with_capacity(frame_size);
        for _ in 0..frame_size {
            let sample = (phase * 2.0 * std::f32::consts::PI).sin() * 0.5;
            pcm_data.push(sample);
            phase += frequency / SAMPLE_RATE as f32;
            if phase >= 1.0 {
                phase -= 1.0;
            }
        }

        // 编码音频
        if let Ok(encoded_len) = encoder.encode_float(&pcm_data, &mut opus_output) {
            let opus_data = opus_output[..encoded_len].to_vec();
            total_frames += 1;
            total_bytes += encoded_len;

            // 处理音频数据
            player.process_audio_data(opus_data)?;
        }

        // 模拟实际的音频帧率
        time::sleep(Duration::from_millis(FRAME_SIZE_MS as u64)).await;
    }

    info!("\n📊 播放统计:");
    info!("   总帧数: {}", total_frames);
    info!("   总字节数: {} bytes", total_bytes);
    info!("   平均比特率: {:.2} kbps", 
        (total_bytes as f32 * 8.0) / (TEST_DURATION_SECS as f32 * 1000.0));
    info!("   平均包大小: {:.2} bytes/packet", 
        total_bytes as f32 / total_frames as f32);

    // 获取缓冲区状态
    let (buffer_size, max_buffer_size) = player.get_buffer_status();
    info!("\n📦 缓冲区状态:");
    info!("   当前大小: {} 帧", buffer_size);
    info!("   最大大小: {} 帧", max_buffer_size);
    info!("   使用率: {:.1}%", (buffer_size as f32 / max_buffer_size as f32) * 100.0);

    // 停止播放
    info!("\n⏹️ 停止播放...");
    player.stop();

    // 等待一会儿确保资源被正确释放
    time::sleep(Duration::from_secs(1)).await;

    info!("\n✅ 测试完成!");
    Ok(())
} 