use xiaozhi_client::{
    init_logging, DeviceStatusChecker, Client, Config, DeviceState,
};
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志记录
    init_logging();
    
    println!("🤖 小智语音助手简单示例");
    println!("========================");
    
    // 使用固定的设备ID进行测试
    let device_id = "df:52:34:be:fa:38";
    
    println!("🔍 检查设备状态: {}", device_id);
    
    // 检查设备状态
    let checker = DeviceStatusChecker::new();
    let status_response = match checker.check_device_status(device_id).await? {
        Some(response) => {
            println!("✅ 设备已激活");
            response
        }
        None => {
            eprintln!("❌ 设备需要激活");
            return Err("设备未激活".into());
        }
    };
    
    // 创建客户端配置
    let config = Config::new(
        status_response.websocket.url,
        status_response.websocket.token,
        device_id.to_string(),
        status_response.mqtt.client_id,
    );
    
    println!("🔧 客户端配置:");
    println!("   WebSocket URL: {}", config.websocket_url);
    println!("   设备ID: {}", config.device_id);
    println!("   客户端ID: {}", config.client_id);
    
    // 创建客户端
    let mut client = Client::new(config)?;
    
    // 用于跟踪对话状态的标志
    let conversation_active = Arc::new(AtomicBool::new(false));
    let conversation_active_clone = conversation_active.clone();
    
    // 设置状态变化回调
    client.set_state_change_callback(move |state| {
        println!("📱 状态变化: {:?}", state);
        
        match state {
            DeviceState::Listening => {
                println!("🎤 正在监听，请说话...");
            }
            DeviceState::Speaking => {
                println!("🔊 AI正在回复...");
                conversation_active_clone.store(true, Ordering::Relaxed);
            }
            DeviceState::Idle => {
                if conversation_active_clone.load(Ordering::Relaxed) {
                    println!("✅ 对话轮次完成");
                    conversation_active_clone.store(false, Ordering::Relaxed);
                }
            }
            DeviceState::Connecting => {
                println!("🔄 正在连接...");
            }
        }
    });
    
    println!("🚀 启动语音聊天...");
    
    // 开始语音聊天
    client.start_voice_chat().await?;
    
    println!("💬 发送欢迎消息开始对话...");
    
    // 等待连接稳定
    sleep(Duration::from_millis(1000)).await;
    
    // 演示不同的交互方式
    println!("\n=== 演示1: 发送文本消息 ===");
    client.send_text_message("你好，我是测试用户").await?;
    
    // 等待一轮对话完成
    sleep(Duration::from_secs(10)).await;
    
    println!("\n=== 演示2: 发送另一个问题 ===");
    client.send_text_message("今天天气怎么样？").await?;
    
    // 等待响应
    sleep(Duration::from_secs(8)).await;
    
    // 演示打断功能
    println!("\n=== 演示3: 打断对话 ===");
    if client.get_device_state().await == DeviceState::Speaking {
        println!("⚡ 正在打断AI回复...");
        client.interrupt_conversation().await?;
        sleep(Duration::from_secs(2)).await;
    }
    
    println!("\n=== 演示4: 最后一个问题 ===");
    client.send_text_message("请简单介绍一下你自己").await?;
    
    // 等待对话完成
    sleep(Duration::from_secs(10)).await;
    
    // 停止语音聊天
    println!("\n🛑 停止语音聊天...");
    client.stop_voice_chat().await?;
    
    // 断开连接
    println!("🔌 断开连接...");
    client.disconnect().await?;
    
    println!("✅ 示例运行完成！");
    
    Ok(())
}

/// 长时间运行示例
#[allow(dead_code)]
async fn long_running_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 长时间运行示例");
    
    let device_id = "df:52:34:be:fa:38";
    let checker = DeviceStatusChecker::new();
    
    let status_response = match checker.check_device_status(device_id).await? {
        Some(response) => response,
        None => return Err("设备未激活".into()),
    };
    
    let config = Config::new(
        status_response.websocket.url,
        status_response.websocket.token,
        device_id.to_string(),
        status_response.mqtt.client_id,
    );
    
    let mut client = Client::new(config)?;
    
    client.set_state_change_callback(|state| {
        println!("状态: {:?} - {}", state, chrono::Utc::now().format("%H:%M:%S"));
    });
    
    // 启动语音聊天
    client.start_voice_chat().await?;
    
    // 定期发送消息进行交互
    let messages = vec![
        "你好",
        "今天天气如何？",
        "请告诉我一个有趣的故事",
        "你能帮我解释一下人工智能吗？",
        "谢谢你的帮助",
    ];
    
    for (i, message) in messages.iter().enumerate() {
        println!("\n[{}] 发送消息: {}", i + 1, message);
        client.send_text_message(message).await?;
        
        // 等待对话完成
        sleep(Duration::from_secs(15)).await;
        
        // 检查连接状态
        println!("当前状态: {:?}", client.get_device_state().await);
    }
    
    // 清理
    client.stop_voice_chat().await?;
    client.disconnect().await?;
    
    Ok(())
}

/// 错误处理示例
#[allow(dead_code)]
async fn error_handling_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 错误处理示例");
    
    // 测试无效设备ID
    let invalid_device_id = "invalid-device-id";
    let checker = DeviceStatusChecker::new();
    
    match checker.check_device_status(invalid_device_id).await {
        Ok(Some(_)) => println!("意外的成功"),
        Ok(None) => println!("✅ 正确处理了未激活设备"),
        Err(e) => println!("✅ 正确处理了错误: {}", e),
    }
    
    // 测试无效配置
    let invalid_config = Config::new(
        "ws://invalid-url".to_string(),
        "invalid-token".to_string(),
        "invalid-device".to_string(),
        "invalid-client".to_string(),
    );
    
    match Client::new(invalid_config) {
        Ok(_) => println!("客户端创建成功（可能的，因为只是配置）"),
        Err(e) => println!("✅ 正确处理了配置错误: {}", e),
    }
    
    println!("错误处理示例完成");
    
    Ok(())
} 