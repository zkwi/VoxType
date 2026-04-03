param(
    [string]$PythonExe = "C:\Users\zkwi\miniconda3\envs\Quantitative-investment\python.exe",
    [bool]$PrintTranscriptToConsole = $true
)

$ErrorActionPreference = "Stop"

Set-Location -LiteralPath $PSScriptRoot

$env:VOICE_INPUT_PRINT_TRANSCRIPT_TO_CONSOLE = $PrintTranscriptToConsole.ToString().ToLowerInvariant()

& $PythonExe .\main.py
if ($LASTEXITCODE -ne 0) {
    throw "程序运行失败，退出码: $LASTEXITCODE"
}
