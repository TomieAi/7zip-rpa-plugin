@echo off
echo Installing Rpa.64.dll to 7-Zip Formats...
copy /Y "%~dp0Rpa.64.dll" "C:\Program Files\7-Zip\Formats\Rpa.64.dll"
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Access denied. Please right-click install.cmd and select "Run as administrator".
    pause
    exit /b %errorlevel%
)
echo [SUCCESS] Installed Rpa.64.dll into C:\Program Files\7-Zip\Formats\Rpa.64.dll!
pause

