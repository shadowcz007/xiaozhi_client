use xiaozhi_client::{
    init_logging, DeviceStatusChecker, Client, Config,
    types::ClientError
};
use std::io::Write;
use tokio::signal;

// 交互模式的实现
async fn interactive_mode(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动交互模式...");
    println!("💡 提示:");
    println!("   - 输入 'start' 开始语音对话");
    println!("   - 输入 'stop' 停止语音对话");
    println!("   - 输入 'quit' 或 'exit' 退出程序");
    println!("");

    // 自动启动语音对话
    println!("🎙️ 自动启动语音对话...");
    client.start_voice_chat(Some("我是shadow，很高兴见到你")).await?;

    let mut input = String::new();
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        
        input.clear();
        std::io::stdin().read_line(&mut input)?;
        
        let command = input.trim().to_lowercase();
        
        match command.as_str() {
            "start" => {
                println!("🎙️ 开始语音对话...");
                client.start_voice_chat(None).await?;
            }
            "stop" => {
                println!("🛑 停止语音对话...");
                client.stop_voice_chat().await?;
            }
            "quit" | "exit" => {
                println!("👋 正在退出...");
                client.stop_voice_chat().await?;
                client.disconnect().await?;
                break;
            }
            _ => {
                println!("❓ 未知命令，可用命令: start, stop, quit, exit");
            }
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志记录（使用环境变量设置）
    init_logging();
    
    println!("🤖 小智语音助手 Rust 客户端");
    println!("================================");
    
    // 检查命令行参数
    let args: Vec<String> = std::env::args().collect();
    
    // 首先检查是否有 --interactive 标志
    let interactive = args.iter().any(|arg| arg == "--interactive");
    
    // 获取非标志参数
    let mut non_flag_args: Vec<String> = args.iter()
        .filter(|arg| !arg.starts_with("--"))
        .cloned()
        .collect();
    
    // 移除程序名称
    if !non_flag_args.is_empty() {
        non_flag_args.remove(0);
    }
    
    // 设置 device_id
    let device_id = if !non_flag_args.is_empty() {
        non_flag_args[0].clone()
    } else {
        // 使用默认设备ID
        "9b:9b:f3:50:dc:17".to_string()
    };

    // 设置 name
    let name = if non_flag_args.len() > 1 {
        non_flag_args[1].clone()
    } else {
        "goodmate".to_string()
    };
    
    println!("🔍 检查设备状态: {}", device_id);
    
    // 创建设备状态检查器
    let checker = DeviceStatusChecker::new();
    
    let status_response = match checker.check_device_status(&device_id, name.as_str()).await? {
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
    
    if interactive {
        // 使用交互模式
        interactive_mode(&client).await?;
    } else {
        // 使用自动模式
        println!("🚀 启动语音聊天...");
        println!("💡 提示:");
        println!("   - 客户端会自动开始语音对话");
        println!("   - 按 Ctrl+C 退出程序");
        println!("   - 首次启动会发送 'hi' 消息开始对话");
        println!("");
        
        // 开始语音聊天
        client.start_voice_chat(Some("hi，打个招呼吧")).await?;
        
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
    }
    
    Ok(())
}


