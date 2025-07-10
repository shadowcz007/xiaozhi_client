use cpal::traits::{DeviceTrait, HostTrait};
use tracing::info;
use std::time::Duration;
use xiaozhi_client::voice::recorder::MicrophoneOpusRecorder;
use tokio::sync::mpsc;

const TEST_DURATION_SECS: u64 = 5; // 测试录音时长

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("🎤 开始录音器测试");
    info!("=================");

    // 获取默认输入设备
    let host = cpal::default_host();
    let input_device = host.default_input_device()
        .ok_or("未找到默认输入设备")?;

    // 打印设备信息
    info!("📱 使用输入设备: {}", input_device.name()?);

    // 获取设备支持的配置
    let supported_configs = input_device.supported_input_configs()?
        .collect::<Vec<_>>();

    // 打印所有支持的配置
    info!("支持的配置:");
    for config in &supported_configs {
        info!("- 采样率: {}-{}Hz, 声道: {}, 格式: {:?}",
            config.min_sample_rate().0,
            config.max_sample_rate().0,
            config.channels(),
            config.sample_format());
    }

    // 创建录音器
    // 注意：我们使用设备支持的配置，录音器内部会进行必要的转换
    let mut recorder = MicrophoneOpusRecorder::new(
        16000, // 使用设备支持的采样率
        1,     // 使用设备支持的声道数
        960    // 帧大小 = 采样率 * 0.02 (20ms)
    )?;
    
    // 获取实际使用的音频配置
    let (actual_sample_rate, actual_channels, frame_size) = recorder.get_audio_config();
    info!("\n📊 实际音频配置:");
    info!("   采样率: {}Hz (录音) → 16000Hz (Opus编码)", actual_sample_rate);
    info!("   声道数: {} (录音) → 1 (Opus编码)", actual_channels);
    info!("   帧大小: {} 样本", frame_size);
    info!("   帧时长: {:.1}ms", (frame_size as f32 / actual_sample_rate as f32) * 1000.0);

    // 开始录音
    info!("\n▶️ 开始录音 ({} 秒)...", TEST_DURATION_SECS);
    let mut opus_receiver = recorder.start_recording()?;

    // 用于统计的变量
    let mut total_opus_packets = 0;
    let mut total_opus_bytes = 0;
    let mut min_packet_size = usize::MAX;
    let mut max_packet_size = 0;

    // 创建一个通道用于异步处理Opus数据
    let (stats_tx, mut stats_rx) = mpsc::channel(100);
    
    // 在后台处理Opus数据
    tokio::spawn(async move {
        while let Some(opus_data) = opus_receiver.recv().await {
            let packet_size = opus_data.len();
            let _ = stats_tx.send((packet_size, opus_data)).await;
        }
    });

    // 录制指定时长
    tokio::time::sleep(Duration::from_secs(TEST_DURATION_SECS)).await;

    // 停止录音
    info!("⏹️ 停止录音");
    recorder.stop_recording();

    // 处理统计数据
    while let Ok((packet_size, opus_data)) = stats_rx.try_recv() {
        total_opus_packets += 1;
        total_opus_bytes += opus_data.len();
        min_packet_size = min_packet_size.min(packet_size);
        max_packet_size = max_packet_size.max(packet_size);
    }

    // 打印统计信息
    info!("\n📊 Opus编码统计:");
    if total_opus_packets > 0 {
        info!("   总包数: {} 个", total_opus_packets);
        info!("   总字节数: {} bytes", total_opus_bytes);
        info!("   平均包大小: {:.2} bytes", total_opus_bytes as f32 / total_opus_packets as f32);
        info!("   最小包大小: {} bytes", min_packet_size);
        info!("   最大包大小: {} bytes", max_packet_size);
        info!("   平均比特率: {:.2} kbps", 
            (total_opus_bytes as f32 * 8.0) / (TEST_DURATION_SECS as f32 * 1000.0));
        info!("   包发送频率: {:.1} packets/s", 
            total_opus_packets as f32 / TEST_DURATION_SECS as f32);
    } else {
        info!("   ⚠️ 未收到任何Opus数据包");
    }

    info!("\n✅ 测试完成!");
    Ok(())
} 