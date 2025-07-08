use std::sync::Arc;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use xiaozhi_client::*;

/// ESP32 MCP服务器示例
/// 
/// 这个示例展示了如何创建一个ESP32设备作为MCP服务器，
/// 响应后台API的MCP调用请求，实现完整的MCP协议流程。
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    init_debug_logging();
    
    println!("🚀 启动ESP32 MCP服务器示例");
    println!("=" .repeat(50));
    
    // 配置ESP32设备信息
    let config = Config::new(
        "ws://localhost:8080/ws".to_string(),  // WebSocket服务器地址
        "esp32_token_123".to_string(),         // 访问令牌
        "esp32_device_001".to_string(),        // 设备ID
        "xiaozhi_client_esp32".to_string(),    // 客户端ID
    );
    
    // 创建客户端实例（ESP32设备）
    let client = create_client(
        config.websocket_url.clone(),
        config.access_token.clone(),
        config.device_id.clone(),
        config.client_id.clone()
    ).await?;
    
    println!("✅ ESP32客户端已创建，设备ID: {}", config.device_id);
    
    // 演示MCP工具功能
    demo_mcp_tools(&client).await?;
    
    // 启动WebSocket连接（在实际ESP32设备中，这会建立与后台的连接）
    println!("\n🌐 准备连接到后台服务器...");
    match client.start_voice_chat(Some("Hello! ESP32设备已准备就绪，MCP协议已启用")).await {
        Ok(_) => {
            println!("✅ 已连接到后台服务器，等待MCP调用请求...");
            
            // 模拟设备运行，等待MCP调用
            simulate_device_operation(&client).await?;
        }
        Err(e) => {
            println!("❌ 连接失败: {}，将以离线模式演示MCP功能", e);
            
            // 即使连接失败，也可以演示本地MCP功能
            demo_offline_mcp_functionality(&client).await?;
        }
    }
    
    Ok(())
}

/// 演示MCP工具功能
async fn demo_mcp_tools(client: &Arc<Client>) -> Result<()> {
    println!("\n📋 ==========  MCP工具演示  ==========");
    
    // 获取MCP工具列表
    match client.get_mcp_tools().await {
        Ok(tools) => {
            println!("🔧 可用的MCP工具 ({} 个):", tools.len());
            for (i, tool) in tools.iter().enumerate() {
                println!("   {}. {} - {}", i + 1, tool.name, tool.description);
            }
        }
        Err(e) => {
            println!("❌ 获取工具列表失败: {}", e);
        }
    }
    
    // 获取MCP资源列表
    match client.get_mcp_resources().await {
        Ok(resources) => {
            println!("\n📄 可用的MCP资源 ({} 个):", resources.len());
            for (i, resource) in resources.iter().enumerate() {
                println!("   {}. {} - {}", i + 1, resource.name, 
                    resource.description.as_deref().unwrap_or("无描述"));
            }
        }
        Err(e) => {
            println!("❌ 获取资源列表失败: {}", e);
        }
    }
    
    println!("=" .repeat(40));
    Ok(())
}

/// 模拟设备操作，展示MCP调用响应
async fn simulate_device_operation(client: &Arc<Client>) -> Result<()> {
    println!("\n🤖 ESP32设备进入运行模式，等待后台MCP调用...");
    
    // 模拟运行30秒，在此期间设备会响应来自后台的MCP调用
    for i in 1..=30 {
        sleep(Duration::from_secs(1)).await;
        
        // 每10秒输出一次状态
        if i % 10 == 0 {
            let state = client.get_device_state().await;
            println!("📊 设备状态检查 ({}s): {:?}", i, state);
        }
        
        // 模拟一些后台可能发起的MCP调用
        if i == 5 {
            println!("\n📞 模拟后台调用: hello_world");
            simulate_mcp_call(client, "hello_world", None).await?;
        }
        
        if i == 15 {
            println!("\n📞 模拟后台调用: get_device_info");
            simulate_mcp_call(client, "get_device_info", None).await?;
        }
        
        if i == 25 {
            println!("\n📞 模拟后台调用: set_led");
            let mut args = HashMap::new();
            args.insert("pin".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
            args.insert("state".to_string(), serde_json::Value::Bool(true));
            simulate_mcp_call(client, "set_led", Some(args)).await?;
        }
    }
    
    println!("\n✅ 设备运行演示完成");
    Ok(())
}

/// 模拟MCP调用（用于演示后台如何调用ESP32设备的工具）
async fn simulate_mcp_call(
    client: &Arc<Client>, 
    tool_name: &str, 
    arguments: Option<HashMap<String, serde_json::Value>>
) -> Result<()> {
    println!("   🔄 正在处理MCP调用: {}", tool_name);
    
    match client.call_mcp_tool(tool_name, arguments).await {
        Ok(result) => {
            println!("   ✅ MCP调用成功: {}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            println!("   ❌ MCP调用失败: {}", e);
        }
    }
    
    Ok(())
}

/// 离线模式演示MCP功能
async fn demo_offline_mcp_functionality(client: &Arc<Client>) -> Result<()> {
    println!("\n🔧 ==========  离线MCP功能演示  ==========");
    
    // Hello World工具演示
    println!("\n1. 测试 Hello World 工具:");
    let mut args = HashMap::new();
    args.insert("name".to_string(), serde_json::Value::String("ESP32".to_string()));
    simulate_mcp_call(client, "hello_world", Some(args)).await?;
    
    // 设备状态工具演示
    println!("\n2. 测试设备状态工具:");
    simulate_mcp_call(client, "get_device_state", None).await?;
    
    // 设备信息工具演示
    println!("\n3. 测试设备信息工具:");
    simulate_mcp_call(client, "get_device_info", None).await?;
    
    // LED控制工具演示
    println!("\n4. 测试LED控制工具:");
    let mut led_args = HashMap::new();
    led_args.insert("pin".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
    led_args.insert("state".to_string(), serde_json::Value::Bool(true));
    simulate_mcp_call(client, "set_led", Some(led_args)).await?;
    
    // 传感器读取工具演示
    println!("\n5. 测试传感器读取工具:");
    let mut sensor_args = HashMap::new();
    sensor_args.insert("sensor_type".to_string(), serde_json::Value::String("temperature".to_string()));
    sensor_args.insert("pin".to_string(), serde_json::Value::Number(serde_json::Number::from(34)));
    simulate_mcp_call(client, "read_sensor", Some(sensor_args)).await?;
    
    // WiFi状态工具演示
    println!("\n6. 测试WiFi状态工具:");
    simulate_mcp_call(client, "get_wifi_status", None).await?;
    
    // 系统信息工具演示
    println!("\n7. 测试系统信息工具:");
    simulate_mcp_call(client, "system_info", None).await?;
    
    // 音频控制工具演示
    println!("\n8. 测试音频控制工具:");
    let mut listen_args = HashMap::new();
    listen_args.insert("mode".to_string(), serde_json::Value::String("always_on".to_string()));
    simulate_mcp_call(client, "start_listening", Some(listen_args)).await?;
    
    sleep(Duration::from_secs(2)).await;
    simulate_mcp_call(client, "stop_listening", None).await?;
    
    println!("\n✅ 离线MCP功能演示完成");
    println!("=" .repeat(50));
    
    Ok(())
}

/// MCP协议流程图展示
fn print_mcp_flow_diagram() {
    println!(r#"
🔄 ESP32 MCP服务器协议流程:

    ESP32设备                     后台API (Client)
         │                              │
         │◄────── WebSocket连接 ─────────│
         │                              │
         │◄───── MCP Initialize ────────│
         │                              │
         │────── Initialize Response ──►│
         │                              │
         │◄───── MCP Tools List ────────│
         │                              │
         │────── Tools List Response ──►│
         │                              │
         │◄───── MCP Tool Call ─────────│
         │                              │
         │────── Tool Call Response ───►│
         │                              │
         │◄───── 更多MCP调用... ────────│
         │                              │

🔧 支持的MCP工具:
   • hello_world - Hello World示例
   • send_message - 发送消息给AI
   • get_device_state - 获取设备状态
   • get_device_info - 获取设备信息
   • start_listening - 开始语音监听
   • stop_listening - 停止语音监听
   • interrupt_conversation - 打断对话
   • set_led - 控制LED灯
   • read_sensor - 读取传感器数据
   • get_wifi_status - 获取WiFi状态
   • system_info - 获取系统信息

📄 支持的MCP资源:
   • esp32://device/status - 设备状态
   • esp32://audio/config - 音频配置
   • esp32://gpio/config - GPIO配置
   • esp32://wifi/status - WiFi状态
"#);
} 