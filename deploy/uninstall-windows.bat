@echo off
REM hyper-node Windows uninstaller (hyper-relay service + files)
REM ASCII only. Run in PowerShell/cmd as Administrator.

setlocal
set DIR=%ProgramData%\hyper-node

echo ==^> Stopping and removing hyper-relay service...
sc stop hyper-relay 2>nul
sc delete hyper-relay 2>nul

echo ==^> Removing old scheduled task (legacy)...
schtasks /End /TN "hyper-relay" 2>nul
schtasks /Delete /F /TN "hyper-relay" 2>nul

echo ==^> Removing old hyper-node scheduled tasks (legacy)...
schtasks /End /TN "hyper-node" 2>nul
schtasks /Delete /F /TN "hyper-node" 2>nul
schtasks /End /TN "hyper-node-test" 2>nul
schtasks /Delete /F /TN "hyper-node-test" 2>nul

echo ==^> Removing old hyper-node service (legacy)...
sc stop hyper-node 2>nul
sc delete hyper-node 2>nul

echo ==^> Removing old firewall rules (legacy)...
netsh advfirewall firewall delete rule name="hyper-node" 2>nul
netsh advfirewall firewall delete rule name="hyper-relay" 2>nul

echo ==^> Killing any running process...
taskkill /F /IM hyper-node.exe 2>nul
taskkill /F /IM hyper-relay.exe 2>nul

echo ==^> Removing %DIR% ...
rmdir /S /Q "%DIR%" 2>nul

echo.
echo ==^> Done! hyper-node + hyper-relay removed.
if exist "%DIR%" (
  echo NOTE: some files could not be removed - check %DIR%
)
