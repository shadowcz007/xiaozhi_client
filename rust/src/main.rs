use xiaozhi_client::{
    init_logging, DeviceStatusChecker, Client, Config, StdioController
};
use std::io::Write;
use std::sync::Arc;

// 交互模式的实现
#[allow(dead_code)]
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
    // 初始化日志
    init_logging();
    
    // 获取设备ID（从命令行参数或使用默认值）
    let device_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "9b:9b:f3:50:dc:17".to_string());
        
    // 获取设备名称（从命令行参数或使用默认值）
    let device_name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "goodmate".to_string());
    
    println!("🔍 正在检查设备状态...");
    println!("📱 设备ID: {}", device_id);
    println!("📝 设备名称: {}", device_name);
    
    // 检查设备状态
    let checker = DeviceStatusChecker::new();
    let status = checker.check_device_status(&device_id, &device_name).await?;
    
    if let Some(status_response) = status {
        println!("✅ 设备已激活，正在初始化客户端...");
        
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
        
        // 将客户端包装在 Arc 中
        let client = Arc::new(client);
        
        // 创建并启动 stdio 控制器
        let controller = StdioController::new(Arc::clone(&client));
        
        // 启动异步任务
        tokio::spawn(async move {
            if let Err(e) = controller.start().await {
                eprintln!("❌ 控制器启动错误: {:?}", e);
            }
        });
        
        // 等待程序退出信号
        tokio::signal::ctrl_c().await?;
        println!("\n👋 收到退出信号，正在清理...");
        
        // 断开连接
        client.disconnect().await?;
        
    } else {
        println!("❌ 设备需要先激活");
        println!("💡 提示: 请先运行设备激活程序");
    }
    
    Ok(())
}


