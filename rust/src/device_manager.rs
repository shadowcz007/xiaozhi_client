use crate::fingerprint::DeviceFingerprint;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub device_id: String,
    pub device_name: String,
    pub serial_number: String,
    pub hmac_key: String,
    pub activated: bool,
    pub activated_at: Option<String>,
    pub is_virtual: bool,
    pub virtual_mac: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeviceStore {
    devices: Vec<Device>,
    current_device_id: Option<String>,
}

impl Default for DeviceStore {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            current_device_id: None,
        }
    }
}

pub struct DeviceManager {
    store: DeviceStore,
    config_path: PathBuf,
}

impl DeviceManager {
    pub fn new() -> Self {
        let config_path = get_config_dir().join("devices.json");
        let store = Self::load_from_file(&config_path).unwrap_or_default();
        Self { store, config_path }
    }

    pub fn with_path(config_path: PathBuf) -> Self {
        let store = Self::load_from_file(&config_path).unwrap_or_default();
        Self { store, config_path }
    }

    fn load_from_file(path: &PathBuf) -> Option<DeviceStore> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_to_file(&self) -> std::io::Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.store)?;
        fs::write(&self.config_path, content)
    }

    pub fn list_devices(&self) -> Vec<&Device> {
        self.store.devices.iter().collect()
    }

    pub fn get_device(&self, device_id: &str) -> Option<&Device> {
        self.store.devices.iter().find(|d| d.device_id == device_id)
    }

    pub fn get_device_mut(&mut self, device_id: &str) -> Option<&mut Device> {
        self.store
            .devices
            .iter_mut()
            .find(|d| d.device_id == device_id)
    }

    pub fn get_current_device(&self) -> Option<&Device> {
        let current_id = self.store.current_device_id.as_deref()?;
        self.get_device(current_id)
    }

    pub fn current_device_id(&self) -> Option<&str> {
        self.store.current_device_id.as_deref()
    }

    pub fn create_physical_device(
        &mut self,
        device_name: Option<String>,
    ) -> Result<Device, String> {
        let fingerprint = DeviceFingerprint::collect(None, false, None);
        let device_id = fingerprint.hostname.clone();
        let (serial_number, _) = fingerprint.generate_serial_number();
        let hmac_key = fingerprint.generate_hmac_key();

        if self.store.devices.iter().any(|d| d.device_id == device_id) {
            return Err(format!("设备 {} 已存在", device_id));
        }

        let device = Device {
            device_id: device_id.clone(),
            device_name: device_name
                .unwrap_or_else(|| format!("物理设备_{}", &device_id[..8.min(device_id.len())])),
            serial_number,
            hmac_key,
            activated: false,
            activated_at: None,
            is_virtual: false,
            virtual_mac: None,
        };

        self.store.devices.push(device.clone());
        if self.store.current_device_id.is_none() {
            self.store.current_device_id = Some(device_id);
        }
        self.save_to_file().map_err(|e| e.to_string())?;
        Ok(device)
    }

    pub fn create_virtual_device(&mut self, device_name: Option<String>) -> Result<Device, String> {
        let virtual_mac = generate_virtual_mac();
        let device_id = virtual_mac.clone();
        let fingerprint =
            DeviceFingerprint::collect(Some(device_id.clone()), true, Some(virtual_mac.clone()));
        let (serial_number, _) = fingerprint.generate_serial_number();
        let hmac_key = fingerprint.generate_hmac_key();

        if self.store.devices.iter().any(|d| d.device_id == device_id) {
            return Err(format!("设备 {} 已存在", device_id));
        }

        let unique_name = device_name.unwrap_or_else(|| {
            let suffix = &virtual_mac.replace(':', "")[..8];
            format!("虚拟设备_{}", suffix)
        });

        let final_name = self.generate_unique_name(&unique_name);

        let device = Device {
            device_id,
            device_name: final_name,
            serial_number,
            hmac_key,
            activated: false,
            activated_at: None,
            is_virtual: true,
            virtual_mac: Some(virtual_mac),
        };

        self.store.devices.push(device.clone());
        if self.store.current_device_id.is_none() {
            self.store.current_device_id = Some(device.device_id.clone());
        }
        self.save_to_file().map_err(|e| e.to_string())?;
        Ok(device)
    }

    pub fn delete_device(&mut self, device_id: &str) -> Result<(), String> {
        let pos = self
            .store
            .devices
            .iter()
            .position(|d| d.device_id == device_id)
            .ok_or_else(|| format!("设备 {} 不存在", device_id))?;

        self.store.devices.remove(pos);

        if self.store.current_device_id.as_deref() == Some(device_id) {
            self.store.current_device_id = self.store.devices.first().map(|d| d.device_id.clone());
        }

        self.save_to_file().map_err(|e| e.to_string())
    }

    pub fn set_current_device(&mut self, device_id: &str) -> Result<(), String> {
        if !self.store.devices.iter().any(|d| d.device_id == device_id) {
            return Err(format!("设备 {} 不存在", device_id));
        }
        self.store.current_device_id = Some(device_id.to_string());
        self.save_to_file().map_err(|e| e.to_string())
    }

    pub fn set_activation_status(
        &mut self,
        device_id: &str,
        activated: bool,
    ) -> Result<(), String> {
        let device = self
            .store
            .devices
            .iter_mut()
            .find(|d| d.device_id == device_id)
            .ok_or_else(|| format!("设备 {} 不存在", device_id))?;

        device.activated = activated;
        if activated {
            device.activated_at = Some(chrono::Utc::now().to_rfc3339());
        } else {
            device.activated_at = None;
        }

        self.save_to_file().map_err(|e| e.to_string())
    }

    pub fn is_device_name_exists(&self, name: &str) -> bool {
        self.store.devices.iter().any(|d| d.device_name == name)
    }

    fn generate_unique_name(&self, base_name: &str) -> String {
        if !self.is_device_name_exists(base_name) {
            return base_name.to_string();
        }

        let mut name = base_name.to_string();
        let mut counter = 1;

        while self.is_device_name_exists(&name) {
            name = format!("{}_{}", base_name, counter);
            counter += 1;
        }

        name
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

fn get_config_dir() -> PathBuf {
    ProjectDirs::from("com", "xiaozhi", "client")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".device-config"))
}

fn generate_virtual_mac() -> String {
    use md5::{Digest as Md5Digest, Md5};
    use uuid::Uuid;

    let base_str = format!("virtual_{}", Uuid::new_v4());
    let mut hasher = Md5::new();
    hasher.update(base_str.as_bytes());
    let result = hasher.finalize();
    let hash = format!("{:012x}", result);

    let mac_bytes: Vec<String> = (0..6)
        .map(|i| format!("{}", &hash[i * 2..i * 2 + 2]))
        .collect();

    let first_byte = u8::from_str_radix(&mac_bytes[0], 16).unwrap_or(0);
    let local_mac = first_byte | 0x02;

    format!(
        "{:02x}:{}:{}:{}:{}:{}",
        local_mac, mac_bytes[1], mac_bytes[2], mac_bytes[3], mac_bytes[4], mac_bytes[5]
    )
}
