const { DeviceActivator, DeviceFingerprint } = require('./device-activator');

/**
 * 设备指纹测试
 */
async function testDeviceFingerprint() {
    console.log('🔍 测试设备指纹功能...\n');

    const fingerprint = new DeviceFingerprint();

    try {
        // 测试设备指纹生成
        console.log('1. 生成设备指纹...');
        const deviceInfo = await fingerprint.generateFingerprint();
        console.log('✅ 设备指纹生成成功');
        console.log('   - 系统:', deviceInfo.system);
        console.log('   - 主机名:', deviceInfo.hostname);
        console.log('   - MAC地址:', deviceInfo.mac_address);

        // 测试序列号生成
        console.log('\n2. 生成设备序列号...');
        const serialInfo = await fingerprint.generateSerialNumber();
        console.log('✅ 序列号生成成功');
        console.log('   - 序列号:', serialInfo.serial);
        console.log('   - 来源:', serialInfo.source);

        // 测试硬件哈希
        console.log('\n3. 生成硬件哈希...');
        const hardwareHash = await fingerprint.generateHardwareHash();
        console.log('✅ 硬件哈希生成成功');
        console.log('   - 哈希值:', hardwareHash.substring(0, 32) + '...');

        // 测试efuse文件
        console.log('\n4. 测试efuse文件管理...');
        await fingerprint.ensureEfuseFile();
        const isActivated = await fingerprint.isActivated();
        console.log('✅ efuse文件管理正常');
        console.log('   - 激活状态:', isActivated ? '已激活' : '未激活');

        return true;
    } catch (error) {
        console.error('❌ 设备指纹测试失败:', error.message);
        return false;
    }
}

/**
 * 激活器配置测试
 */
async function testActivatorConfig() {
    console.log('\n🔧 测试激活器配置...\n');

    try {
        // 测试默认配置
        console.log('1. 测试默认配置...');
        const activator1 = new DeviceActivator();
        console.log('✅ 默认配置创建成功');
        console.log('   - OTA URL:', activator1.config.otaUrl);
        console.log('   - 认证URL:', activator1.config.authUrl);
        console.log('   - 客户端ID:', activator1.config.clientId.substring(0, 8) + '...');

        // 测试自定义配置
        console.log('\n2. 测试自定义配置...');
        const customConfig = {
            otaUrl: 'https://test-server.com/api/',
            authUrl: 'https://test-auth.com/',
            clientId: 'test-client-123',
            maxRetries: 10,
            retryInterval: 2000
        };

        const activator2 = new DeviceActivator(customConfig);
        console.log('✅ 自定义配置创建成功');
        console.log('   - OTA URL:', activator2.config.otaUrl);
        console.log('   - 最大重试:', activator2.config.maxRetries);
        console.log('   - 重试间隔:', activator2.config.retryInterval);

        // 测试设备身份确保
        console.log('\n3. 测试设备身份确保...');
        const { serialNumber, isActivated } = await activator1.ensureDeviceIdentity();
        console.log('✅ 设备身份确保成功');
        console.log('   - 序列号:', serialNumber);
        console.log('   - 激活状态:', isActivated);

        return true;
    } catch (error) {
        console.error('❌ 激活器配置测试失败:', error.message);
        return false;
    }
}

/**
 * HMAC签名测试
 */
async function testHmacSigning() {
    console.log('\n🔐 测试HMAC签名功能...\n');

    const fingerprint = new DeviceFingerprint();

    try {
        // 确保有HMAC密钥
        await fingerprint.ensureEfuseFile();
        const hmacKey = await fingerprint.getHmacKey();

        if (!hmacKey) {
            throw new Error('未找到HMAC密钥');
        }

        console.log('1. HMAC密钥检查...');
        console.log('✅ HMAC密钥存在');
        console.log('   - 密钥长度:', hmacKey.length);
        console.log('   - 密钥预览:', hmacKey.substring(0, 16) + '...');

        // 测试签名生成
        const testChallenge = 'test-challenge-' + Date.now();
        console.log('\n2. 测试签名生成...');
        console.log('   - 挑战字符串:', testChallenge);

        const signature1 = await fingerprint.generateHmac(testChallenge);
        console.log('✅ 第一次签名成功');
        console.log('   - 签名:', signature1.substring(0, 16) + '...');

        // 测试签名一致性
        const signature2 = await fingerprint.generateHmac(testChallenge);
        console.log('\n3. 测试签名一致性...');

        if (signature1 === signature2) {
            console.log('✅ 签名一致性验证成功');
            console.log('   - 相同输入产生相同签名');
        } else {
            throw new Error('签名不一致');
        }

        // 测试不同挑战的不同签名
        const differentChallenge = 'different-challenge-' + Date.now();
        const signature3 = await fingerprint.generateHmac(differentChallenge);

        if (signature1 !== signature3) {
            console.log('✅ 不同输入产生不同签名');
        } else {
            console.log('⚠️  警告: 不同输入产生了相同签名 (概率极低但可能)');
        }

        return true;
    } catch (error) {
        console.error('❌ HMAC签名测试失败:', error.message);
        return false;
    }
}

/**
 * 网络连接测试(模拟)
 */
async function testNetworkConnection() {
    console.log('\n🌐 测试网络连接功能...\n');

    const activator = new DeviceActivator({
        // 使用测试服务器，预期会失败
        otaUrl: 'https://httpbin.org/status/404',
        maxRetries: 2,
        retryInterval: 1000
    });

    try {
        console.log('1. 测试网络请求处理...');
        console.log('   (使用测试URL，预期会失败)');

        await activator.checkDeviceStatus();
        console.log('❓ 意外成功 - 这不应该发生');

    } catch (error) {
        console.log('✅ 网络错误处理正常');
        console.log('   - 错误类型:', error.name);
        console.log('   - 是否为HTTP错误:', error.response ? '是' : '否');

        if (error.response) {
            console.log('   - 状态码:', error.response.status);
        }
    }

    // 测试无效URL
    try {
        console.log('\n2. 测试无效URL处理...');
        const invalidActivator = new DeviceActivator({
            otaUrl: 'invalid-url',
            maxRetries: 1,
            retryInterval: 500
        });

        await invalidActivator.checkDeviceStatus();
        console.log('❓ 意外成功 - 这不应该发生');

    } catch (error) {
        console.log('✅ 无效URL错误处理正常');
        console.log('   - 错误信息:', error.message.substring(0, 50) + '...');
    }

    return true;
}

/**
 * 文件系统测试
 */
async function testFileSystem() {
    console.log('\n📁 测试文件系统功能...\n');

    const fingerprint = new DeviceFingerprint();

    try {
        console.log('1. 测试配置目录创建...');
        console.log('   - 配置目录:', fingerprint.configDir);

        await fingerprint.ensureEfuseFile();
        console.log('✅ 配置目录和文件创建成功');

        console.log('\n2. 测试文件读写...');
        const testData = { test: true, timestamp: Date.now() };

        // 测试缓存写入
        await fingerprint.cacheFingerprint(testData);
        console.log('✅ 缓存文件写入成功');

        // 测试缓存读取
        const cachedData = await fingerprint.loadCachedFingerprint();
        if (cachedData && cachedData.test === true) {
            console.log('✅ 缓存文件读取成功');
        } else {
            throw new Error('缓存数据不匹配');
        }

        console.log('\n3. 测试efuse数据管理...');
        const originalStatus = await fingerprint.isActivated();

        // 切换激活状态
        await fingerprint.setActivationStatus(!originalStatus);
        const newStatus = await fingerprint.isActivated();

        if (newStatus !== originalStatus) {
            console.log('✅ 激活状态更新成功');
        } else {
            throw new Error('激活状态未更新');
        }

        // 恢复原状态
        await fingerprint.setActivationStatus(originalStatus);
        console.log('✅ 状态恢复成功');

        return true;
    } catch (error) {
        console.error('❌ 文件系统测试失败:', error.message);
        return false;
    }
}

/**
 * 运行所有测试
 */
async function runAllTests() {
    console.log('🧪 Node.js 设备激活器 - 功能测试\n');
    console.log('='.repeat(50));

    const tests = [
        { name: '设备指纹', fn: testDeviceFingerprint },
        { name: '激活器配置', fn: testActivatorConfig },
        { name: 'HMAC签名', fn: testHmacSigning },
        { name: '网络连接', fn: testNetworkConnection },
        { name: '文件系统', fn: testFileSystem }
    ];

    const results = [];

    for (const test of tests) {
        console.log(`\n${'='.repeat(50)}`);

        try {
            const success = await test.fn();
            results.push({ name: test.name, success, error: null });
        } catch (error) {
            console.error(`❌ ${test.name}测试异常:`, error.message);
            results.push({ name: test.name, success: false, error: error.message });
        }
    }

    // 显示测试结果摘要
    console.log(`\n${'='.repeat(50)}`);
    console.log('📊 测试结果摘要\n');

    let passedCount = 0;
    for (const result of results) {
        const status = result.success ? '✅ 通过' : '❌ 失败';
        console.log(`${status} - ${result.name}`);
        if (result.error) {
            console.log(`   错误: ${result.error}`);
        }
        if (result.success) passedCount++;
    }

    console.log(`\n总计: ${passedCount}/${results.length} 测试通过`);

    if (passedCount === results.length) {
        console.log('🎉 所有测试都通过了！');
        return true;
    } else {
        console.log('⚠️  部分测试失败，请检查错误信息');
        return false;
    }
}

// 主程序
const args = process.argv.slice(2);
const testType = args[0] || 'all';

switch (testType) {
    case 'fingerprint':
        testDeviceFingerprint();
        break;
    case 'config':
        testActivatorConfig();
        break;
    case 'hmac':
        testHmacSigning();
        break;
    case 'network':
        testNetworkConnection();
        break;
    case 'file':
        testFileSystem();
        break;
    case 'all':
    default:
        runAllTests().then(success => {
            process.exit(success ? 0 : 1);
        });
        break;
}