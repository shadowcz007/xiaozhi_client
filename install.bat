@echo off
setlocal enabledelayedexpansion

set "REPO=shadowcz007/xiaozhi_client"
set "BINARY_NAME=xiaozhi_client.exe"
set "INSTALL_DIR=%USERPROFILE%\AppData\Local\Programs\xiaozhi_client"

echo ==========================================
echo   小智语音助手 - Windows 安装程序
echo ==========================================
echo.

:: 检测架构
if "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
    set "PLATFORM=x86_64-pc-windows-msvc"
) else if "%PROCESSOR_ARCHITECTURE%"=="ARM64" (
    set "PLATFORM=aarch64-pc-windows-msvc"
) else (
    echo 不支持的架构: %PROCESSOR_ARCHITECTURE%
    exit /b 1
)

echo 平台: %PLATFORM%
echo.

:: 获取最新版本
echo 检测最新版本...
for /f "delims=" %%i in ('powershell -Command "(Invoke-WebRequest -Uri 'https://api.github.com/repos/%REPO%/releases/latest' -UseBasicParsing).Content | ConvertFrom-Json | Select-Object -ExpandProperty tag_name"') do set "VERSION=%%i"
if not defined VERSION set "VERSION=v1.0.0"

echo 最新版本: %VERSION%
echo.

:: 下载
set "DOWNLOAD_URL=https://github.com/%REPO%/releases/download/%VERSION%/%BINARY_NAME%"
set "TEMP_FILE=%TEMP%\%BINARY_NAME%"

echo 下载中...
powershell -Command "Invoke-WebRequest -Uri '%DOWNLOAD_URL%' -OutFile '%TEMP_FILE%'"

if not exist "%TEMP_FILE%" (
    echo 下载失败
    exit /b 1
)

:: 创建安装目录
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
)

:: 安装
echo 安装到 %INSTALL_DIR%...
copy /y "%TEMP_FILE%" "%INSTALL_DIR%\%BINARY_NAME%"
del "%TEMP_FILE%"

:: 添加到 PATH
setx PATH "%INSTALL_DIR%;%PATH%" >nul 2>&1

echo.
echo ==========================================
echo   安装成功！
echo ==========================================
echo.
echo 请重新打开命令行窗口使 PATH 生效
echo 运行: xiaozhi_client --manage
echo.
pause