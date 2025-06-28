const otaUrl = 'https://api.tenclass.net/xiaozhi/ota/';

export async function checkDeviceStatus(name, device_id) {
    if (!device_id) {
        throw new Error('设备ID不能为空');
    }

    let clientId;
    // 平台兼容：浏览器和 Node.js
    if (typeof window !== 'undefined' && window.crypto) {
        clientId = window.crypto.randomUUID();
    } else {
        const crypto = await
        import ('crypto');
        clientId = crypto.randomUUID();
    }

    // 请求头
    const headers = {
        "Activation-Version": "2",
        "Device-Id": device_id,
        "Client-Id": clientId,
        "Content-Type": "application/json",
        "User-Agent": name + "/1.0.0"
    };

    // 请求体
    const payload = {};

    // console.log('##payload', headers, payload);

    try {
        const response = await fetch(otaUrl, {
            method: 'POST',
            headers: headers,
            body: JSON.stringify(payload)
        });

        if (!response.ok) {
            const errorText = await response.text();
            console.error('服务器响应:', errorText);
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const result = await response.json();
        if (result.activation) {
            //需要激活
            return null;
        } else {
            //正常使用
            return result;
        }

    } catch (error) {
        console.error('检查设备状态失败:', error.message);
        throw error;
    }
}