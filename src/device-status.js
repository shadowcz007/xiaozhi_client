import axios from 'axios';
import crypto from 'crypto';

export async function checkDeviceStatusBase(otaUrl, clientId, deviceInfo = {}) {
    // 虚拟设备：使用虚拟MAC地址生成指纹
    const virtualMac = deviceInfo.fingerprint.mac_address;
    const serialNumber = deviceInfo.serial_number;

    // console.log('deviceInfo', deviceInfo.name )
    const headers = {
        'Activation-Version': '2',
        'Device-Id': virtualMac,
        'Client-Id': clientId,
        'Content-Type': 'application/json',
        'User-Agent': deviceInfo.name + '/1.0.0'
    };

    const payload = {
        version: 2,
        mac_address: virtualMac,
        uuid: clientId,
        hostname: deviceInfo.fingerprint.hostname,
        serial_number: serialNumber,
        system: deviceInfo.fingerprint.system,
        cpu: deviceInfo.fingerprint.cpu,
        timestamp: Date.now(),
        device_id: virtualMac
    };

    try {
        console.log(`正在检查设备状态 (${virtualMac})...`);
        const response = await axios.post(otaUrl, payload, { headers });
        return response.data;
    } catch (error) {
        console.error('检查设备状态失败:', error.response && error.response.data || error.message);
        throw error;
    }
}


if (
    import.meta.url ===
    import.meta.resolve(process.argv[1])) {

    const clientId = crypto.randomUUID();
    const deviceInfo = {
        "name": "mix",
        "type": "virtual",
        "serial_number": "SN-53C05F84-f268cbb2c593",
        "registered_time": "2025-06-26T12:23:31.253Z",
        "last_used": "2025-06-26T12:23:31.253Z",
        "fingerprint": {
            "system": "darwin",
            "hostname": "shadowdeMacBook-Pro-2.local",
            "mac_address": "f2:68:cb:b2:c5:93",
            "mac_type": "虚拟网卡",
            "cpu": {
                "model": "Intel(R) Core(TM) i5-8259U CPU @ 2.30GHz",
                "cores": 8,
                "arch": "x64",
                "platform": "darwin"
            },
            "system_serial": "C02XJ07TJHC9",
            "device_id": "f2:68:cb:b2:c5:93",
            "is_virtual": true,
            "fingerprint_created_at": "2025-06-26T12:23:31.253Z",
            "activation_status": false,
            "device_name": "mix",
            "device_type": "virtual",
            "virtual_mac": "f2:68:cb:b2:c5:93"
        }
    }



    checkDeviceStatusBase('https://ota.xiaozhi.com', clientId, deviceInfo).then(res => {
        console.log('checkDeviceStatusBase', res);
    });

}