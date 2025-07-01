use xiaozhi_client::{
    init_logging,
    voice::MicrophoneOpusRecorder,
};
use tokio::time::{sleep, Duration};
use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    init_logging();
    
    println!("🎤 麦克风输入测试");
    println!("======================");
    println!("此测试将录制10秒钟的音频，并将Opus数据包信息打印到控制台。");
    println!("同时，原始的Opus数据将被保存到 `mic_test_output.opus` 文件中。");

    // 创建录音器
    // 使用小智项目常用的配置: 16kHz采样率, 单声道
    let sample_rate = 16000;
    let channels = 1;
    let frame_duration_ms = 20;
    let frame_size = (sample_rate * frame_duration_ms / 1000) as usize;

    let mut recorder = match MicrophoneOpusRecorder::new(sample_rate, channels, frame_size) {
        Ok(rec) => rec,
        Err(e) => {
            eprintln!("❌ 创建录音器失败: {}", e);
            eprintln!("💡 请确保你有可用的麦克风输入设备。");
            return Err(e.into());
        }
    };
    
    println!("✅ 录音器创建成功，配置: {}Hz, {}声道", sample_rate, channels);
    
    // 开始录音
    println!("▶️ 开始录音...");
    let mut opus_receiver = match recorder.start_recording() {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("❌ 开始录音失败: {}", e);
            return Err(e.into());
        }
    };
    
    // 创建一个文件来保存Opus流
    let mut output_file = File::create("mic_test_output.opus")?;
    println!("💾 Opus数据将保存到: mic_test_output.opus");

    // 用于控制录音时间的标志
    let recording_active = Arc::new(AtomicBool::new(true));
    let recording_active_clone = recording_active.clone();

    // 在10秒后停止录音
    tokio::spawn(async move {
        sleep(Duration::from_secs(10)).await;
        recording_active_clone.store(false, Ordering::Relaxed);
        println!("\n⏱️ 10秒录音时间到。");
    });

    let mut packet_count = 0;
    let start_time = tokio::time::Instant::now();

    println!("----------------------------------------");
    println!("开始接收Opus数据包 (每个'.'代表一个数据包):");

    // 接收Opus数据
    while recording_active.load(Ordering::Relaxed) {
        match opus_receiver.recv().await {
            Some(opus_data) => {
                // 写入文件
                if let Err(e) = output_file.write_all(&opus_data) {
                    eprintln!("\n⚠️ 写入文件失败: {}", e);
                }
                
                // 打印一个点表示收到数据
                print!(".");
                if let Err(e) = io::stdout().flush() {
                    eprintln!("刷新标准输出失败: {}", e);
                }

                packet_count += 1;
            }
            None => {
                println!("\n❗ 音频通道已关闭。");
                break;
            }
        }
    }
    
    let duration = start_time.elapsed();
    println!("\n----------------------------------------");
    
    // 停止录音
    println!("⏹️ 停止录音...");
    recorder.stop_recording();
    
    println!("\n✅ 测试完成!");
    println!("   - 录音时长: {:.2}秒", duration.as_secs_f32());
    println!("   - 共收到 {} 个Opus数据包", packet_count);
    if packet_count > 0 {
        let pps = packet_count as f32 / duration.as_secs_f32();
        println!("   - 平均每秒 {:.1} 个数据包 (预期值: ~50)", pps);
    }
    println!("   - Opus数据已保存到 `mic_test_output.opus`");
    
    Ok(())
} 