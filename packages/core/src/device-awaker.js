import http from 'http';

/**
 * 唤醒器基类 (BaseAwakener)
 * 定义了所有设备唤醒器插件的通用接口。
 * 插件的职责是监听外部事件（如按键、HTTP请求等），
 * 并在事件发生时调用 client.startVoiceChat() 来唤醒设备。
 */
export class BaseAwakener {
    constructor() {
        this.client = null;
    }

    /**
     * 将唤醒器附加到一个 Client 实例上。
     * @param {import('./client').Client} client - 客户端实例
     */
    attach(client) {
        this.client = client;
        console.log(`🔌 [${this.constructor.name}] 已附加到客户端。`);
    }

    /**
     * 启动唤醒器，开始监听触发事件。
     * 子类必须实现此方法。
     */
    start() {
        throw new Error(`Awakener [${this.constructor.name}] 必须实现 start() 方法。`);
    }

    /**
     * 停止唤醒器，清理监听器。
     * 子类必须实现此方法。
     */
    stop() {
        throw new Error(`Awakener [${this.constructor.name}] 必须实现 stop() 方法。`);
    }

    /**
     * 供子类调用的标准唤醒方法。
     * 检查客户端是否存在，并启动语音聊天。
     */
    _awake() {
        if (!this.client) {
            console.warn(`⚠️ [${this.constructor.name}] 唤醒失败：没有附加的客户端。`);
            return;
        }

        if (this.client.deviceState === 'idle') {
            console.log(`🚀 [${this.constructor.name}] 触发唤醒，启动语音聊天...`);
            this.client.startVoiceChat();
        } else {
            console.log(`⚠️ [${this.constructor.name}] 触发唤醒，但客户端不处于 idle 状态 (当前: ${this.client.deviceState})，无需操作。`);
        }
    }
}

/**
 * 键盘唤醒器 (KeyboardAwakener)
 * 监听终端的回车键来唤醒设备。
 */
export class KeyboardAwakener extends BaseAwakener {
    constructor() {
        super();
        this.handleKeyPress = this.handleKeyPress.bind(this);
    }

    handleKeyPress(data) {
        if (data.toString().trim() === '') { // 检测回车
            this._awake();
        }
    }

    start() {
        console.log('⌨️  键盘唤醒器已启动。在终端按 [Enter] 键来唤醒设备。');
        // 'data' 事件监听器在 Node.js 中用于流式读取
        process.stdin.on('data', this.handleKeyPress);
        process.stdin.resume(); // 开始读取标准输入
    }

    stop() {
        console.log('⌨️  键盘唤醒器已停止。');
        process.stdin.pause(); // 暂停读取
        process.stdin.off('data', this.handleKeyPress);
    }
}


/**
 * HTTP 唤醒器 (HttpAwakener)
 * 通过一个 HTTP POST 请求来唤醒设备。
 */
export class HttpAwakener extends BaseAwakener {
    constructor(port = 8088) {
        super();
        this.port = port;
        this.server = null;
    }

    start() {
        this.server = http.createServer((req, res) => {
            if (req.method === 'POST' && req.url === '/awake') {
                this._awake();
                res.writeHead(200, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({ message: 'Awakening triggered' }));
            } else {
                res.writeHead(404, { 'Content-Type': 'text/plain' });
                res.end('Not Found');
            }
        });

        this.server.listen(this.port, () => {
            console.log(`🌐 HTTP 唤醒器已启动，监听端口 ${this.port}。`);
            console.log(`   运行 'curl -X POST http://localhost:${this.port}/awake' 来唤醒。`);
        });
    }

    stop() {
        if (this.server) {
            this.server.close(() => {
                console.log('🌐 HTTP 唤醒器已停止。');
            });
        }
    }
}