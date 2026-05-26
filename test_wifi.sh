#!/bin/bash
# WiFi 状态测试脚本

echo "=== WiFi 状态测试 ==="
echo

echo "1. networksetup -getairportnetwork en0"
networksetup -getairportnetwork en0
echo

echo "2. ipconfig getifaddr en0"
ipconfig getifaddr en0
echo

echo "3. networksetup -listallhardwareports | grep -A 3 'Wi-Fi'"
networksetup -listallhardwareports | grep -A 3 "Wi-Fi"
echo

echo "4. system_profiler SPNetworkDataType -json (Wi-Fi section)"
system_profiler SPNetworkDataType -json 2>/dev/null | grep -A 30 '"_name" : "Wi-Fi"'
echo

echo "=== 编译测试 ==="
cd "$(dirname "$0")/rust"

echo "编译中..."
cargo build --release 2>&1 | tail -2
echo

echo "=== 运行程序测试 ==="
cd ..

# 启动程序测试（后台，5秒后终止）
( sleep 5 && pkill -f xiaozhi_client 2>/dev/null ) &
./rust/target/release/xiaozhi_client --device-id 9b:9b:f3:50:dc:17 2>&1 | head -20
kill %1 2>/dev/null || true

echo
echo "=== 测试完成 ==="