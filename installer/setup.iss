; SANKEY Copier Unified Installer
; Installs rust-server (Windows Service) + Desktop App + MT4/MT5 Components

#define MyAppName "SANKEY Copier"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "SANKEY Copier Team"
#define MyAppURL "https://github.com/poruru210/sankey-copier"
#define MyAppExeName "sankey-copier-desktop.exe"
#define MyServerExeName "rust-server.exe"

[Setup]
; NOTE: The value of AppId uniquely identifies this application. Do not use the same AppId value in installers for other applications.
AppId={{8F9B3C2E-5D7A-4B1C-9E2F-6A8D3C5B7E9F}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\LICENSE
OutputDir=output
OutputBaseFilename=SankeyCopierSetup-{#MyAppVersion}
SetupIconFile=..\app.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startservice"; Description: "Start rust-server service after installation"; GroupDescription: "Service Options:"; Flags: checkedonce

[Files]
; Desktop App (Tauri - includes web-ui embedded as static files)
; Built with: npm run tauri build -- --bundles none
Source: "..\desktop-app\src-tauri\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

; rust-server
Source: "..\rust-server\target\release\{#MyServerExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\rust-server\config.toml"; DestDir: "{app}"; Flags: ignoreversion onlyifdoesntexist

; MT4/MT5 Components (if built)
Source: "..\mql\build\mt4\Experts\*.ex4"; DestDir: "{app}\mql\mt4\Experts"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\mql\build\mt4\Libraries\*.dll"; DestDir: "{app}\mql\mt4\Libraries"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\mql\build\mt5\Experts\*.ex5"; DestDir: "{app}\mql\mt5\Experts"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\mql\build\mt5\Libraries\*.dll"; DestDir: "{app}\mql\mt5\Libraries"; Flags: ignoreversion skipifsourcedoesntexist

; Icon
Source: "..\app.ico"; DestDir: "{app}"; Flags: ignoreversion

; NOTE: Don't use "Flags: ignoreversion" on any shared system files

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"; Tasks: desktopicon

[Run]
; Install rust-server as Windows Service
Filename: "sc.exe"; Parameters: "create SankeyCopierServer binPath= ""{app}\{#MyServerExeName}"" DisplayName= ""SANKEY Copier Server"" start= auto"; Flags: runhidden; StatusMsg: "Registering Windows Service..."
Filename: "sc.exe"; Parameters: "description SankeyCopierServer ""Trade copier server for MT4/MT5. Runs 24/7 in background."""; Flags: runhidden
Filename: "sc.exe"; Parameters: "start SankeyCopierServer"; Flags: runhidden; Tasks: startservice; StatusMsg: "Starting rust-server service..."

; Optionally launch Desktop App
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Stop and remove service before uninstall
Filename: "sc.exe"; Parameters: "stop SankeyCopierServer"; Flags: runhidden
Filename: "sc.exe"; Parameters: "delete SankeyCopierServer"; Flags: runhidden

[Code]
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;

  // Check if service already exists and stop it
  if Exec('sc.exe', 'query SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    if ResultCode = 0 then
    begin
      // Service exists, stop it
      Exec('sc.exe', 'stop SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
      Sleep(2000); // Wait for service to stop
    end;
  end;
end;

function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;

  // Stop service before uninstall
  if Exec('sc.exe', 'query SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    if ResultCode = 0 then
    begin
      Exec('sc.exe', 'stop SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
      Sleep(2000);
    end;
  end;
end;
