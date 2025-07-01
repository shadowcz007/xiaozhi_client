use xiaozhi_client::{
    init_debug_logging, DeviceStatusChecker, Client, Config,
    types::ClientError
};
use std::io::{self, Write};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志记录
    init_debug_logging();
    
    println!("🤖 小智语音助手 Rust 客户端");
    println!("================================");
    
    // 检查命令行参数
    let args: Vec<String> = std::env::args().collect();
    let device_id = if args.len() > 1 {
        args[1].clone()
    } else {
        // 使用默认设备ID
        "9b:9b:f3:50:dc:17".to_string()
    };

    let name = if args.len() > 2 {
        args[2].clone()
    } else {
        "goodmate".to_string()
    };
    
    println!("🔍 检查设备状态: {}", device_id);
    
    // 创建设备状态检查器
    let checker = DeviceStatusChecker::new();
    
    let status_response = match checker.check_device_status(&device_id,name.as_str()).await? {
        Some(status) => {
            println!("✅ 设备已激活，开始初始化客户端...");
            status
        }
        None => {
            eprintln!("❌ 设备需要激活");
            eprintln!("💡 请先激活设备后再使用客户端");
            return Err(ClientError::DeviceNotActivated.into());
        }
    };
    
    // 创建配置
    let config = Config::new(
        status_response.websocket.url,
        status_response.websocket.token,
        device_id,
        status_response.mqtt.client_id,
    );
    
    // 创建客户端
    let mut client = Client::new(config)?;
    
    // 设置状态变化回调
    client.set_state_change_callback(|state| {
        println!("📱 状态变化: {:?}", state);
    });
    
    println!("🚀 启动语音聊天...");
    println!("💡 提示:");
    println!("   - 客户端会自动开始语音对话");
    println!("   - 按 Ctrl+C 退出程序");
    println!("   - 首次启动会发送 'hi' 消息开始对话");
    println!("");
    
    // 开始语音聊天
    client.start_voice_chat().await?;
    
    println!("🎙️ 语音聊天已启动，等待交互...");
    
    // 等待中断信号
    match signal::ctrl_c().await {
        Ok(()) => {
            println!("\n📱 收到退出信号，正在关闭...");
            
            // 停止语音聊天
            if let Err(e) = client.stop_voice_chat().await {
                eprintln!("⚠️ 停止语音聊天时出错: {}", e);
            }
            
            // 断开连接
            if let Err(e) = client.disconnect().await {
                eprintln!("⚠️ 断开连接时出错: {}", e);
            }
            
            println!("✅ 程序已安全退出");
        }
        Err(err) => {
            eprintln!("❌ 监听退出信号时出错: {}", err);
            return Err(err.into());
        }
    }
    
    Ok(())
}

/// 交互式模式（备用实现）
#[allow(dead_code)]
async fn interactive_mode(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 进入交互式模式");
    println!("可用命令:");
    println!("  start - 开始语音聊天");
    println!("  stop  - 停止语音聊天");
    println!("  text <message> - 发送文本消息");
    println!("  interrupt - 打断当前对话");
    println!("  status - 显示当前状态");
    println!("  quit - 退出程序");
    println!("");
    
    loop {
        print!("xiaozhi> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let command = parts[0];
        
        match command {
            "start" => {
                println!("🚀 启动语音聊天...");
                if let Err(e) = client.start_voice_chat().await {
                    eprintln!("❌ 启动失败: {}", e);
                }
            }
            "stop" => {
                println!("🛑 停止语音聊天...");
                if let Err(e) = client.stop_voice_chat().await {
                    eprintln!("❌ 停止失败: {}", e);
                }
            }
            "text" => {
                if parts.len() > 1 {
                    let message = parts[1];
                    println!("💬 发送文本消息: {}", message);
                    if let Err(e) = client.send_text_message(message).await {
                        eprintln!("❌ 发送失败: {}", e);
                    }
                } else {
                    eprintln!("❌ 请提供要发送的消息内容");
                }
            }
            "interrupt" => {
                println!("⚡ 打断当前对话...");
                if let Err(e) = client.interrupt_conversation().await {
                    eprintln!("❌ 打断失败: {}", e);
                }
            }
            "status" => {
                let state = client.get_device_state().await;
                let is_recording = client.is_recording();
                let keep_listening = client.is_keep_listening();
                
                println!("📱 当前状态:");
                println!("   设备状态: {:?}", state);
                println!("   正在录音: {}", is_recording);
                println!("   持续监听: {}", keep_listening);
            }
            "quit" | "exit" => {
                println!("👋 再见！");
                break;
            }
            _ => {
                eprintln!("❌ 未知命令: {}", command);
                eprintln!("💡 输入 'quit' 退出程序");
            }
        }
    }
    
    Ok(())
} 