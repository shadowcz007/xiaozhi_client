const crypto = require('crypto');
const fs = require('fs').promises;
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');
const axios = require('axios');

/**
 * 设备指纹收集器 - 用于生成唯一的设备标识
 */
class DeviceFingerprint {
    constructor() {
        this.system = os.platform();
        this.configDir = '.device-config'
        this.fingerprintCacheFile = path.join(this.configDir, '.device_fingerprint.json');
        this.efuseFile = path.join(this.configDir, 'efuse.json');
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

            // 优先选择以太网，然后WiFi
            const priorityOrder = ['Ethernet', 'Wi-Fi', 'en0', 'eth0', 'wlan0'];

            for (const name of priorityOrder) {
                if (networkInterfaces[name]) {
                    const infaces = networkInterfaces[name].find(net => !net.internal && net.mac !== '00:00:00:00:00:00');
                    if (infaces) {
                        return {
                            mac: infaces.mac.toLowerCase(),
                            type: name.includes('Wi-Fi') || name.includes('wlan') ? 'WiFi网卡' : '有线网卡'
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
                model: cpus[0]?.model || 'Unknown',
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
                    .find(line => line.includes('SerialNumber='))?.split('=')[1]?.trim();
            } else if (this.system === 'darwin') {
                serial = execSync('system_profiler SPHardwareDataType | grep "Serial Number"', { encoding: 'utf8' })
                    .split(':')[1]?.trim();
            } else if (this.system === 'linux') {
                try {
                    serial = execSync('sudo dmidecode -s system-serial-number', { encoding: 'utf8' }).trim();
                } catch {
                    // 如果没有sudo权限，尝试其他方法
                    serial = execSync('cat /sys/class/dmi/id/product_serial 2>/dev/null || echo "unknown"', { encoding: 'utf8' }).trim();
                }
            }

            return serial && serial !== 'unknown' ? serial : null;
        } catch (error) {
            console.warn('获取系统序列号失败:', error.message);
            return null;
        }
    }

    /**
     * 生成设备指纹
     */
    async generateFingerprint() {
        // 检查缓存
        const cached = await this.loadCachedFingerprint();
        if (cached) return cached;

        const macInfo = this.getMacAddress();
        const cpuInfo = this.getCpuInfo();
        const systemSerial = this.getSystemSerial();

        const fingerprint = {
            system: this.system,
            hostname: this.getHostname(),
            mac_address: macInfo?.mac,
            mac_type: macInfo?.type,
            cpu: cpuInfo,
            system_serial: systemSerial,
            timestamp: Date.now(),
            mixlab_client_id: this.mixlab_client_id
        };

        // 缓存指纹
        await this.cacheFingerprint(fingerprint);
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
            console.log('设备指纹已缓存');
        } catch (error) {
            console.error('缓存设备指纹失败:', error.message);
        }
    }

    /**
     * 生成硬件哈希
     */
    async generateHardwareHash() {
        const fingerprint = await this.generateFingerprint();

        const identifiers = [
            fingerprint.hostname,
            fingerprint.mac_address,
            fingerprint.cpu?.model,
            fingerprint.system_serial,
            fingerprint.system
        ].filter(Boolean);

        const fingerprintStr = identifiers.join('||');
        return crypto.createHash('sha256').update(fingerprintStr).digest('hex');
    }

    /**
     * 生成设备序列号
     */
    async generateSerialNumber() {
        const fingerprint = await this.generateFingerprint();

        if (fingerprint.mac_address) {
            const macClean = fingerprint.mac_address.replace(/:/g, '');
            const shortHash = crypto.createHash('md5').update(macClean).digest('hex').substring(0, 8).toUpperCase();
            return {
                serial: `SN-${shortHash}-${macClean}`,
                source: fingerprint.mac_type || '网络接口'
            };
        }

        // 备选方案：使用硬件哈希
        const hardwareHash = await this.generateHardwareHash();
        return {
            serial: `SN-${hardwareHash.substring(0, 16).toUpperCase()}`,
            source: '硬件哈希值'
        };
    }

    /**
     * 确保efuse文件存在
     */
    async ensureEfuseFile() {
        try {
            await fs.access(this.efuseFile);
            console.log('efuse配置文件已存在');
        } catch {
            // 文件不存在，创建新的
            const { serial, source } = await this.generateSerialNumber();
            const hmacKey = await this.generateHardwareHash();

            console.log(`生成序列号: ${serial} (来源: ${source})`);
            console.log(`生成HMAC密钥: ${hmacKey.substring(0, 16)}...`);

            const defaultData = {
                serial_number: serial,
                hmac_key: hmacKey,
                activation_status: false,
                created_at: new Date().toISOString()
            };

            await fs.mkdir(this.configDir, { recursive: true });
            await fs.writeFile(this.efuseFile, JSON.stringify(defaultData, null, 2));
            console.log('新设备：已创建efuse配置文件');
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
            console.error('加载efuse数据失败:', error.message);
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
            console.error('保存efuse数据失败:', error.message);
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
}

/**
 * 设备激活管理器
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
        console.log('config',this.config);

        this.deviceFingerprint = new DeviceFingerprint();
    }

    /**
     * 生成客户端ID
     */
    generateClientId() {
        return crypto.randomUUID();
    }

    /**
     * 确保设备身份信息
     */
    async ensureDeviceIdentity() {
        await this.deviceFingerprint.ensureEfuseFile();

        const serialNumber = await this.deviceFingerprint.getSerialNumber();
        const isActivated = await this.deviceFingerprint.isActivated();

        console.log(`设备身份信息: 序列号: ${serialNumber}, 激活状态: ${isActivated ? '已激活' : '未激活'}`);

        return { serialNumber, isActivated };
    }

    /**
     * 检查设备状态
     */
    async checkDeviceStatus() {
        const fingerprint = await this.deviceFingerprint.generateFingerprint();
        const serialNumber = await this.deviceFingerprint.getSerialNumber();

        const headers = {
            'Activation-Version': '2',
            'Device-Id': fingerprint.mac_address,
            'Client-Id': this.config.clientId,
            'Content-Type': 'application/json',
            'User-Agent': 'Mixlab/1.0.0'
        };

        const payload = {
            version: 2,
            mac_address: fingerprint.mac_address,
            uuid: this.config.clientId,
            hostname: fingerprint.hostname,
            serial_number: serialNumber,
            system: fingerprint.system,
            cpu: fingerprint.cpu,
            timestamp: Date.now()
        };

        try {
            console.log('正在检查设备状态...');
            const response = await axios.post(this.config.otaUrl, payload, { headers });
            return response.data;
        } catch (error) {
            console.error('检查设备状态失败:', error.response?.data || error.message);
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

        const { challenge, code, message = '请在xiaozhi.me输入验证码' } = activationData;

        console.log('\n==================',activationData);
        console.log(`激活提示: ${message}`);
        console.log(`验证码: ${code.split('').join(' ')}`);
        console.log('请访问', this.config.authUrl, '输入上述验证码');
        console.log('==================\n');

        // 复制验证码到剪贴板（如果可用）
        try {
            const clipboardy = require('clipboardy');
            await clipboardy.write(code);
            console.log('验证码已复制到剪贴板');
        } catch {
            console.log('无法复制到剪贴板，请手动复制验证码');
        }

        // 尝试打开浏览器
        try {
            const open = require('open');
            await open(this.config.authUrl);
            console.log('已尝试打开浏览器');
        } catch {
            console.log('无法自动打开浏览器，请手动访问激活网址');
        }

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
        const fingerprint = await this.deviceFingerprint.generateFingerprint();

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

        console.log('开始激活设备...');

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
                    console.log('\n*** 设备激活成功! ***\n');
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
                    const errorMsg = error.response.data?.error || `HTTP ${error.response.status}`;
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
    async start() {
        try {
            console.log('设备激活器启动...');

            // 确保设备身份
            await this.ensureDeviceIdentity();

            // 检查是否已激活
            const isActivated = await this.deviceFingerprint.isActivated();
            if (isActivated) {
                console.log('设备已激活，无需重复激活');
                return true;
            }

            // 检查设备状态
            const statusResponse = await this.checkDeviceStatus();

            if (statusResponse.activation) {
                // 需要激活
                console.log('检测到激活请求，开始设备激活流程');
                return await this.processActivation(statusResponse.activation);
            } else {
                // 已激活或其他状态
                console.log('设备状态正常，无需激活');
                if (statusResponse.mqtt || statusResponse.websocket) {
                    console.log('配置信息:', JSON.stringify({
                        mqtt: statusResponse.mqtt,
                        websocket: statusResponse.websocket
                    }, null, 2));
                }
                return true;
            }
        } catch (error) {
            console.error('激活流程失败:', error.message);
            return false;
        }
    }

    /**
     * 延时函数
     */
    sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
}

module.exports = { DeviceFingerprint, DeviceActivator };

// 如果直接运行此文件，则启动激活流程
if (require.main === module) {
    const activator = new DeviceActivator({
        // 可以在这里自定义配置
        // otaUrl: 'https://your-custom-server.com/ota/',
        // authUrl: 'https://your-auth-site.com/',
    });

    activator.start()
        .then(success => {
            if (success) {
                console.log('激活流程完成');
                process.exit(0);
            } else {
                console.log('激活失败');
                process.exit(1);
            }
        })
        .catch(error => {
            console.error('激活流程异常:', error);
            process.exit(1);
        });
}