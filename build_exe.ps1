param(
    [string]$PythonExe = "python"
)

$ErrorActionPreference = "Stop"

Set-Location -LiteralPath $PSScriptRoot

if (Test-Path -LiteralPath ".\build") {
    Remove-Item -Recurse -Force ".\build"
}
if (Test-Path -LiteralPath ".\dist") {
    Remove-Item -Recurse -Force ".\dist"
}

& $PythonExe -m pip install -r requirements.txt
if ($LASTEXITCODE -ne 0) { throw "安装 requirements 失败" }
& $PythonExe -m pip install pyinstaller
if ($LASTEXITCODE -ne 0) { throw "安装 pyinstaller 失败" }
& $PythonExe -m PyInstaller --clean --noconfirm voice_input.spec
if ($LASTEXITCODE -ne 0) { throw "PyInstaller 打包失败" }

Copy-Item -LiteralPath ".\config.json" -Destination ".\dist\voice_input\config.json" -Force

Write-Host ""
Write-Host "构建完成：dist\\voice_input\\voice_input.exe"
Write-Host "分发时请一起带上：dist\\voice_input\\config.json"
