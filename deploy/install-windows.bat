@echo off
REM hyper-node Windows installer (service + key protection + firewall)
REM ASCII only. Run in PowerShell/cmd as Administrator.
REM Usage: install-windows.bat [your-api-key]

setlocal
set URL=https://github.com/saves24/HyperScope/releases/latest/download/hyper-node-windows-amd64.exe
set BIN=%ProgramData%\hyper-node\hyper-node.exe
set DIR=%ProgramData%\hyper-node

echo ==^> Creating directory %DIR%
if not exist "%DIR%" mkdir "%DIR%"

echo ==^> Downloading hyper-node...
powershell -NoProfile -Command "Invoke-WebRequest -Uri '%URL%' -OutFile '%BIN%'"
if errorlevel 1 goto :fail

echo ==^> Setting up key...
if "%~1"=="" (
  echo    No key given - run manually: "%BIN%" key setup ^<your-key^>
) else (
  "%BIN%" key setup %~1
)

echo ==^> Protecting key file (SYSTEM + Administrators only)...
if exist "%DIR%\key" (
  icacls "%DIR%\key" /inheritance:r /grant:r "SYSTEM:(R)" "Administrators:(R)" >nul 2>&1
)

echo ==^> Opening firewall port 5000 (private + domain networks only)...
netsh advfirewall firewall delete rule name="hyper-node" 2>nul
netsh advfirewall firewall add rule name="hyper-node" dir=in action=allow protocol=TCP localport=5000 profile=private,domain
if errorlevel 1 goto :fail

echo ==^> Removing old scheduled task (if any)...
schtasks /End /TN "hyper-node" 2>nul
schtasks /Delete /F /TN "hyper-node" 2>nul

echo ==^> Registering as a Windows service (starts at boot, no logon required)...
sc stop hyper-node 2>nul
sc delete hyper-node 2>nul
sc create hyper-node binPath= "\"%BIN%\" service" start= auto DisplayName= "hyper-node - system monitoring collector"
if errorlevel 1 goto :fail
sc description hyper-node "HyperScope collector: monitors this machine and serves metrics to the panel." >nul 2>&1

echo ==^> Starting service now...
sc start hyper-node

echo.
echo ==^> Done! hyper-node runs as a service (auto-start at boot, no logon needed).
echo    Config: %DIR%  (key, mode, cert)
echo    Manual start:  sc start hyper-node
echo    Manual stop:   sc stop hyper-node
echo.
echo ==^> Optional reverse-push mode (no inbound port):
echo    Register the node in the panel, then run:
echo    "%BIN%" connect http://^<panel-host^>:8089 ^<node-name^> ^<node-key^>
goto :eof

:fail
echo.
echo FAILED. Run this script as Administrator.
exit /b 1
