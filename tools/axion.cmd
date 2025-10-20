@echo off
setlocal

set SCRIPT_DIR=%~dp0
if "%SCRIPT_DIR:~-1%"=="\" set SCRIPT_DIR=%SCRIPT_DIR:~0,-1%

set CMD=%1
if /I "%CMD%"=="plan" (
    shift
) else if /I "%CMD%"=="run" (
    shift
) else (
    set CMD=run
)

python "%SCRIPT_DIR%\axion_runner.py" %CMD% %*
endlocal
