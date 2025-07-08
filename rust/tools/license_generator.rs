use std::env;
use std::process;

// 包含 crypto 模块
mod crypto {
    include!("../src/crypto.rs");
}
use crypto::generate_encoded_license;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("使用方法: {} <license> <password>", args[0]);
        eprintln!("");
        eprintln!("例如:");
        eprintln!("  {} my-license-key my-password-123", args[0]);
        process::exit(1);
    }

    let license = &args[1];
    let password = &args[2];

    println!("🔑 正在生成许可证密钥...");
    println!("📝 License: {}", license);
    println!("🔒 Password: {}", password);
    println!("");

    match generate_encoded_license(license, password) {
        Ok(encoded_key) => {
            println!("✅ 生成成功！");
            println!("");
            println!("📋 Base64 编码的许可证密钥:");
            println!("{}", encoded_key);
            println!("");
            println!("💡 使用方法:");
            println!("./xiaozhi_client --key {}", encoded_key);
            println!("");
            println!("🔧 完整命令示例:");
            println!("./xiaozhi_client --key {} --device-id \"9b:9b:f3:50:dc:17\" --device-name \"goodmate\"", encoded_key);
        }
        Err(e) => {
            eprintln!("❌ 生成许可证失败: {}", e);
            process::exit(1);
        }
    }
} 