use tokio::io::{self, AsyncBufReadExt, BufReader};
use std::sync::Arc;
use crate::types::ListeningMode;
use crate::client::Client;

/// 命令行控制器
pub struct StdioController {
    client: Arc<Client>,
}

impl StdioController {
    /// 创建新的命令行控制器
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// 启动命令行控制
    pub async fn start(&self) -> anyhow::Result<()> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        println!("\n🎮 控制命令：");
        println!("1. idle - 切换到空闲状态");
        // println!("2. listening - 开始监听 (手动模式)");
        println!("2. start - 开始监听 (持续模式)");
        println!("3. stop - 打断当前对话"); 

        self.client.start_voice_chat(Some("hi")).await?;

        while let Ok(Some(line)) = lines.next_line().await {
            let command = line.trim().to_lowercase();
            
            match command.as_str() {
                "idle" => {
                    println!("🔄 切换到空闲状态");
                    if let Err(e) = self.client.stop_listening_and_set_idle().await {
                        println!("❌ 切换失败: {}", e);
                    }
                },
                "start" => {
                    println!("👂 开始监听 (持续模式)");
                    if let Err(e) = self.client.start_listening(ListeningMode::AlwaysOn).await {
                        println!("❌ 启动监听失败: {}", e);
                    }
                },
                "stop" => {
                    println!("✋ 打断对话");
                    if let Err(e) = self.client.interrupt_conversation().await {
                        println!("❌ 打断失败: {}", e);
                    }
                },
                _ => {
                    println!("❓ 未知命令: {}", command);
                }
            }
        }

        Ok(())
    }
} 