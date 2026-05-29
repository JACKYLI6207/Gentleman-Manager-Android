@echo off
chcp 65001 >nul
cd /d "%~dp0"
title Gentleman Manager - 完整 APK
echo.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-apk.ps1" -Mode Full
set ERR=%ERRORLEVEL%
echo.
if %ERR% neq 0 (
  echo [失敗] 錯誤碼 %ERR%
  if /i not "%1"=="--auto" pause
  exit /b %ERR%
)
if /i not "%1"=="--auto" pause
exit /b 0
