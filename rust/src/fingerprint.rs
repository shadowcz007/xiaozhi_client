use md5::{Digest as Md5Digest, Md5};
use sha2::{Digest as Sha256Digest, Sha256};
use sysinfo::{MacAddr, Networks, System};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DeviceFingerprint {
    pub mac_address: String,
    pub mac_type: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub hostname: String,
    pub system: String,
    pub system_serial: Option<String>,
    pub device_id: String,
    pub is_virtual: bool,
}

impl DeviceFingerprint {
    pub fn collect(
        device_id: Option<String>,
        is_virtual: bool,
        virtual_mac: Option<String>,
    ) -> Self {
        let system = System::new_all();
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let (mac_address, mac_type) = if is_virtual {
            (
                virtual_mac.unwrap_or_else(|| generate_virtual_mac(&hostname)),
                "虚拟网卡".to_string(),
            )
        } else {
            get_mac_address()
        };

        let cpu_model = system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let cpu_cores = system.cpus().len();

        let system_serial = get_system_serial();

        let device_id = device_id.unwrap_or_else(|| {
            if is_virtual {
                mac_address.clone()
            } else {
                hostname.clone()
            }
        });

        Self {
            mac_address,
            mac_type,
            cpu_model,
            cpu_cores,
            hostname,
            system: std::env::consts::OS.to_string(),
            system_serial,
            device_id,
            is_virtual,
        }
    }

    pub fn generate_hmac_key(&self) -> String {
        let identifiers = [
            self.hostname.as_str(),
            self.mac_address.as_str(),
            self.cpu_model.as_str(),
            self.system_serial.as_deref().unwrap_or(""),
            self.system.as_str(),
            self.device_id.as_str(),
        ];

        let fingerprint_str = identifiers.join("||");
        let mut hasher = Sha256::new();
        hasher.update(fingerprint_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn generate_serial_number(&self) -> (String, String) {
        if !self.mac_address.is_empty() && self.mac_address != "00:00:00:00:00:00" {
            let mac_clean = self.mac_address.replace(':', "");
            let mut hasher = Md5::new();
            hasher.update(mac_clean.as_bytes());
            let result = hasher.finalize();
            let hash = format!("{:x}", result);
            let short_hash = hash[..8].to_uppercase();
            (
                format!("SN-{}-{}", short_hash, &mac_clean[..mac_clean.len().min(8)]),
                self.mac_type.clone(),
            )
        } else {
            let hardware_hash = self.generate_hmac_key();
            (
                format!("SN-{}", &hardware_hash[..16].to_uppercase()),
                "硬件哈希值".to_string(),
            )
        }
    }

    pub fn fingerprint_string(&self) -> String {
        [
            self.hostname.as_str(),
            self.mac_address.as_str(),
            self.cpu_model.as_str(),
            self.system_serial.as_deref().unwrap_or(""),
            self.system.as_str(),
            self.device_id.as_str(),
        ]
        .join("||")
    }
}

fn get_mac_address() -> (String, String) {
    let _system = System::new_all();
    let networks = sysinfo::Networks::new_with_refreshed_list();

    let priority_order = ["en0", "en1", "Ethernet", "Wi-Fi"];

    for name in &priority_order {
        if let Some(interface) = networks.get(*name) {
            let mac = format_mac(&interface.mac_address());
            if !mac.is_empty() && mac != "00:00:00:00:00:00" {
                let mac_type = if *name == "en0" {
                    "WiFi网卡"
                } else if name.starts_with("en") {
                    "有线网卡"
                } else {
                    *name
                };
                return (mac.to_lowercase(), mac_type.to_string());
            }
        }
    }

    for (name, interface) in networks.iter() {
        let mac = format_mac(&interface.mac_address());
        if !mac.is_empty() && mac != "00:00:00:00:00:00" {
            return (mac.to_lowercase(), format!("网络接口({})", name));
        }
    }

    ("00:00:00:00:00:00".to_string(), "未找到".to_string())
}

fn format_mac(mac: &MacAddr) -> String {
    let bytes = mac.0;
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(target_os = "macos")]
fn get_system_serial() -> Option<String> {
    use std::process::Command;
    Command::new("system_profiler")
        .args(["SPHardwareDataType"])
        .output()
        .ok()
        .and_then(|output| {
            let output = String::from_utf8_lossy(&output.stdout);
            output
                .lines()
                .find(|line| line.contains("Serial Number"))
                .and_then(|line| line.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty() && s != "unknown")
}

#[cfg(target_os = "linux")]
fn get_system_serial() -> Option<String> {
    use std::fs;
    fs::read_to_string("/sys/class/dmi/id/product_serial")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "unknown")
}

#[cfg(target_os = "windows")]
fn get_system_serial() -> Option<String> {
    use std::process::Command;
    Command::new("wmic")
        .args(["bios", "get", "serialnumber", "/value"])
        .output()
        .ok()
        .and_then(|output| {
            let output = String::from_utf8_lossy(&output.stdout);
            output
                .lines()
                .find(|line| line.contains("SerialNumber="))
                .and_then(|line| line.split('=').nth(1))
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty() && s != "unknown")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn get_system_serial() -> Option<String> {
    None
}

fn generate_virtual_mac(hostname: &str) -> String {
    let base_str = format!("{}_{}", hostname, Uuid::new_v4());
    let mut hasher = Md5::new();
    hasher.update(base_str.as_bytes());
    let result = hasher.finalize();
    let hash = format!("{:x}", result);
    let hex_str = &hash[..12];

    let mac_bytes: Vec<String> = (0..6)
        .map(|i| format!("{}", &hex_str[i * 2..i * 2 + 2]))
        .collect();

    let first_byte = u8::from_str_radix(&mac_bytes[0], 16).unwrap_or(0);
    let local_mac = first_byte | 0x02;

    format!(
        "{:02x}:{}:{}:{}:{}:{}",
        local_mac, mac_bytes[1], mac_bytes[2], mac_bytes[3], mac_bytes[4], mac_bytes[5]
    )
}
