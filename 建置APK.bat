@echo off
chcp 65001 >nul
cd /d "%~dp0"
title Gentleman Manager - 建置 APK（完整版，相容舊用法）
echo.
echo 提示：日常測試可改用「建置APK-快速.bat」；參數 --fast / --full 可指定種類。
echo.
set BUILD_MODE=Full
if /i "%1"=="--fast" set BUILD_MODE=Fast
if /i "%2"=="--fast" set BUILD_MODE=Fast
if /i "%1"=="--full" set BUILD_MODE=Full
if /i "%2"=="--full" set BUILD_MODE=Full
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-apk.ps1" -Mode %BUILD_MODE%
set ERR=%ERRORLEVEL%
echo.
if %ERR% neq 0 (
  echo [失敗] 錯誤碼 %ERR%
  if /i not "%1"=="--auto" pause
  exit /b %ERR%
)
if /i not "%1"=="--auto" pause
exit /b 0
