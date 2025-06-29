// 管理设备

import { DeviceActivator } from '@xiaozhi/core';


// 如果直接运行此文件，则启动激活流程
if (
    import.meta.url ===
    import.meta.resolve(process.argv[1])) {
    const args = process.argv.slice(2);

    const activator = new DeviceActivator({
        // 可以在这里自定义配置
        // otaUrl: 'https://your-custom-server.com/ota/',
        // authUrl: 'https://your-auth-site.com/',
    });

    // 检查命令行参数
    if (args.includes('--menu')) {
        // 显示设备管理菜单
        activator.showDeviceMenu()
            .then(() => {
                console.log('设备管理完成');
                process.exit(0);
            })
            .catch(error => {
                console.error('设备管理异常:', error);
                process.exit(1);
            });
    } else {
        // 默认激活流程
        const deviceId = args.find(arg => arg.startsWith('--device='))?.split('=')[1];
        const forceActivation = args.includes('--force');

        if (forceActivation) {
            console.log('强制激活模式已启用');
        }

        activator.start(deviceId, forceActivation)
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
}