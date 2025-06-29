import crypto from 'crypto';
import { promises as fs } from 'fs';
import path from 'path';
import os from 'os';
import { execSync } from 'child_process';
import axios from 'axios';
import readline from 'readline';
/**
 * 设备指纹收集器 - 支持多设备注册码功能
 */
class DeviceFingerprint {
    constructor(deviceId = 'default') {
        this.system = os.platform();
        this.deviceId = deviceId;
        this.configDir = '.device-config';
        this.fingerprintCacheFile = path.join(this.configDir, `.device_fingerprint_${deviceId}.json`);
        this.efuseFile = path.join(this.configDir, `efuse_${deviceId}.json`);
        this.activationDir = path.join(this.configDir, 'activation');
        this.devicesRegistryFile = path.join(this.configDir, 'devices_registry.json');
    }

    /**
     * 获取计算机主机名
     */
    getHostname() {
        return os.hostname();
    }

    /**
     * 获取网络适配器的MAC地址
     */
    getMacAddress() {
        try {
            const networkInterfaces = os.networkInterfaces();

            // 移除Linux网络接口名称，只保留Windows和macOS
            const priorityOrder = ['Ethernet', 'Wi-Fi', 'en0'];


            for (const name of priorityOrder) {
                if (networkInterfaces[name]) {
                    const infaces = networkInterfaces[name].find(net => !net.internal && net.mac !== '00:00:00:00:00:00');
                    if (infaces) {
                        return {
                            mac: infaces.mac.toLowerCase(),
                            type: name.includes('Wi-Fi') ? 'WiFi网卡' : '有线网卡'
                        };
                    }
                }
            }

            // 如果没找到优先的，就选择第一个有效的
            for (const [name, interfaces] of Object.entries(networkInterfaces)) {
                const validInterface = interfaces.find(net => !net.internal && net.mac !== '00:00:00:00:00:00');
                if (validInterface) {
                    return {
                        mac: validInterface.mac.toLowerCase(),
                        type: '网络接口'
                    };
                }
            }

            throw new Error('未找到有效的MAC地址');
        } catch (error) {
            console.error('获取MAC地址失败:', error.message);
            return null;
        }
    }

    /**
     * 获取CPU信息
     */
    getCpuInfo() {
        try {
            const cpus = os.cpus();
            return {
                model: (cpus[0] && cpus[0].model) || 'Unknown',
                cores: cpus.length,
                arch: os.arch(),
                platform: os.platform()
            };
        } catch (error) {
            console.error('获取CPU信息失败:', error.message);
            return { model: 'Unknown', cores: 1, arch: os.arch(), platform: os.platform() };
        }
    }

    /**
     * 获取系统序列号（Windows/macOS）
     */
    getSystemSerial() {
        try {
            let serial = '';

            if (this.system === 'win32') {
                serial = execSync('wmic bios get serialnumber /value', { encoding: 'utf8' })
                    .split('\n')
                    .find(line => line.includes('SerialNumber=')) && line.split('=')[1] && line.split('=')[1].trim();
            } else if (this.system === 'darwin') {
                const output = execSync('system_profiler SPHardwareDataType | grep "Serial Number"', { encoding: 'utf8' });
                const parts = output.split(':');
                serial = parts[1] ? parts[1].trim() : null;
            } else {
                // 不支持的系统平台
                console.warn(`不支持的系统平台: ${this.system}`);
                return null;
            }

            return serial && serial !== 'unknown' ? serial : null;
        } catch (error) {
            console.warn('获取系统序列号失败:', error.message);
            return null;
        }
    }

    /**
     * 生成虚拟MAC地址
     */
    generateVirtualMac() {
        // 基于设备ID和一些随机性生成虚拟MAC地址
        const baseStr = `${this.deviceId}_${Date.now()}_${Math.random()}`;
        const hash = crypto.createHash('md5').update(baseStr).digest('hex');
        const hexStr = hash.substring(0, 12);

        // 格式化为MAC地址格式，确保第一个字节是本地管理的MAC地址
        const macBytes = [];
        for (let i = 0; i < 12; i += 2) {
            macBytes.push(hexStr.substring(i, i + 2));
        }

        // 设置本地管理位（第一个字节的第二位设为1）
        const firstByte = parseInt(macBytes[0], 16);
        macBytes[0] = (firstByte | 0x02).toString(16).padStart(2, '0');

        return macBytes.join(':');
    }

    /**
     * 生成设备指纹
     */
    async generateFingerprint(useVirtualMac = false, virtualMac = null) {
        // 检查缓存
        const cached = await this.loadCachedFingerprint();
        if (cached && !useVirtualMac) return cached;

        let macInfo;
        if (useVirtualMac && virtualMac) {
            macInfo = {
                mac: virtualMac.toLowerCase(),
                type: '虚拟网卡'
            };
        } else {
            macInfo = this.getMacAddress();
        }

        const cpuInfo = this.getCpuInfo();
        const systemSerial = this.getSystemSerial();

        const fingerprint = {
            system: this.system,
            hostname: this.getHostname(),
            mac_address: (macInfo && macInfo.mac),
            mac_type: (macInfo && macInfo.type),
            cpu: cpuInfo,
            system_serial: systemSerial,
            timestamp: Date.now(),
            device_id: this.deviceId,
            is_virtual: useVirtualMac
        };

        // 缓存指纹
        if (!useVirtualMac) {
            await this.cacheFingerprint(fingerprint);
        }
        return fingerprint;
    }

    /**
     * 加载缓存的指纹
     */
    async loadCachedFingerprint() {
        try {
            const data = await fs.readFile(this.fingerprintCacheFile, 'utf8');
            return JSON.parse(data);
        } catch {
            return null;
        }
    }

    /**
     * 缓存指纹
     */
    async cacheFingerprint(fingerprint) {
        try {
            await fs.mkdir(this.configDir, { recursive: true });
            await fs.writeFile(this.fingerprintCacheFile, JSON.stringify(fingerprint, null, 2));
            console.log(`设备指纹已缓存 (${this.deviceId})`);
        } catch (error) {
            console.error('缓存设备指纹失败:', error.message);
        }
    }

    /**
     * 生成硬件哈希
     */
    async generateHardwareHash(useVirtualMac = false, virtualMac = null) {
        const fingerprint = await this.generateFingerprint(useVirtualMac, virtualMac);

        const identifiers = [
            fingerprint.hostname,
            fingerprint.mac_address,
            (fingerprint.cpu && fingerprint.cpu.model),
            fingerprint.system_serial,
            fingerprint.system,
            fingerprint.device_id
        ].filter(Boolean);

        const fingerprintStr = identifiers.join('||');
        return crypto.createHash('sha256').update(fingerprintStr).digest('hex');
    }

    /**
     * 生成设备序列号
     */
    async generateSerialNumber(useVirtualMac = false, virtualMac = null) {
        const fingerprint = await this.generateFingerprint(useVirtualMac, virtualMac);

        if (fingerprint.mac_address) {
            const macClean = fingerprint.mac_address.replace(/:/g, '');
            const shortHash = crypto.createHash('md5').update(macClean).digest('hex').substring(0, 8).toUpperCase();
            return {
                serial: `SN-${shortHash}-${macClean}`,
                source: fingerprint.mac_type || '网络接口'
            };
        }

        // 备选方案：使用硬件哈希
        const hardwareHash = await this.generateHardwareHash(useVirtualMac, virtualMac);
        return {
            serial: `SN-${hardwareHash.substring(0, 16).toUpperCase()}`,
            source: '硬件哈希值'
        };
    }

    /**
     * 确保efuse文件存在
     */
    async ensureEfuseFile(useVirtualMac = false, virtualMac = null, deviceName = null) {
        try {
            await fs.access(this.efuseFile);
            console.log(`efuse配置文件已存在 (${this.deviceId})`);
        } catch {
            // 文件不存在，创建新的
            const { serial, source } = await this.generateSerialNumber(useVirtualMac, virtualMac);
            const hmacKey = await this.generateHardwareHash(useVirtualMac, virtualMac);

            console.log(`生成序列号: ${serial} (来源: ${source}) (设备ID: ${this.deviceId})`);
            console.log(`生成HMAC密钥: ${hmacKey.substring(0, 16)}... (设备ID: ${this.deviceId})`);

            const defaultData = {
                serial_number: serial,
                hmac_key: hmacKey,
                activation_status: false,
                created_at: new Date().toISOString(),
                device_id: this.deviceId,
                device_name: deviceName || (useVirtualMac ? `虚拟设备_${this.deviceId}` : `物理设备_${this.deviceId}`),
                device_type: useVirtualMac ? 'virtual' : 'physical',
                virtual_mac: useVirtualMac ? virtualMac : null
            };

            await fs.mkdir(this.configDir, { recursive: true });
            await fs.writeFile(this.efuseFile, JSON.stringify(defaultData, null, 2));
            console.log(`新设备：已创建efuse配置文件 (${this.deviceId})`);
        }
    }

    /**
     * 加载efuse数据
     */
    async loadEfuseData() {
        try {
            const data = await fs.readFile(this.efuseFile, 'utf8');
            return JSON.parse(data);
        } catch (error) {
            console.error(`加载efuse数据失败 (${this.deviceId}):`, error.message);
            return { serial_number: null, hmac_key: null, activation_status: false };
        }
    }

    /**
     * 保存efuse数据
     */
    async saveEfuseData(data) {
        try {
            await fs.writeFile(this.efuseFile, JSON.stringify(data, null, 2));
            return true;
        } catch (error) {
            console.error(`保存efuse数据失败 (${this.deviceId}):`, error.message);
            return false;
        }
    }

    async saveActivationData(deviceId, data) {
        try {
            await fs.mkdir(this.activationDir, { recursive: true });
            await fs.writeFile(path.join(this.activationDir, `${deviceId}.json`), JSON.stringify(data, null, 2));
            return true;
        } catch (error) {
            console.error(`保存激活数据失败 (${this.deviceId}):`, error.message);
            return false;
        }
    }

    /**
     * 获取序列号
     */
    async getSerialNumber() {
        const data = await this.loadEfuseData();
        return data.serial_number;
    }

    /**
     * 获取HMAC密钥
     */
    async getHmacKey() {
        const data = await this.loadEfuseData();
        return data.hmac_key;
    }

    /**
     * 设置激活状态
     */
    async setActivationStatus(status) {
        const data = await this.loadEfuseData();
        data.activation_status = status;
        data.activated_at = status ? new Date().toISOString() : null;
        return await this.saveEfuseData(data);
    }

    /**
     * 检查是否已激活
     */
    async isActivated() {
        const data = await this.loadEfuseData();
        return data.activation_status || false;
    }

    /**
     * 生成HMAC签名
     */
    async generateHmac(challenge) {
        const hmacKey = await this.getHmacKey();
        if (!hmacKey) {
            throw new Error('未找到HMAC密钥，无法生成签名');
        }

        return crypto.createHmac('sha256', hmacKey).update(challenge).digest('hex');
    }

    /**
     * 注册设备到设备清单
     */
    async registerDevice(deviceName, deviceType, useVirtualMac = false, virtualMac = null) {
        const registry = await this.loadDevicesRegistry();
        // 检查是否为虚拟设备或物理设备
        const isVirtualDevice = deviceType === 'virtual' || useVirtualMac;

        // 生成设备指纹
        const fingerprint = await this.generateFingerprint(isVirtualDevice, virtualMac);

        // 获取更详细的设备信息
        const efuseData = await this.loadEfuseData();


        registry[this.deviceId] = {
            name: deviceName,
            type: deviceType,
            serial_number: await this.getSerialNumber(),
            registered_time: new Date().toISOString(),
            last_used: new Date().toISOString(),
            // 新增：存储完整的指纹信息
            fingerprint: {
                system: fingerprint.system,
                hostname: fingerprint.hostname,
                mac_address: fingerprint.mac_address,
                mac_type: fingerprint.mac_type,
                cpu: fingerprint.cpu,
                system_serial: fingerprint.system_serial,
                device_id: fingerprint.device_id,
                is_virtual: fingerprint.is_virtual,
                fingerprint_created_at: new Date().toISOString(),
                // 可选：包含efuse信息的关键部分
                activation_status: efuseData.activation_status || false,
                device_name: efuseData.device_name || deviceName,
                device_type: efuseData.device_type || deviceType,
                virtual_mac: efuseData.virtual_mac || null
            }
        };
        return await this.saveDevicesRegistry(registry);
    }

    /**
     * 加载设备注册清单
     */
    async loadDevicesRegistry() {
        try {
            const data = await fs.readFile(this.devicesRegistryFile, 'utf8');
            return JSON.parse(data);
        } catch {
            return {};
        }
    }

    /**
     * 保存设备注册清单
     */
    async saveDevicesRegistry(registry) {
        try {
            await fs.mkdir(this.configDir, { recursive: true });
            await fs.writeFile(this.devicesRegistryFile, JSON.stringify(registry, null, 2));
            return true;
        } catch (error) {
            console.error('保存设备注册清单失败:', error.message);
            return false;
        }
    }
}

/**
 * 多设备管理器
 */
class MultiDeviceManager {
    constructor() {
        this.configDir = '.device-config';
        this.devicesRegistryFile = path.join(this.configDir, 'devices_registry.json');
        this.currentDeviceFile = path.join(this.configDir, 'current_device.json');
    }

    /**
     * 检查设备名称是否已存在
     */
    async isDeviceNameExists(deviceName) {
        const registry = await this.loadDevicesRegistry();
        return Object.values(registry).some(device => device.name === deviceName);
    }

    /**
     * 生成唯一的设备名称
     */
    async generateUniqueDeviceName(baseName) {
        let uniqueName = baseName;
        let counter = 1;

        while (await this.isDeviceNameExists(uniqueName)) {
            uniqueName = `${baseName}_${counter}`;
            counter++;
        }

        return uniqueName;
    }

    /**
     * 创建虚拟设备
     */
    async createVirtualDevice(deviceName = null) {
        // 先生成临时设备ID用于生成虚拟MAC
        const tempDeviceId = `temp_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`;
        const tempVirtualDevice = new DeviceFingerprint(tempDeviceId);
        const virtualMac = tempVirtualDevice.generateVirtualMac();

        // 直接使用虚拟MAC作为设备ID
        const deviceId = virtualMac;

        // 确保设备名称唯一
        let finalDeviceName;
        if (deviceName) {
            // 用户指定了名称，需要检查唯一性
            if (await this.isDeviceNameExists(deviceName)) {
                finalDeviceName = await this.generateUniqueDeviceName(deviceName);
                console.log(`设备名称 "${deviceName}" 已存在，自动调整为 "${finalDeviceName}"`);
            } else {
                finalDeviceName = deviceName;
            }
        } else {
            // 用户没有指定名称，生成默认名称并确保唯一性
            const defaultName = `虚拟设备_${virtualMac.replace(/:/g, '').substring(0, 8)}`;
            finalDeviceName = await this.generateUniqueDeviceName(defaultName);
        }

        // 使用虚拟MAC作为设备ID创建设备指纹实例
        const virtualDevice = new DeviceFingerprint(deviceId);

        // 创建efuse文件
        await virtualDevice.ensureEfuseFile(true, virtualMac, finalDeviceName);

        // 注册设备
        await virtualDevice.registerDevice(finalDeviceName, 'virtual', true, virtualMac);

        const serialNumber = await virtualDevice.getSerialNumber();
        const hmacKey = await virtualDevice.getHmacKey();

        console.log(`\n虚拟设备创建成功:`);
        console.log(`设备ID: ${deviceId}`);
        console.log(`设备名称: ${finalDeviceName}`);
        console.log(`虚拟MAC: ${virtualMac}`);
        console.log(`序列号: ${serialNumber}`);
        console.log(`HMAC密钥: ${hmacKey.substring(0, 16)}...`);

        return { deviceId, deviceName: finalDeviceName, virtualMac, serialNumber, hmacKey };
    }

    /**
     * 列出所有设备
     */
    async listDevices() {
        const registry = await this.loadDevicesRegistry();
        return registry;
    }

    /**
     * 删除设备
     */
    async deleteDevice(deviceId) {
        try {
            // 从注册清单中删除
            const registry = await this.loadDevicesRegistry();
            if (registry[deviceId]) {
                delete registry[deviceId];
                await this.saveDevicesRegistry(registry);
            }

            // 删除efuse文件
            const efuseFile = path.join(this.configDir, `efuse_${deviceId}.json`);
            try {
                await fs.unlink(efuseFile);
            } catch {}

            // 删除指纹缓存文件
            const fingerprintFile = path.join(this.configDir, `.device_fingerprint_${deviceId}.json`);
            try {
                await fs.unlink(fingerprintFile);
            } catch {}

            console.log(`设备 ${deviceId} 已删除`);
            return true;
        } catch (error) {
            console.error('删除设备失败:', error.message);
            return false;
        }
    }

    /**
     * 获取设备详细信息
     */
    async getDeviceInfo(deviceId) {
        const registry = await this.loadDevicesRegistry();
        if (!registry[deviceId]) {
            return null;
        }

        const deviceInfo = {...registry[deviceId] };

        // 加载efuse数据
        const device = new DeviceFingerprint(deviceId);
        const efuseData = await device.loadEfuseData();

        return {...deviceInfo, ...efuseData };
    }

    /**
     * 设置当前使用的设备
     */
    async setCurrentDevice(deviceId) {
        const registry = await this.loadDevicesRegistry();
        if (!registry[deviceId]) {
            throw new Error(`设备 ${deviceId} 不存在`);
        }

        // 更新最后使用时间
        registry[deviceId].last_used = new Date().toISOString();
        await this.saveDevicesRegistry(registry);

        // 保存当前设备
        const currentDevice = {
            device_id: deviceId,
            switched_at: new Date().toISOString()
        };

        await fs.mkdir(this.configDir, { recursive: true });
        await fs.writeFile(this.currentDeviceFile, JSON.stringify(currentDevice, null, 2));

        console.log(`已切换到设备: ${registry[deviceId].name} (${deviceId})`);
        return true;
    }

    /**
     * 获取当前使用的设备
     */
    async getCurrentDevice() {
        try {
            const data = await fs.readFile(this.currentDeviceFile, 'utf8');
            const currentDevice = JSON.parse(data);
            return currentDevice.device_id;
        } catch {
            return 'default'; // 默认设备
        }
    }

    /**
     * 加载设备注册清单
     */
    async loadDevicesRegistry() {
        try {
            const registryFile = path.join(this.configDir, 'devices_registry.json');
            const data = await fs.readFile(registryFile, 'utf8');
            return JSON.parse(data);
        } catch {
            return {};
        }
    }

    /**
     * 保存设备注册清单
     */
    async saveDevicesRegistry(registry) {
        try {
            await fs.mkdir(this.configDir, { recursive: true });
            const registryFile = path.join(this.configDir, 'devices_registry.json');
            await fs.writeFile(registryFile, JSON.stringify(registry, null, 2));
            return true;
        } catch (error) {
            console.error('保存设备注册清单失败:', error.message);
            return false;
        }
    }
}

/**
 * 设备激活管理器 - 支持多设备
 */
class DeviceActivator {
    constructor(config = {}) {
        this.config = {
            otaUrl: config.otaUrl || 'https://api.tenclass.net/xiaozhi/ota/',
            authUrl: config.authUrl || 'https://xiaozhi.me/',
            clientId: config.clientId || this.generateClientId(),
            maxRetries: config.maxRetries || 60,
            retryInterval: config.retryInterval || 5000,
            ...config
        };
        this.mixlab_client_id = config.mixlab_client_id;
        delete this.config.mixlab_client_id;
        console.log('config', this.config);

        this.multiDeviceManager = new MultiDeviceManager();
        this.deviceFingerprint = null; // 将在运行时设置
    }

    /**
     * 生成客户端ID
     */
    generateClientId() {
        return crypto.randomUUID();
    }

    /**
     * 初始化设备指纹（支持多设备）
     */
    async initializeDeviceFingerprint(deviceId = null) {
        if (!deviceId) {
            deviceId = await this.multiDeviceManager.getCurrentDevice();
        }

        this.deviceFingerprint = new DeviceFingerprint(deviceId);
        console.log(`使用设备: ${deviceId}`);
        return deviceId;
    }

    /**
     * 确保设备身份信息
     */
    async ensureDeviceIdentity(deviceId = null) {
        const actualDeviceId = await this.initializeDeviceFingerprint(deviceId);

        // 检查设备类型，确定是否为虚拟设备
        const deviceInfo = await this.multiDeviceManager.getDeviceInfo(actualDeviceId);
        const isVirtualDevice = deviceInfo && deviceInfo.device_type === 'virtual';

        if (isVirtualDevice) {
            // 虚拟设备：使用虚拟MAC地址
            const virtualMac = deviceInfo.virtual_mac;
            await this.deviceFingerprint.ensureEfuseFile(true, virtualMac, deviceInfo.name);
        } else {
            // 物理设备：使用默认方式
            await this.deviceFingerprint.ensureEfuseFile();
        }

        const serialNumber = await this.deviceFingerprint.getSerialNumber();
        const isActivated = await this.deviceFingerprint.isActivated();

        console.log(`设备身份信息 (${actualDeviceId}): 序列号: ${serialNumber}, 激活状态: ${isActivated ? '已激活' : '未激活'}${isVirtualDevice ? ' [虚拟设备]' : ' [物理设备]'}`);

        return { deviceId: actualDeviceId, serialNumber, isActivated };
    }

    /**
     * 检查设备状态
     */
    async checkDeviceStatus(deviceId = null) {
        if (deviceId) {
            await this.initializeDeviceFingerprint(deviceId);
        }

        // 检查设备类型，确定是否为虚拟设备
        const deviceInfo = await this.multiDeviceManager.getDeviceInfo(this.deviceFingerprint.deviceId);
        const isVirtualDevice = deviceInfo && deviceInfo.device_type === 'virtual';

        let fingerprint;
        if (isVirtualDevice) {
            // 虚拟设备：使用虚拟MAC地址生成指纹
            const virtualMac = deviceInfo.virtual_mac;
            fingerprint = await this.deviceFingerprint.generateFingerprint(true, virtualMac);
        } else {
            // 物理设备：使用默认方式
            fingerprint = await this.deviceFingerprint.generateFingerprint();
        }

        const serialNumber = await this.deviceFingerprint.getSerialNumber();

        // console.log('deviceInfo', deviceInfo.name )
        const headers = {
            'Activation-Version': '2',
            'Device-Id': fingerprint.mac_address,
            'Client-Id': this.config.clientId,
            'Content-Type': 'application/json',
            'User-Agent': deviceInfo.name + '/1.0.0'
        };

        const payload = {
            version: 2,
            mac_address: fingerprint.mac_address,
            uuid: this.config.clientId,
            hostname: fingerprint.hostname,
            serial_number: serialNumber,
            system: fingerprint.system,
            cpu: fingerprint.cpu,
            timestamp: Date.now(),
            device_id: this.deviceFingerprint.deviceId
        };

        try {
            console.log(`正在检查设备状态 (${this.deviceFingerprint.deviceId})...`);
            // console.log('##payload', payload);
            // console.log('##headers', headers);

            const response = await axios.post(this.config.otaUrl, payload, { headers });
            await this.deviceFingerprint.saveActivationData(this.deviceFingerprint.deviceId, {
                device_id: this.deviceFingerprint.deviceId,
                payload,
                headers,
                data: response.data
            });
            return response.data;
        } catch (error) {
            console.error('检查设备状态失败:', (error.response && error.response.data) || error.message);
            throw error;
        }
    }

    /**
     * 处理激活流程
     */
    async processActivation(activationData) {
        if (!activationData.challenge || !activationData.code) {
            throw new Error('激活数据中缺少必要字段');
        }

        const deviceInfo = await this.multiDeviceManager.getDeviceInfo(this.deviceFingerprint.deviceId);

        const { challenge, code, message = '请在xiaozhi.me输入验证码' } = activationData;

        console.log('\n==================', activationData);
        console.log(`激活提示: ${message}`);
        console.log(`验证码: ${code}`);
        console.log('请访问', this.config.authUrl, '输入上述验证码');
        console.log('设备名称:', deviceInfo.name);
        console.log(`当前设备: ${this.deviceFingerprint.deviceId}`);
        console.log('==================\n');

        return await this.activate(challenge);
    }

    /**
     * 执行激活
     */
    async activate(challenge) {
        const serialNumber = await this.deviceFingerprint.getSerialNumber();
        if (!serialNumber) {
            throw new Error('设备没有序列号，无法完成激活');
        }

        const hmacSignature = await this.deviceFingerprint.generateHmac(challenge);

        // 检查设备类型，确定是否为虚拟设备
        const deviceInfo = await this.multiDeviceManager.getDeviceInfo(this.deviceFingerprint.deviceId);
        const isVirtualDevice = deviceInfo && deviceInfo.device_type === 'virtual';

        let fingerprint;
        if (isVirtualDevice) {
            // 虚拟设备：使用虚拟MAC地址生成指纹
            const virtualMac = deviceInfo.virtual_mac;
            fingerprint = await this.deviceFingerprint.generateFingerprint(true, virtualMac);
        } else {
            // 物理设备：使用默认方式
            fingerprint = await this.deviceFingerprint.generateFingerprint();
        }

        const payload = {
            Payload: {
                algorithm: 'hmac-sha256',
                serial_number: serialNumber,
                challenge: challenge,
                hmac: hmacSignature
            }
        };

        const headers = {
            'Activation-Version': '2',
            'Device-Id': fingerprint.mac_address,
            'Client-Id': this.config.clientId,
            'Content-Type': 'application/json'
        };

        const activateUrl = this.config.otaUrl.replace(/\/$/, '') + '/activate';

        console.log(`开始激活设备 (${this.deviceFingerprint.deviceId})...`);

        for (let attempt = 1; attempt <= this.config.maxRetries; attempt++) {
            try {
                console.log(`尝试激活 (${attempt}/${this.config.maxRetries})...`);

                const response = await axios.post(activateUrl, payload, {
                    headers,
                    timeout: 10000
                });

                console.log(`\n激活响应 (HTTP ${response.status}):`);
                console.log(JSON.stringify(response.data, null, 2));

                if (response.status === 200) {
                    console.log(`\n*** 设备激活成功! (${this.deviceFingerprint.deviceId}) ***\n`);
                    await this.deviceFingerprint.setActivationStatus(true);
                    return true;
                } else if (response.status === 202) {
                    console.log('\n等待用户在网站输入验证码，继续等待...\n');
                    await this.sleep(this.config.retryInterval);
                } else {
                    console.log(`\n服务器返回状态码 ${response.status}，继续等待...\n`);
                    await this.sleep(this.config.retryInterval);
                }
            } catch (error) {
                if (error.response) {
                    const errorMsg = (error.response.data && error.response.data.error) || `HTTP ${error.response.status}`;
                    console.log(`\n服务器返回: ${errorMsg}，继续等待验证码激活...\n`);

                    if (errorMsg.includes('Device not found') && attempt % 5 === 0) {
                        console.log('\n提示: 如果错误持续出现，可能需要在网站上刷新页面获取新验证码\n');
                    }
                } else {
                    console.log(`激活过程中发生错误: ${error.message}，重试中...`);
                }

                await this.sleep(this.config.retryInterval);
            }
        }

        console.log('\n激活失败，达到最大等待时间，请重新获取验证码并尝试激活\n');
        return false;
    }

    /**
     * 启动激活流程
     */
    async start(deviceId = null, forceActivation = false) {
        try {
            console.log('设备激活器启动...');

            // 确保设备身份
            const identity = await this.ensureDeviceIdentity(deviceId);

            // 检查是否已激活（除非强制激活）
            if (identity.isActivated && !forceActivation) {
                console.log(`设备已激活，无需重复激活 (${identity.deviceId})`);
                return true;
            }

            // 对于虚拟设备或强制激活，直接请求激活流程
            const deviceInfo = await this.multiDeviceManager.getDeviceInfo(identity.deviceId);
            const isVirtualDevice = deviceInfo && deviceInfo.device_type === 'virtual';
            let statusResponse;
            if (isVirtualDevice || forceActivation || !identity.isActivated) {
                console.log(`开始${isVirtualDevice ? '虚拟' : ''}设备激活流程 (${identity.deviceId})...`);

                // 检查设备状态
                try {
                    statusResponse = await this.checkDeviceStatus();
                    console.log('检查设备状态:', statusResponse);
                } catch (error) {
                    console.log('检查设备状态失败，直接进入激活流程:', error.message);
                    // 如果状态检查失败，模拟一个激活请求
                    statusResponse = {
                        activation: {
                            challenge: `challenge_${Date.now()}_${Math.random().toString(36).substring(2)}`,
                            code: this.generateRandomCode(),
                            message: '请在xiaozhi.me输入验证码完成设备激活'
                        }
                    };
                }

                if (statusResponse.activation) {
                    // 需要激活
                    console.log('检测到激活请求，开始设备激活流程');
                    return await this.processActivation(statusResponse.activation);
                } else {
                    // 服务器返回无需激活的处理
                    if (!identity.isActivated || forceActivation) {
                        const reason = !identity.isActivated ? '本地状态为未激活' : '强制激活模式';
                        console.log(`服务器返回无需激活，但${reason} ...`);
                        // 生成本地激活流程
                        return false;
                    } else {
                        // 已激活且服务器确认，非强制模式
                        console.log('设备状态正常，无需激活');
                        if (statusResponse.mqtt || statusResponse.websocket) {
                            console.log('配置信息:', JSON.stringify({
                                mqtt: statusResponse.mqtt,
                                websocket: statusResponse.websocket
                            }, null, 2));
                        }
                        return true;
                    }
                }
            } else {
                console.log(`设备已激活，无需重复激活 (${identity.deviceId})`);
                return true;
            }
        } catch (error) {
            console.error('激活流程失败:', error.message);
            return false;
        }
    }

    /**
     * 生成随机验证码
     */
    generateRandomCode() {
        const chars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
        let result = '';
        for (let i = 0; i < 6; i++) {
            result += chars.charAt(Math.floor(Math.random() * chars.length));
        }
        return result;
    }

    /**
     * 显示设备管理菜单
     */
    async showDeviceMenu() {

        const rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout
        });

        const question = (prompt) => new Promise(resolve => rl.question(prompt, resolve));

        while (true) {
            console.log('\n========== 多设备管理菜单 ==========');
            console.log('1. 列出所有设备');
            console.log('2. 创建虚拟设备');
            console.log('3. 切换设备');

            console.log('5. 激活指定设备');
            console.log('6. 查看设备详情');
            console.log('7. 删除设备');
            console.log('8. 退出');
            console.log('=====================================\n');

            const choice = await question('请选择操作 (1-8): ');

            try {
                switch (choice) {
                    case '1':
                        await this.listAllDevices();
                        break;
                    case '2':
                        await this.createVirtualDeviceInteractive(question);
                        break;
                    case '3':
                        await this.switchDeviceInteractive(question);
                        break;

                    case '5':
                        await this.activateSpecificDeviceInteractive(question);
                        break;
                    case '6':
                        await this.showDeviceDetailsInteractive(question);
                        break;
                    case '7':
                        await this.deleteDeviceInteractive(question);
                        break;
                    case '8':
                        console.log('退出设备管理');
                        rl.close();
                        return;
                    default:
                        console.log('无效选择，请重试');
                }
            } catch (error) {
                console.error('操作失败:', error.message);
            }
        }
    }

    /**
     * 列出所有设备
     */
    async listAllDevices() {
        const devices = await this.multiDeviceManager.listDevices();
        const currentDeviceId = await this.multiDeviceManager.getCurrentDevice();

        console.log('\n========== 设备列表 ==========');
        if (Object.keys(devices).length === 0) {
            console.log('暂无注册设备');
        } else {
            for (const [deviceId, info] of Object.entries(devices)) {
                const status = deviceId === currentDeviceId ? ' [当前]' : '';
                console.log(`ID: ${deviceId}${status}`);
                console.log(`  名称: ${info.name}`);
                console.log(`  类型: ${info.type}`);
                console.log(`  序列号: ${info.serial_number}`);
                console.log(`  注册时间: ${info.registered_time}`);
                console.log(`  最后使用: ${info.last_used}`);
                console.log('');
            }
        }
        console.log('==============================\n');
    }

    /**
     * 交互式创建虚拟设备
     */
    async createVirtualDeviceInteractive(question) {
        const deviceName = await question('输入设备名称 (回车使用默认名称): ');
        const result = await this.multiDeviceManager.createVirtualDevice(deviceName || null);
        console.log('\n虚拟设备创建成功!');
    }

    /**
     * 交互式切换设备
     */
    async switchDeviceInteractive(question) {
        await this.listAllDevices();
        const deviceId = await question('输入要切换的设备ID: ');
        await this.multiDeviceManager.setCurrentDevice(deviceId);
    }

    /**
     * 交互式激活指定设备
     */
    async activateSpecificDeviceInteractive(question) {
        await this.listAllDevices();
        const deviceId = await question('输入要激活的设备ID: ');
        // const forceReactivation = await question('是否强制重新激活? (y/N): ');
        // const forceActivation = forceReactivation.toLowerCase() === 'y';
        await this.start(deviceId, true);
    }

    /**
     * 交互式查看设备详情
     */
    async showDeviceDetailsInteractive(question) {
        await this.listAllDevices();
        const deviceId = await question('输入要查看的设备ID: ');
        const info = await this.multiDeviceManager.getDeviceInfo(deviceId);

        if (info) {
            console.log('\n========== 设备详情 ==========');
            console.log(JSON.stringify(info, null, 2));
            console.log('==============================\n');
        } else {
            console.log('设备不存在');
        }
    }

    /**
     * 交互式删除设备
     */
    async deleteDeviceInteractive(question) {
        await this.listAllDevices();
        const deviceId = await question('输入要删除的设备ID: ');
        const confirm = await question(`确认删除设备 ${deviceId}? (y/N): `);

        if (confirm.toLowerCase() === 'y') {
            await this.multiDeviceManager.deleteDevice(deviceId);
        } else {
            console.log('取消删除');
        }
    }

    /**
     * 延时函数
     */
    sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
}

export { DeviceFingerprint, DeviceActivator, MultiDeviceManager };