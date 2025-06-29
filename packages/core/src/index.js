/**
 * 小智客户端使用示例
 * 展示如何在 Node.js 和浏览器环境下使用统一的 API
 */

import { Client } from './client.js';
import { PlatformFactory } from './platform-factory.js';
import { checkDeviceStatus } from './device-status.js';
import { DeviceActivator } from './device-activator.js';
import { BaseAwakener, KeyboardAwakener, HttpAwakener } from './device-awaker.js';

export {
    Client,
    PlatformFactory,
    checkDeviceStatus,
    DeviceActivator,
    BaseAwakener,
    KeyboardAwakener,
    HttpAwakener
};