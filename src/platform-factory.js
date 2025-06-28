// 浏览器适配器 - 基础版本
import { BrowserAudioRecorder } from './adapters/browser/browser-audio-recorder.js';
import { BrowserWebSocket } from './adapters/browser/browser-websocket.js';

// 浏览器适配器 - Opus 版本 
import { BrowserAudioPlayerOpus } from './adapters/browser/browser-audio-player-opus.js';

/**
 * 平台检测工具
 */
export class PlatformDetector {
    /**
     * 检测当前运行环境
     * @returns {string} 'node' | 'browser' | 'unknown'
     */
    static detectPlatform() {
        // 检查是否在浏览器环境
        if (typeof window !== 'undefined' && typeof document !== 'undefined') {
            return 'browser';
        }

        // 检查是否在 Node.js 环境
        if (typeof process !== 'undefined' &&
            process.versions &&
            process.versions.node) {
            return 'node';
        }

        // 检查是否在 Web Worker 环境
        if (typeof importScripts !== 'undefined') {
            return 'worker'; // Web Worker 暂时按浏览器处理
        }

        return 'unknown';
    }

    /**
     * 检查浏览器功能支持
     */
    static checkBrowserSupport() {
        if (typeof window === 'undefined') {
            return { supported: false, reason: '非浏览器环境' };
        }

        const support = {
            getUserMedia: !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia),
            webAudio: !!(window.AudioContext || window.webkitAudioContext),
            webSocket: !!window.WebSocket,
            crypto: !!(window.crypto && window.crypto.getRandomValues),
            webAssembly: !!window.WebAssembly,
            opusDecoder: !!window['opus-decoder']
        };

        const unsupported = Object.entries(support)
            .filter(([key, value]) => !value)
            .map(([key]) => key);

        if (unsupported.length > 0) {
            return {
                supported: false,
                reason: `浏览器不支持以下功能: ${unsupported.join(', ')}`,
                details: support
            };
        }

        return { supported: true, details: support };
    }
}

/**
 * 平台工厂类
 * 根据运行环境自动选择合适的音频录音器、播放器和 WebSocket 实现
 */
export class PlatformFactory {
    /**
     * 获取当前平台信息
     */
    static getPlatformInfo() {
        if (typeof window !== 'undefined' && window.AudioContext) {
            return {
                platform: 'browser',
                support: true,
                features: {
                    audioContext: true,
                    webSocket: true,
                    mediaDevices: !!navigator.mediaDevices
                }
            };
        }

        if (typeof process !== 'undefined' && process.versions && process.versions.node) {
            return {
                platform: 'node',
                support: true,
                features: {
                    audify: true,
                    ws: true
                }
            };
        }

        return {
            platform: 'unknown',
            support: false,
            reason: '不支持的运行环境'
        };
    }

    /**
     * 创建音频录音器
     * @param {object} options 录音器配置选项
     * @returns {Promise<IAudioRecorder>} 音频录音器实例
     */
    static async createAudioRecorder(options = {}) {
        const info = this.getPlatformInfo();
        if (!info.support) {
            throw new Error(`不支持的平台: ${info.reason}`);
        }

        switch (info.platform) {
            case 'browser':
                return new BrowserAudioRecorder(options);
            case 'node':
                {
                    const { NodeAudioRecorder } = await
                    import ('./adapters/node/node-audio-recorder.js');
                    return new NodeAudioRecorder(options);
                }
            default:
                throw new Error(`未知平台: ${info.platform}`);
        }
    }

    /**
     * 创建音频播放器
     * @param {object} options 播放器配置选项
     * @returns {Promise<IAudioPlayer>} 音频播放器实例
     */
    static async createAudioPlayer(options = {}) {
        const info = this.getPlatformInfo();
        if (!info.support) {
            throw new Error(`不支持的平台: ${info.reason}`);
        }

        switch (info.platform) {
            case 'browser':
                console.log('✅ 使用 Opus 播放器');
                return new BrowserAudioPlayerOpus(options);

            case 'node':
                {
                    const { NodeAudioPlayer } = await
                    import ('./adapters/node/node-audio-player.js');
                    return new NodeAudioPlayer(options);
                }
            default:
                throw new Error(`未知平台: ${info.platform}`);
        }
    }

    /**
     * 创建 WebSocket 协议处理器
     * @param {object} config WebSocket 配置
     * @returns {Promise<IWebSocketProtocol>} WebSocket 协议处理器实例
     */
    static async createWebSocketProtocol(config) {
        const info = this.getPlatformInfo();
        if (!info.support) {
            throw new Error(`不支持的平台: ${info.reason}`);
        }

        switch (info.platform) {
            case 'browser':
                return new BrowserWebSocket(config);
            case 'node':
                {
                    const { NodeWebSocketProtocol } = await
                    import ('./adapters/node/node-websocket.js');
                    return new NodeWebSocketProtocol(config);
                }
            default:
                throw new Error(`未知平台: ${info.platform}`);
        }
    }


}