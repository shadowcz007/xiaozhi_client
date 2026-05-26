#!/bin/bash
set -e

REPO="shadowcz007/xiaozhi_client"
BINARY_NAME="xiaozhi_client"
INSTALL_DIR="/usr/local/bin"

# 检测操作系统
OS="$(uname -s)"
ARCH="$(uname -m)"

# 确定平台
case "$OS" in
    Linux*)
        case "$ARCH" in
            x86_64) PLATFORM="x86_64-unknown-linux-gnu" ;;
            aarch64|arm64) PLATFORM="aarch64-unknown-linux-gnu" ;;
            *) echo "不支持的架构: $ARCH"; exit 1 ;;
        esac
        ;;
    Darwin*)
        case "$ARCH" in
            x86_64) PLATFORM="x86_64-apple-darwin" ;;
            arm64) PLATFORM="aarch64-apple-darwin" ;;
            *) echo "不支持的架构: $ARCH"; exit 1 ;;
        esac
        ;;
    *)
        echo "不支持的操作系统: $OS"
        exit 1
        ;;
esac

# 获取最新版本号
echo "检测最新版本..."
VERSION=$(curl -s https://api.github.com/repos/${REPO}/releases/latest | grep -o '"tag_name":.*' | cut -d'"' -f4)
if [ -z "$VERSION" ]; then
    echo "获取版本失败，使用 v1.0.0"
    VERSION="v1.0.0"
fi

echo "最新版本: $VERSION"
echo "平台: $PLATFORM"

# 下载链接
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY_NAME}-${PLATFORM}"
TEMP_FILE="/tmp/${BINARY_NAME}-${PLATFORM}"

echo "下载中..."
curl -L -o "$TEMP_FILE" "$DOWNLOAD_URL"

# 验证文件
if [ ! -s "$TEMP_FILE" ]; then
    echo "下载失败，文件为空"
    exit 1
fi

# 安装
echo "安装到 ${INSTALL_DIR}..."
sudo cp "$TEMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
rm "$TEMP_FILE"

echo "安装成功！"
echo "运行: xiaozhi_client --manage"