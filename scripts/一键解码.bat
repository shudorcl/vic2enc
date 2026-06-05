@echo off
chcp 936 >nul
setlocal
title Vic2 编码转换 - 解码（游戏格式 → 可读中文）
cd /d "%~dp0"

echo ============================================
echo   Vic2 编码转换  ·  解码（游戏格式 → 可读）
echo ============================================
echo.

if not exist "vic2enc.exe" (
  echo [错误] 找不到 vic2enc.exe
  echo        请把本脚本和 vic2enc.exe 放在同一个文件夹里再运行。
  echo.
  pause
  exit /b 1
)

set "TARGET=%~1"
if "%TARGET%"=="" (
  echo 请把要转换的【文件】或【localisation 文件夹】拖到本窗口，
  echo 然后按回车确认：
  set /p "TARGET=> "
)
rem 去掉路径可能带的引号
set "TARGET=%TARGET:"=%"

if "%TARGET%"=="" (
  echo [错误] 没有提供路径。
  echo.
  pause
  exit /b 1
)
if not exist "%TARGET%" (
  echo [错误] 路径不存在：%TARGET%
  echo.
  pause
  exit /b 1
)

set "OUT=%TARGET%_decoded"
echo.
echo 输入：%TARGET%
echo 输出：%OUT%
echo 正在解码（游戏 GBK 伪 Latin-1  →  可读 UTF-8）...
echo.
"vic2enc.exe" decode -i "%TARGET%" -o "%OUT%"
if errorlevel 1 (
  echo.
  echo [失败] 转换出错，请把上面的提示截图反馈。
) else (
  echo.
  echo [完成] 已输出到：
  echo        %OUT%
)
echo.
pause
