use clap::{Arg, Command};
use std::process;
use std::sync::Arc;
use xiaozhi_client::{
    init_logging, ActivationResult, Client, Config, Device, DeviceFingerprint, DeviceManager,
    DeviceStatusChecker, DeviceStatusResult, StdioController,
};
use xiaozhi_client::ui::InteractiveMenu;

#[allow(dead_code)]
async fn activate_device(device: &Device, checker: &DeviceStatusChecker, menu: &InteractiveMenu) -> Result<bool, String> {
    menu.loading("正在检查设备状态...");

    let status = checker
        .check_device_status(&device.device_id, &device.device_name)
        .await
        .map_err(|e| e.to_string())?;

    match status {
        DeviceStatusResult::Activated(_) => {
            println!("\r");
            menu.success("设备已激活");
            return Ok(true);
        }
        DeviceStatusResult::NeedsActivation(info) => {
            if let (Some(challenge), Some(code), Some(message)) = (
                &info.challenge,
                &info.activation_code,
                &info.activation_message,
            ) {
                menu.activation_code(code, message, "https://xiaozhi.me");

                let hmac_signature = device.hmac_key.clone();

                menu.loading("等待激活...");

                match checker
                    .activate_with_retry(
                        challenge,
                        &device.device_id,
                        &device.serial_number,
                        &hmac_signature,
                        60,
                        5000,
                    )
                    .await
                {
                    Ok(ActivationResult::Success) => {
                        println!("\r");
                        menu.success("设备激活成功!");
                        Ok(true)
                    }
                    Ok(ActivationResult::WaitingForCode { .. }) => {
                        println!("\r");
                        menu.success("激活验证完成!");
                        Ok(true)
                    }
                    Ok(ActivationResult::Failed(msg)) => {
                        println!("\r");
                        Err(format!("激活失败: {}", msg))
                    }
                    Err(e) => {
                        println!("\r");
                        Err(format!("激活请求失败: {}", e))
                    }
                }
            } else {
                Err("激活信息不完整".to_string())
            }
        }
        DeviceStatusResult::NeedsActivationNoInfo => Err("服务器未返回激活信息".to_string()),
    }
}

async fn run_device_manager() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = DeviceManager::new();
    let checker = DeviceStatusChecker::new();
    let menu = InteractiveMenu::new();

    loop {
        menu.clear_screen();
        menu.header("小智设备管理");
        menu.menu_item("1", "列出所有设备", "查看已注册设备列表");
        menu.menu_item("2", "创建虚拟设备", "创建新的虚拟设备");
        menu.menu_item("3", "切换当前设备", "设置默认启动设备");
        menu.menu_item("4", "激活指定设备", "完成设备激活流程");
        menu.menu_item("5", "查看设备详情", "查看设备详细信息");
        menu.menu_item("6", "删除设备", "删除设备");
        menu.menu_item("7", "启动当前设备", "使用当前设备启动语音助手");
        menu.menu_item("0", "退出", "退出程序");
        menu.footer();

        let choice = menu.prompt("请选择操作");

        match choice.as_str() {
            "1" => {
                menu.section("设备列表");
                let devices = manager.list_devices();
                let current_id = manager.current_device_id();

                if devices.is_empty() {
                    menu.warning("暂无注册设备");
                } else {
                    for (i, device) in devices.iter().enumerate() {
                        let is_current = Some(device.device_id.as_str()) == current_id;
                        menu.device_list_item(
                            i + 1,
                            &device.device_name,
                            &device.device_id,
                            device.activated,
                            is_current,
                        );
                    }
                }
                menu.prompt("");
            }
            "2" => {
                menu.section("创建虚拟设备");
                let name = menu.prompt_with_default("设备名称", "");
                menu.loading("正在创建设备...");

                match manager.create_virtual_device(if name.is_empty() { None } else { Some(name) }) {
                    Ok(device) => {
                        menu.loading_end(
                            true,
                            &format!("虚拟设备创建成功: {}", device.device_name),
                        );
                        menu.info(&format!("设备ID: {}", device.device_id));
                        if let Some(ref mac) = device.virtual_mac {
                            menu.info(&format!("虚拟MAC: {}", mac));
                        }
                    }
                    Err(e) => {
                        menu.loading_end(false, "创建设备失败");
                        menu.error(&e);
                    }
                }
                menu.prompt("");
            }
            "3" => {
                menu.section("切换当前设备");
                let devices = manager.list_devices();

                if devices.is_empty() {
                    menu.warning("暂无设备");
                } else {
                    for (i, device) in devices.iter().enumerate() {
                        menu.device_list_item(
                            i + 1,
                            &device.device_name,
                            &device.device_id,
                            device.activated,
                            false,
                        );
                    }
                    menu.footer();

                    let num_str = menu.prompt("输入要切换的设备编号");
                    let num: usize = match num_str.trim().parse() {
                        Ok(n) => n,
                        Err(_) => {
                            menu.error("无效输入");
                            menu.prompt("");
                            continue;
                        }
                    };

                    if num < 1 || num > devices.len() {
                        menu.error("无效编号");
                    } else {
                        let device = devices[num - 1];
                        let device_id = device.device_id.clone();
                        let device_name = device.device_name.clone();
                        match manager.set_current_device(&device_id) {
                            Ok(_) => menu.success(&format!("已切换到: {}", device_name)),
                            Err(e) => menu.error(&e),
                        }
                    }
                }
                menu.prompt("");
            }
            "4" => {
                menu.section("激活设备");
                let devices = manager.list_devices();

                if devices.is_empty() {
                    menu.warning("暂无设备");
                } else {
                    let unactivated: Vec<_> = devices
                        .iter()
                        .filter(|d| !d.activated)
                        .collect();

                    if unactivated.is_empty() {
                        menu.info("所有设备都已激活");
                    } else {
                        menu.info("可激活的设备:");
                        for (i, device) in unactivated.iter().enumerate() {
                            menu.device_list_item(
                                i + 1,
                                &device.device_name,
                                &device.device_id,
                                device.activated,
                                false,
                            );
                        }
                        menu.footer();

                        let num_str = menu.prompt("输入要激活的设备编号");
                        let num: usize = match num_str.trim().parse() {
                            Ok(n) => n,
                            Err(_) => {
                                menu.error("无效输入");
                                menu.prompt("");
                                continue;
                            }
                        };

                        if num < 1 || num > unactivated.len() {
                            menu.error("无效编号");
                        } else {
                            let device = unactivated[num - 1].clone();
                            let device_id = device.device_id.clone();
                            let result = activate_device(&device, &checker, &menu).await;
                            match result {
                                Ok(true) => {
                                    manager.set_activation_status(&device_id, true).ok();
                                }
                                Ok(false) => {}
                                Err(e) => menu.error(&e),
                            }
                            manager = DeviceManager::new();
                        }
                    }
                }
                menu.prompt("");
            }
            "5" => {
                menu.section("设备详情");
                let devices = manager.list_devices();

                if devices.is_empty() {
                    menu.warning("暂无设备");
                } else {
                    for (i, device) in devices.iter().enumerate() {
                        menu.device_list_item(
                            i + 1,
                            &device.device_name,
                            &device.device_id,
                            device.activated,
                            false,
                        );
                    }
                    menu.footer();

                    let num_str = menu.prompt("输入要查看的设备编号");
                    let num: usize = match num_str.trim().parse() {
                        Ok(n) => n,
                        Err(_) => {
                            menu.error("无效输入");
                            menu.prompt("");
                            continue;
                        }
                    };

                    if num < 1 || num > devices.len() {
                        menu.error("无效编号");
                    } else {
                        let device = devices[num - 1];
                        menu.section(&device.device_name);
                        menu.device_detail("", "设备ID", &device.device_id);
                        menu.device_detail("", "序列号", &device.serial_number);
                        menu.device_detail("", "类型", if device.is_virtual { "虚拟设备" } else { "物理设备" });
                        menu.device_detail("", "激活状态", if device.activated { "已激活" } else { "未激活" });
                        if let Some(ref mac) = device.virtual_mac {
                            menu.device_detail("", "虚拟MAC", mac);
                        }
                        if let Some(ref at) = device.activated_at {
                            menu.device_detail("", "激活时间", at);
                        }
                    }
                }
                menu.prompt("");
            }
            "6" => {
                menu.section("删除设备");
                let devices = manager.list_devices();

                if devices.is_empty() {
                    menu.warning("暂无设备");
                } else {
                    for (i, device) in devices.iter().enumerate() {
                        menu.device_list_item(
                            i + 1,
                            &device.device_name,
                            &device.device_id,
                            device.activated,
                            false,
                        );
                    }
                    menu.footer();

                    let num_str = menu.prompt("输入要删除的设备编号");
                    let num: usize = match num_str.trim().parse() {
                        Ok(n) => n,
                        Err(_) => {
                            menu.error("无效输入");
                            menu.prompt("");
                            continue;
                        }
                    };

                    if num < 1 || num > devices.len() {
                        menu.error("无效编号");
                    } else {
                        let device = devices[num - 1];
                        let device_id = device.device_id.clone();
                        let device_name = device.device_name.clone();
                        if menu.confirm(&format!("确认删除设备 {}?", device_name)) {
                            match manager.delete_device(&device_id) {
                                Ok(_) => menu.success("设备已删除"),
                                Err(e) => menu.error(&e),
                            }
                        } else {
                            menu.info("取消删除");
                        }
                    }
                }
                menu.prompt("");
            }
            "7" => {
                if let Some(current) = manager.get_current_device() {
                    menu.success(&format!("启动设备: {}", current.device_name));
                    menu.info(&format!("设备ID: {}", current.device_id));
                    menu.info("请使用 --device-id 参数启动客户端");
                    menu.prompt("");
                } else {
                    menu.warning("没有当前设备，请先创建并切换");
                    menu.prompt("");
                }
            }
            "0" | "q" | "quit" => {
                menu.success("再见!");
                break;
            }
            _ => {
                menu.error("无效选择，请重试");
                menu.prompt("");
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("小智语音助手客户端")
        .version("0.1.0")
        .author("shadow")
        .about("XiaoZhi Voice Assistant Client - 基于 Rust 开发的智能语音助手")
        .long_about(
            "小智语音助手客户端\n\n\
            一款基于 Rust 开发的智能语音助手，支持实时语音对话、多设备管理。\n\n\
            使用示例:\n  xiaozhi_client --manage                      # 设备管理\n  xiaozhi_client --device-id <ID>            # 启动语音助手\n  xiaozhi_client --activate                   # 激活当前设备\n\n\
            GitHub: https://github.com/shadowcz007/xiaozhi_client\n\
            作者: shadow",
        )
        .arg(Arg::new("manage").long("manage").action(clap::ArgAction::SetTrue).help("进入设备管理模式"))
        .arg(Arg::new("activate").long("activate").action(clap::ArgAction::SetTrue).help("激活设备"))
        .arg(
            Arg::new("device-id")
                .long("device-id")
                .value_name("DEVICE_ID")
                .help("设备ID")
                .default_value("9b:9b:f3:50:dc:17"),
        )
        .arg(
            Arg::new("device-name")
                .long("device-name")
                .value_name("DEVICE_NAME")
                .help("设备名称")
                .default_value("goodmate"),
        )
        .get_matches();

    if matches.get_flag("manage") {
        run_device_manager().await?;
        return Ok(());
    }

    if matches.get_flag("activate") {
        let manager = DeviceManager::new();
        let menu = InteractiveMenu::new();

        if let Some(current) = manager.get_current_device() {
            let checker = DeviceStatusChecker::new();
            if let Err(e) = activate_device(current, &checker, &menu).await {
                eprintln!("激活失败: {}", e);
                process::exit(1);
            }
            menu.success("设备激活成功!");
        } else {
            eprintln!("没有当前设备，请先在管理模式下创建设备");
            process::exit(1);
        }
        return Ok(());
    }

    init_logging();

    let device_id = matches.get_one::<String>("device-id").unwrap();
    let device_name = matches.get_one::<String>("device-name").unwrap();

    println!("正在检查设备状态...");
    println!("设备ID: {}", device_id);
    println!("设备名称: {}", device_name);

    let checker = DeviceStatusChecker::new();
    let status = checker.check_device_status(&device_id, &device_name).await?;

    match status {
        DeviceStatusResult::Activated(status_response) => {
            println!("设备已激活，正在初始化客户端...");

            let config = Config::new(
                status_response.websocket.url,
                status_response.websocket.token,
                device_id.to_string(),
                status_response.mqtt.client_id,
            );

            let mut client = Client::new(config)?;

            client.set_state_change_callback(|state| {
                println!("状态变化: {:?}", state);
            });

            let client = Arc::new(client);

            let controller = StdioController::new(Arc::clone(&client));

            tokio::spawn(async move {
                if let Err(e) = controller.start().await {
                    eprintln!("控制器启动错误: {:?}", e);
                }
            });

            tokio::signal::ctrl_c().await?;
            println!("\n收到退出信号，正在清理...");

            client.disconnect().await?;
        }
        DeviceStatusResult::NeedsActivation(_) => {
            println!("设备需要先激活");
            println!("提示: 请先运行设备激活程序");
            println!("   --manage 进入设备管理菜单");
            println!("   --activate 激活当前设备");
        }
        DeviceStatusResult::NeedsActivationNoInfo => {
            println!("设备需要先激活");
            println!("提示: 请先运行设备激活程序");
            println!("   --manage 进入设备管理菜单");
            println!("   --activate 激活当前设备");
        }
    }

    Ok(())
}