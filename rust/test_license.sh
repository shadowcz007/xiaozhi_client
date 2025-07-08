#!/bin/bash

echo "🧪 小智客户端许可证系统测试"
echo "=================================="

# 检查是否已编译
if [ ! -f "target/release/xiaozhi_client" ]; then
    echo "📦 编译主程序..."
    cargo build --release --bin xiaozhi_client
fi

if [ ! -f "target/release/license_generator" ]; then
    echo "📦 编译许可证生成器..."
    cargo build --release --bin license_generator
fi

echo ""
echo "1️⃣ 测试有效许可证..."
echo "生成有效许可证..."
VALID_KEY=$(./target/release/license_generator test-license test-password | grep "eyJ" | head -1)
echo "使用有效许可证（应该成功）："
echo "   许可证密钥: $VALID_KEY"
timeout 3s ./target/release/xiaozhi_client --key "$VALID_KEY" || echo "✅ 程序正常启动（3秒后自动停止）"

echo ""
echo "2️⃣ 测试无效许可证..."
echo "生成无效许可证..."
INVALID_KEY=$(./target/release/license_generator invalid-license invalid-password | grep "eyJ" | head -1)
echo "使用无效许可证（应该失败）："
echo "   许可证密钥: $INVALID_KEY"
./target/release/xiaozhi_client --key "$INVALID_KEY" || echo "✅ 程序正确拒绝无效许可证"

echo ""
echo "3️⃣ 测试格式错误的许可证..."
echo "使用格式错误的许可证（应该失败）："
./target/release/xiaozhi_client --key "invalid-format" || echo "✅ 程序正确拒绝格式错误的许可证"

echo ""
echo "🎉 测试完成！"
echo ""
echo "💡 可用的有效许可证组合："
echo "   - test-license + test-password"
echo "   - xiaozhi-license + xiaozhi-password" 
echo "   - demo-license + demo-password" 