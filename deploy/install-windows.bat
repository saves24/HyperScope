@echo off
REM hyper-node Windows installer (relay-mode collector + hyper-relay service)
REM ASCII only. Run in PowerShell/cmd as Administrator.
REM Usage: install-windows.bat [your-api-key]

setlocal
set URL=https://github.com/saves24/HyperScope/releases/latest/download/hyper-node-windows-amd64.exe
set RELAY_URL=https://github.com/saves24/HyperScope/releases/latest/download/hyper-relay-windows-amd64.exe
set BIN=%ProgramData%\hyper-node\hyper-node.exe
set RELAY_BIN=%ProgramData%\hyper-node\hyper-relay.exe
set DIR=%ProgramData%\hyper-node

echo ==^> Creating directory %DIR%
if not exist "%DIR%" mkdir "%DIR%"

echo ==^> Downloading hyper-node...
powershell -NoProfile -Command "Invoke-WebRequest -Uri '%URL%' -OutFile '%BIN%'"
if errorlevel 1 goto :fail

echo ==^> Downloading hyper-relay (same-machine relay agent)...
powershell -NoProfile -Command "Invoke-WebRequest -Uri '%RELAY_URL%' -OutFile '%RELAY_BIN%'"
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

REM Relay mode (protocol v0.2): the collector has NO listening port, so no
REM inbound firewall rule is created. Metrics are served through hyper-relay
REM on demand; outbound connections are allowed by default.

echo ==^> Removing old scheduled task (if any)...
schtasks /End /TN "hyper-node" 2>nul
schtasks /Delete /F /TN "hyper-node" 2>nul

REM hyper-node is NOT registered as a service: it is woken on demand by
REM hyper-relay (spawns "hyper-node.exe collect" per poll, then exits).
REM hyper-relay is a native Windows service (its `service` command registers
REM a ServiceMain with SCM).

REM The relay port (8686) must accept inbound connections from the panel.
REM Use profile=any: the machine's network may be classified Public (Wi-Fi).
echo ==^> Opening firewall port 8686...
netsh advfirewall firewall delete rule name="hyper-relay" 2>nul
netsh advfirewall firewall add rule name="hyper-relay" dir=in action=allow protocol=TCP localport=8686 profile=any
if errorlevel 1 goto :fail

echo ==^> Registering hyper-relay Windows service...
sc stop hyper-relay 2>nul
sc delete hyper-relay 2>nul
sc create hyper-relay binPath= "\"%RELAY_BIN%\" service" start= auto DisplayName= "hyper-relay - HyperScope relay agent"
if errorlevel 1 goto :fail
sc description hyper-relay "HyperScope relay agent: wakes the local collector on demand (port 8686)." >nul 2>&1

echo ==^> Starting service...
sc start hyper-relay
if errorlevel 1 goto :fail

echo.
echo ==^> Done! hyper-relay runs as a Windows service (auto-start at boot).
echo    hyper-node is not resident: the relay spawns it on demand for each poll.
echo    Config: %DIR%  (key, mode, cert)
echo    Manual start:  sc start hyper-relay
echo    Manual stop:   sc stop hyper-relay
echo.
echo ==^> Add this node in the panel (all nodes run in relay mode):
echo    address = this machine's IP, key = hyper-node key show
echo    (collector commands still work: "%%BIN%%" collect / key setup / identity)
echo.
echo ==^> Trust devices (REQUIRED for remote commands): relay commands are
echo    signed; add each controlling device (panel/phone) to the trust list:
echo    "%%BIN%%" device add ^<device-id^> ^<device-pubkey^> admin
echo    (get the device pubkey from the panel/phone; see "%%BIN%%" device list)
goto :eof

:fail
echo.
echo FAILED. Run this script as Administrator.
exit /b 1
