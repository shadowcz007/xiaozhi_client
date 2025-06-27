import crypto from 'crypto';

const otaUrl = 'https://api.tenclass.net/xiaozhi/ota/';

export async function checkDeviceStatus(device_id) {

    const clientId = crypto.randomUUID();
    const data = {
        "device_id": device_id,
        "payload": {},
        "headers": {
            "Activation-Version": "2",
            "Device-Id": device_id,
            "Client-Id": clientId,
            "Content-Type": "application/json",
            "User-Agent": "mix/1.0.0"
        }
    }

    const payload = data.payload;
    const headers = data.headers;

    try {
        const response = await fetch(otaUrl, {
            method: 'POST',
            headers: headers,
            body: JSON.stringify(payload)
        });

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const result = await response.json();
        if (result.activation) {
            //需要激活
            return
        } else {
            //正常使用
            return result
        }

    } catch (error) {
        console.error('检查设备状态失败:', error.message);
        throw error;
    }
}