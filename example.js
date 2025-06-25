const { DeviceActivator, DeviceFingerprint } = require('./device-activator');

/**
 * 基本使用示例
 */
async function basicExample() {
    console.log('=== 基本激活示例 ===\n');

    // 创建激活器实例
    const activator = new DeviceActivator({
        otaUrl: 'https://api.tenclass.net/xiaozhi/ota/',
        authUrl: 'https://xiaozhi.me/',
        maxRetries: 30, // 最多重试30次
        retryInterval: 3000 // 每3秒重试一次
    });

    try {
        const success = await activator.start();
        if (success) {
            console.log('✅ 设备激活成功!');
        } else {
            console.log('❌ 设备激活失败');
        }
    } catch (error) {
        console.error('激活过程发生错误:', error.message);
    }
}

/**
 * 设备指纹检查示例
 */
async function fingerprintExample() {
    console.log('\n=== 设备指纹检查示例 ===\n');

    const fingerprint = new DeviceFingerprint();

    try {
        // 生成设备指纹
        const deviceInfo = await fingerprint.generateFingerprint();
        console.log('设备指纹信息:');
        console.log('- 系统:', deviceInfo.system);
        console.log('- 主机名:', deviceInfo.hostname);
        console.log('- MAC地址:', deviceInfo.mac_address);
        console.log('- 网卡类型:', deviceInfo.mac_type);
        console.log('- CPU型号:', deviceInfo.cpu ? deviceInfo.cpu.model : 'Unknown');
        console.log('- CPU核心数:', deviceInfo.cpu ? deviceInfo.cpu.cores : 'Unknown');

        // 生成序列号
        const serialInfo = await fingerprint.generateSerialNumber();
        console.log('\n序列号信息:');
        console.log('- 序列号:', serialInfo.serial);
        console.log('- 生成来源:', serialInfo.source);

        // 生成硬件哈希
        const hardwareHash = await fingerprint.generateHardwareHash();
        console.log('- 硬件哈希:', hardwareHash.substring(0, 16) + '...');

        // 检查激活状态
        await fingerprint.ensureEfuseFile();
        const isActivated = await fingerprint.isActivated();
        console.log('- 激活状态:', isActivated ? '已激活' : '未激活');

    } catch (error) {
        console.error('获取设备指纹失败:', error.message);
    }
}

/**
 * 手动激活示例
 */
async function manualActivationExample() {
    console.log('\n=== 手动激活示例 ===\n');

    const activator = new DeviceActivator();

    try {
        // 1. 确保设备身份
        await activator.ensureDeviceIdentity();

        // 2. 检查设备状态
        console.log('正在检查设备状态...');
        const statusResponse = await activator.checkDeviceStatus();

        // 3. 处理响应
        if (statusResponse.activation) {
            console.log('需要激活，激活信息:');
            console.log('- 验证码:', statusResponse.activation.code);
            console.log('- 提示信息:', statusResponse.activation.message);
            console.log('- 挑战字符串:', statusResponse.activation.challenge);

            // 开始激活流程
            const success = await activator.processActivation(statusResponse.activation);
            console.log('激活结果:', success ? '成功' : '失败');
        } else {
            console.log('设备无需激活或已激活');
            if (statusResponse.mqtt) {
                console.log('MQTT配置:', statusResponse.mqtt);
            }
            if (statusResponse.websocket) {
                console.log('WebSocket配置:', statusResponse.websocket);
            }
        }

    } catch (error) {
        console.error('手动激活过程失败:', error.message);
    }
}

/**
 * 批量设备管理示例
 */
async function batchDeviceExample() {
    console.log('\n=== 批量设备管理示例 ===\n');

    const devices = [
        { name: 'Device-001', clientId: 'device-001-uuid' },
        { name: 'Device-002', clientId: 'device-002-uuid' },
        { name: 'Device-003', clientId: 'device-003-uuid' }
    ];

    console.log('模拟批量设备激活管理:');

    for (const device of devices) {
        console.log(`\n处理设备: ${device.name}`);

        const activator = new DeviceActivator({
            // clientId: device.clientId,
            maxRetries: 5 // 减少重试次数用于演示
        });

        try {
            const { serialNumber, isActivated } = await activator.ensureDeviceIdentity();
            console.log(`- 序列号: ${serialNumber}`);
            console.log(`- 状态: ${isActivated ? '已激活' : '未激活'}`);

            if (!isActivated) {
                console.log(`- ${device.name} 需要激活`);
                // 在实际环境中，这里会执行激活流程
                const success = await activator.start();
                console.log('激活结果:', success ? '成功' : '失败');
            }
        } catch (error) {
            console.error(`- ${device.name} 处理失败:`, error.message);
        }
    }
}

// 主函数
async function main() {
    console.log('Node.js 设备激活器示例\n');

    // 根据命令行参数选择示例
    const args = process.argv.slice(2);
    const exampleType = args[0] || 'basic';

    switch (exampleType) {
        case 'basic':
            await basicExample();
            break;
        case 'custom':
            await customConfigExample();
            break;
        case 'fingerprint':
            await fingerprintExample();
            break;
        case 'manual':
            await manualActivationExample();
            break;
        case 'batch':
            await batchDeviceExample();
            break;
        case 'all':
            await basicExample();
            await customConfigExample();
            await fingerprintExample();
            await manualActivationExample();
            await batchDeviceExample();
            break;
        default:
            console.log('使用方法:');
            console.log('  node example.js [type]');
            console.log('');
            console.log('示例类型:');
            console.log('  basic      - 基本激活示例');
            console.log('  custom     - 自定义配置示例');
            console.log('  fingerprint- 设备指纹检查示例');
            console.log('  manual     - 手动激活示例');
            console.log('  batch      - 批量设备管理示例');
            console.log('  all        - 运行所有示例');
            break;
    }
}

// 运行示例
batchDeviceExample()