import { nodeClient } from './src/index.js';

// 设备ID，需要替换为实际的设备ID
const deviceId = '9b:9b:f3:50:dc:17';
const name = 'goodmate';

async function main() {
    await nodeClient(name, deviceId);
}

main();