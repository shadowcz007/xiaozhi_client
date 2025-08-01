use xiaozhi_client::{
    init_logging, DeviceStatusChecker, DeviceStatusResult, Client, Config, StdioController
};
use std::io::Write;
use std::sync::Arc;
use std::process;
use clap::{Arg, Command};

mod crypto;
use crypto::LicenseVerifier;

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
    // 解析命令行参数
    let matches = Command::new("XiaoZhi Client")
        .version("0.1.0")
        .author("shadow")
        .about("小智语音助手客户端")
        .arg(
            Arg::new("key")
                .long("key")
                .value_name("ENCODED_KEY")
                .help("Base64 编码的许可证密钥")
                .required(false)  // 开发环境下不强制要求
        )
        .arg(
            Arg::new("device-id")
                .long("device-id")
                .value_name("DEVICE_ID")
                .help("设备ID")
                .default_value("9b:9b:f3:50:dc:17")
        )
        .arg(
            Arg::new("device-name")
                .long("device-name")
                .value_name("DEVICE_NAME")
                .help("设备名称")
                .default_value("goodmate")
        )
        .get_matches();

    // 在开发环境中使用默认的测试许可证
    let encoded_key = if cfg!(debug_assertions) {
        matches.get_one::<String>("key").map(|s| s.as_str()).unwrap_or(
            "eyJsaWNlbnNlIjoidGVzdC1saWNlbnNlIiwicGFzc3dvcmQiOiJ0ZXN0LXBhc3N3b3JkIn0="
        )
    } else {
        matches.get_one::<String>("key")
            .map(|s| s.as_str())
            .ok_or("生产环境需要提供许可证密钥")?
    };

    // 初始化验证器
    let verifier = LicenseVerifier::new();

    // 在开发环境中，如果验证器初始化失败，跳过验证
    if !cfg!(debug_assertions) {
        // 解码并验证 license
        let license_key = match LicenseVerifier::decode_license_key(encoded_key) {
            Ok(key) => key,
            Err(e) => {
                eprintln!("❌ 无效的密钥格式: {}", e);
                eprintln!("💡 请使用正确的 base64 编码格式的许可证密钥");
                process::exit(1);
            }
        };

        match verifier.verify_license(&license_key) {
            Ok(true) => println!("✅ 许可证验证成功"),
            Ok(false) => {
                eprintln!("❌ 无效的许可证");
                eprintln!("💡 请联系管理员获取有效的许可证");
                process::exit(1);
            }
            Err(e) => {
                eprintln!("❌ 许可证验证失败: {}", e);
                process::exit(1);
            }
        }
    }

    // 初始化日志
    init_logging();
    
    // 获取设备ID和设备名称
    let device_id = matches.get_one::<String>("device-id").unwrap();
    let device_name = matches.get_one::<String>("device-name").unwrap();
    
    println!("🔍 正在检查设备状态...");
    println!("📱 设备ID: {}", device_id);
    println!("📝 设备名称: {}", device_name);
    
    // 检查设备状态
    let checker = DeviceStatusChecker::new();
    let status = checker.check_device_status(&device_id, &device_name).await?;
    
    match status {
        DeviceStatusResult::Activated(status_response) => {
            // 设备已激活，正常启动
            println!("✅ 设备已激活，正在初始化客户端...");
            
            // 添加调试信息
            println!("🔍 调试信息:");
            println!("   - 设备ID: {}", device_id);
            println!("   - 设备名称: {}", device_name);
            println!("   - WebSocket URL: {}", status_response.websocket.url);
            println!("   - WebSocket Token: {}", status_response.websocket.token);
            println!("   - MQTT Client ID: {}", status_response.mqtt.client_id);
            println!("   - MQTT Endpoint: {}", status_response.mqtt.endpoint);
            println!("   - Firmware URL: {}", status_response.firmware.url);
            println!("   - Firmware Version: {}", status_response.firmware.version);
            println!("   - Server Time: {}", status_response.server_time.timestamp);
            println!();
            
            // 创建配置
            let config = Config::new(
                status_response.websocket.url,
                status_response.websocket.token,
                device_id.to_string(),
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
        }
        DeviceStatusResult::NeedsActivation(activation_info) => {
            // 设备需要激活，显示详细信息
            println!("❌ 设备需要先激活");
           
        }
        DeviceStatusResult::NeedsActivationNoInfo => {
            println!("❌ 设备需要先激活");
            println!("💡 提示: 请先运行设备激活程序");
        }
    }
    
    Ok(())
}


