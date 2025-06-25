# Node.js 设备激活器

基于小智项目激活机制的Node.js设备激活系统，实现设备身份验证、HMAC签名验证和验证码激活流程。

## 特性

- 🔒 **安全认证**: 基于HMAC-SHA256的设备身份验证
- 🔑 **设备指纹**: 自动生成基于硬件信息的唯一设备标识
- 📱 **验证码激活**: 防止自动化攻击的人工验证机制
- 🔄 **自动重试**: 智能轮询等待激活完成
- 🛠️ **跨平台**: 支持Windows、macOS、Linux
- 📝 **持久化**: 本地存储设备身份和激活状态
- 🎯 **易集成**: 简单的API接口，易于集成到现有项目

## 安装

```bash
# 克隆项目
git clone <your-repo-url>
cd device-activator

# 安装依赖
npm install
```

### 依赖说明

- `axios`: HTTP客户端，用于与服务器通信
- `clipboardy`: 可选依赖，用于复制验证码到剪贴板
- `open`: 可选依赖，用于自动打开浏览器

## 快速开始

### 基本使用

```javascript
const { DeviceActivator } = require('./device-activator');

// 创建激活器实例
const activator = new DeviceActivator({
    otaUrl: 'https://api.tenclass.net/xiaozhi/ota/',
    authUrl: 'https://xiaozhi.me/',
    maxRetries: 60,
    retryInterval: 5000
});

// 启动激活流程
activator.start()
    .then(success => {
        if (success) {
            console.log('设备激活成功!');
        } else {
            console.log('设备激活失败');
        }
    })
    .catch(error => {
        console.error('激活过程发生错误:', error);
    });
```

### 运行示例

```bash
# 基本激活示例
npm start

# 或者
node device-activator.js

# 运行不同类型的示例
node example.js fingerprint  # 查看设备指纹
node example.js manual      # 手动激活流程
node example.js batch       # 批量设备管理
node example.js all         # 运行所有示例
```

## API 文档

### DeviceActivator

设备激活管理器主类。

#### 构造函数

```javascript
new DeviceActivator(config)
```

**参数:**
- `config` (Object): 配置选项
  - `otaUrl` (string): OTA服务器URL，默认: `'https://api.tenclass.net/xiaozhi/ota/'`
  - `authUrl` (string): 授权网站URL，默认: `'https://xiaozhi.me/'`
  - `clientId` (string): 客户端ID，默认: 自动生成UUID
  - `maxRetries` (number): 最大重试次数，默认: `60`
  - `retryInterval` (number): 重试间隔(毫秒)，默认: `5000`

#### 方法

##### start()

启动激活流程。

```javascript
const success = await activator.start();
```

**返回值:** `Promise<boolean>` - 激活是否成功

##### ensureDeviceIdentity()

确保设备身份信息已创建。

```javascript
const { serialNumber, isActivated } = await activator.ensureDeviceIdentity();
```

**返回值:** `Promise<Object>` - 包含序列号和激活状态

##### checkDeviceStatus()

检查设备在服务器上的状态。

```javascript
const statusResponse = await activator.checkDeviceStatus();
```

**返回值:** `Promise<Object>` - 服务器响应数据

##### processActivation(activationData)

处理激活流程。

```javascript
const success = await activator.processActivation({
    challenge: 'server-challenge-string',
    code: '123456',
    message: '请在xiaozhi.me输入验证码'
});
```

**参数:**
- `activationData` (Object): 激活数据
  - `challenge` (string): 服务器挑战字符串
  - `code` (string): 验证码
  - `message` (string): 提示信息

**返回值:** `Promise<boolean>` - 激活是否成功

### DeviceFingerprint

设备指纹收集器，用于生成设备唯一标识。

#### 构造函数

```javascript
new DeviceFingerprint()
```

#### 方法

##### generateFingerprint()

生成设备指纹。

```javascript
const fingerprint = await deviceFingerprint.generateFingerprint();
```

**返回值:** `Promise<Object>` - 设备指纹信息

##### generateSerialNumber()

生成设备序列号。

```javascript
const { serial, source } = await deviceFingerprint.generateSerialNumber();
```

**返回值:** `Promise<Object>` - 序列号和生成来源

##### isActivated()

检查设备是否已激活。

```javascript
const isActivated = await deviceFingerprint.isActivated();
```

**返回值:** `Promise<boolean>` - 是否已激活

##### generateHmac(challenge)

生成HMAC签名。

```javascript
const signature = await deviceFingerprint.generateHmac('challenge-string');
```

**参数:**
- `challenge` (string): 挑战字符串

**返回值:** `Promise<string>` - HMAC签名

## 激活流程

### 1. 设备身份生成

系统自动基于硬件信息生成：
- **序列号**: 基于MAC地址的唯一标识，格式 `SN-{hash}-{mac}`
- **HMAC密钥**: 基于硬件指纹的SHA256哈希
- **配置文件**: 存储在 `~/.device-config/efuse.json`

### 2. 服务器通信

```
设备 → 服务器: 发送设备信息请求状态
服务器 → 设备: 返回激活请求(含验证码和挑战)
设备: 显示验证码，等待用户操作
用户: 在网站输入验证码
设备 → 服务器: 发送HMAC签名验证
服务器 → 设备: 确认激活成功
```

### 3. 安全机制

- **设备唯一性**: 基于硬件指纹，难以伪造
- **密码学验证**: HMAC-SHA256签名保证身份真实性
- **人工验证**: 验证码防止批量自动激活
- **时效性**: 激活请求有超时机制

## 配置文件

### efuse.json

设备身份信息存储文件，位于 `~/.device-config/efuse.json`:

```json
{
  "serial_number": "SN-A1B2C3D4-aa:bb:cc:dd:ee:ff",
  "hmac_key": "0123456789abcdef...",
  "activation_status": false,
  "created_at": "2024-01-01T00:00:00.000Z",
  "activated_at": null
}
```

### .device_fingerprint.json

设备指纹缓存文件:

```json
{
  "system": "darwin",
  "hostname": "MacBook-Pro.local",
  "mac_address": "aa:bb:cc:dd:ee:ff",
  "mac_type": "WiFi网卡",
  "cpu": {
    "model": "Apple M1",
    "cores": 8,
    "arch": "arm64",
    "platform": "darwin"
  },
  "system_serial": "C02XJ0ABML85",
  "timestamp": 1704067200000
}
```

## 服务器接口

### 设备状态检查

**请求:**
```
POST /xiaozhi/ota/
Content-Type: application/json

{
  "version": 2,
  "mac_address": "aa:bb:cc:dd:ee:ff",
  "uuid": "client-uuid",
  "hostname": "device-hostname",
  "serial_number": "SN-xxx",
  "system": "darwin",
  "cpu": {...},
  "timestamp": 1704067200000
}
```

**响应 (需要激活):**
```json
{
  "activation": {
    "message": "请访问xiaozhi.me输入激活码",
    "code": "123456",
    "challenge": "random-challenge-string",
    "timeout_ms": 30000
  }
}
```

**响应 (已激活):**
```json
{
  "mqtt": {
    "endpoint": "mqtt.server.com",
    "client_id": "device123",
    "username": "user",
    "password": "pass"
  },
  "websocket": {
    "url": "wss://api.server.com/ws/",
    "token": "access-token"
  }
}
```

### 设备激活

**请求:**
```
POST /xiaozhi/ota/activate
Content-Type: application/json

{
  "Payload": {
    "algorithm": "hmac-sha256",
    "serial_number": "SN-xxx",
    "challenge": "challenge-string",
    "hmac": "hmac-signature"
  }
}
```

**响应:**
- `200`: 激活成功
- `202`: 等待用户输入验证码
- `4xx`: 激活失败

## 错误处理

### 常见错误

1. **网络连接失败**
   ```
   Error: connect ECONNREFUSED
   ```
   检查网络连接和服务器地址。

2. **HMAC签名错误**
   ```
   Error: 无法生成HMAC签名
   ```
   检查efuse.json文件是否正确生成。

3. **验证码超时**
   ```
   HTTP 202: 等待用户输入验证码
   ```
   用户需要在网站及时输入验证码。

### 调试模式

设置环境变量启用详细日志：

```bash
DEBUG=device-activator node device-activator.js
```

## 高级配置

### 自定义服务器

```javascript
const activator = new DeviceActivator({
    otaUrl: 'https://your-server.com/api/device/',
    authUrl: 'https://your-portal.com/',
    maxRetries: 30,
    retryInterval: 3000
});
```

### 批量设备管理

```javascript
const devices = [
    { name: 'Device-001', clientId: 'uuid-1' },
    { name: 'Device-002', clientId: 'uuid-2' }
];

for (const device of devices) {
    const activator = new DeviceActivator({
        clientId: device.clientId
    });
    
    const success = await activator.start();
    console.log(`${device.name}: ${success ? '成功' : '失败'}`);
}
```

### 集成到现有项目

```javascript
const { DeviceActivator } = require('./device-activator');

class MyDeviceManager {
    constructor() {
        this.activator = new DeviceActivator({
            otaUrl: process.env.OTA_URL,
            authUrl: process.env.AUTH_URL
        });
    }
    
    async initialize() {
        const isActivated = await this.activator.deviceFingerprint.isActivated();
        
        if (!isActivated) {
            console.log('设备未激活，开始激活流程...');
            const success = await this.activator.start();
            
            if (!success) {
                throw new Error('设备激活失败');
            }
        }
        
        console.log('设备已激活，可以正常使用');
        return true;
    }
}
```

## 贡献

欢迎提交Issue和Pull Request来改进这个项目。

## 许可证

MIT License

## 更新日志

### v1.0.0
- 初始版本
- 实现基本的设备激活功能
- 支持HMAC签名验证
- 支持验证码激活流程
- 跨平台硬件指纹生成 