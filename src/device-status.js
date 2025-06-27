import crypto from 'crypto';

const otaUrl = 'https://api.tenclass.net/xiaozhi/ota/';
const clientId = crypto.randomUUID();

export async function checkDeviceStatus(payload, headers) {
    try {
        const response = await fetch(otaUrl, {
            method: 'POST',
            headers: headers,
            body: JSON.stringify(payload)
        });

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        return await response.json();
    } catch (error) {
        console.error('检查设备状态失败:', error.message);
        throw error;
    }
}

if (
    import.meta.url ===
    import.meta.resolve(process.argv[1])) {

    const data = {
        "device_id": "df:52:34:be:fa:38",
        "payload": {},
        "headers": {
            "Activation-Version": "2",
            "Device-Id": "df:52:34:be:fa:38",
            "Client-Id": clientId,
            "Content-Type": "application/json",
            "User-Agent": "mix/1.0.0"
        }
    }

    const payload = data.payload;
    const headers = data.headers;

    const result = await checkDeviceStatus(payload, headers);
    if (result.activation) {
        //需要激活
    } else {
        //正常使用

    }
    console.log('##result', result);
}