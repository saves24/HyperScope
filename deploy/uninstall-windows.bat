@echo off
REM hyper-node Windows uninstaller (service + firewall + files)
REM ASCII only. Run in PowerShell/cmd as Administrator.

setlocal
set BIN=%ProgramData%\hyper-node\hyper-node.exe
set DIR=%ProgramData%\hyper-node

echo ==^> Stopping and removing service (hyper-node)...
sc stop hyper-node 2>nul
sc delete hyper-node 2>nul

echo ==^> Removing scheduled task (legacy)...
schtasks /End /TN "hyper-node" 2>nul
schtasks /Delete /F /TN "hyper-node" 2>nul
schtasks /End /TN "hyper-node-test" 2>nul
schtasks /Delete /F /TN "hyper-node-test" 2>nul

echo ==^> Removing firewall rule (port 5000)...
netsh advfirewall firewall delete rule name="hyper-node" 2>nul

echo ==^> Killing any running process...
taskkill /F /IM hyper-node.exe 2>nul

echo ==^> Removing %DIR% ...
rmdir /S /Q "%DIR%" 2>nul

echo.
echo ==^> Done! hyper-node removed.
if exist "%DIR%" (
  echo NOTE: some files could not be removed - check %DIR%
)
