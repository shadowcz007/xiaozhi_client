// server.js
import { file } from "bun";

Bun.serve({
    port: 8443,
    tls: {
        cert: file("cert.pem"),
        key: file("key.pem"),
    },
    async fetch(req) {
        const url = new URL(req.url);

        // 根路径返回您的 HTML 文件
        if (url.pathname === "/" || url.pathname === "/index.html") {
            return new Response(file("xiaozhi-simple-demo.html"), {
                headers: {
                    "Content-Type": "text/html",
                },
            });
        }

        // 处理其他静态文件
        try {
            const filePath = url.pathname.slice(1); // 移除开头的 /
            return new Response(file(filePath));
        } catch {
            return new Response("Not Found", { status: 404 });
        }
    },
});

console.log("🚀 HTTPS 服务器运行在 https://localhost:8443");
console.log("📄 访问您的小智演示页面：https://localhost:8443");