; SANKEY Copier Windows Installer Script
; Requires Inno Setup 6.2.2 or later
; https://jrsoftware.org/isinfo.php
;
; Version can be overridden via command line:
; ISCC /DMyAppVersion=1.2.3 setup.iss

#define MyAppName "SANKEY Copier"
#ifndef MyAppVersion
  #define MyAppVersion "1.0.0"
#endif
#define MyAppPublisher "SANKEY Copier Project"
#define MyAppURL "https://github.com/your-org/sankey-copier"
#define MyAppExeName "sankey-copier-server.exe"

[Setup]
; Basic application information
AppId={{12345678-1234-1234-1234-123456789012}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}

; Installation directories
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes

; Output
OutputDir=Output
OutputBaseFilename=SankeyCopierSetup-{#MyAppVersion}
SetupIconFile=resources\icon.ico
Compression=lzma2/ultra64
SolidCompression=yes

; Privileges and compatibility
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
MinVersion=10.0.17763
ArchitecturesInstallIn64BitMode=x64

; UI
WizardStyle=modern
DisableWelcomePage=no
LicenseFile=resources\license.txt

; Uninstall
UninstallDisplayIcon={app}\sankey-copier-tray.exe
UninstallDisplayName={#MyAppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Messages]
; English
english.WelcomeLabel2=This will install [name/ver] on your computer.%n%nVersion: {#MyAppVersion}%n%nIt is recommended that you close all other applications before continuing.

; Japanese
japanese.WelcomeLabel2=[name/ver] をコンピュータにインストールします。%n%nバージョン: {#MyAppVersion}%n%n続行する前に、他のすべてのアプリケーションを閉じることをお勧めします。

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "Start services automatically on Windows startup"; Flags: checkedonce
Name: "trayapp"; Description: "Launch tray application on Windows startup"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce

[Files]
; Rust Server
Source: "..\rust-server\target\release\sankey-copier-server.exe"; DestDir: "{app}"; Flags: ignoreversion

; System Tray Application
Source: "..\sankey-copier-tray\target\release\sankey-copier-tray.exe"; DestDir: "{app}"; Flags: ignoreversion

; NSSM for service management
Source: "resources\nssm.exe"; DestDir: "{app}"; Flags: ignoreversion

; Web UI (Next.js standalone build)
Source: "..\web-ui\.next\standalone\*"; DestDir: "{app}\web-ui"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\web-ui\.next\static\*"; DestDir: "{app}\web-ui\.next\static"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\web-ui\public\*"; DestDir: "{app}\web-ui\public"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

; Configuration files
Source: "..\rust-server\config.toml"; DestDir: "{app}"; Flags: ignoreversion onlyifdoesntexist

; MT4/MT5 Components
Source: "..\mql-zmq-dll\target\release\sankey_copier_zmq.dll"; DestDir: "{app}\mql\dll\x64"; Flags: ignoreversion
Source: "..\mql-zmq-dll\target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll"; DestDir: "{app}\mql\dll\x86"; Flags: ignoreversion

; MQL Compiled Files (for immediate use)
Source: "..\mql\MT4\Experts\*.ex4"; DestDir: "{app}\mql\MT4\Experts"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\mql\MT5\Experts\*.ex5"; DestDir: "{app}\mql\MT5\Experts"; Flags: ignoreversion skipifsourcedoesntexist

; Documentation
Source: "resources\license.txt"; DestDir: "{app}"; Flags: ignoreversion

[Dirs]
; Create data directories
Name: "{app}\data"; Permissions: users-full
Name: "{app}\data\logs"; Permissions: users-full

[Registry]
; Add tray application to Windows startup
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "SANKEY Copier Tray"; ValueData: """{app}\sankey-copier-tray.exe"""; Flags: uninsdeletevalue; Tasks: trayapp

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{code:GetWebUIUrl}"; IconFilename: "{app}\{#MyAppExeName}"
Name: "{group}\Open Web Interface"; Filename: "{code:GetWebUIUrl}"
Name: "{group}\Server Status"; Filename: "{sys}\sc.exe"; Parameters: "query SankeyCopierServer"
Name: "{group}\Stop Services"; Filename: "{app}\nssm.exe"; Parameters: "stop SankeyCopierServer"
Name: "{group}\Start Services"; Filename: "{app}\nssm.exe"; Parameters: "start SankeyCopierServer"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{code:GetWebUIUrl}"; IconFilename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; Install and start Windows services
; Note: Services will be configured to start automatically

; Install Rust Server service
Filename: "{app}\nssm.exe"; Parameters: "install SankeyCopierServer ""{app}\sankey-copier-server.exe"""; Flags: runhidden; StatusMsg: "Installing Rust server service..."
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer DisplayName ""SANKEY Copier Server"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer Description ""Backend server for SANKEY Copier MT4/MT5 trade copying system"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer AppDirectory ""{app}"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer AppStdout ""{app}\data\logs\server-stdout.log"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer AppStderr ""{app}\data\logs\server-stderr.log"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer Start SERVICE_AUTO_START"; Flags: runhidden; Tasks: autostart
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer Start SERVICE_DEMAND_START"; Flags: runhidden; Tasks: not autostart

; Install Web UI service (Node.js standalone)
Filename: "{app}\nssm.exe"; Parameters: "install SankeyCopierWebUI node"; Flags: runhidden; StatusMsg: "Installing Web UI service..."
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI Application node"; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI AppParameters \""{app}\web-ui\server.js\"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI DisplayName ""SANKEY Copier Web UI"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI Description ""Web interface for SANKEY Copier"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI AppDirectory ""{app}\web-ui"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI AppStdout ""{app}\data\logs\webui-stdout.log"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI AppStderr ""{app}\data\logs\webui-stderr.log"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI Start SERVICE_AUTO_START"; Flags: runhidden; Tasks: autostart
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI Start SERVICE_DEMAND_START"; Flags: runhidden; Tasks: not autostart

; Set WebUI to depend on Server
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI DependOnService SankeyCopierServer"; Flags: runhidden

; Environment variables for Web UI service will be set by CurStepChanged procedure

; Start services
Filename: "{app}\nssm.exe"; Parameters: "start SankeyCopierServer"; Flags: runhidden nowait; StatusMsg: "Starting services..."
Filename: "{app}\nssm.exe"; Parameters: "start SankeyCopierWebUI"; Flags: runhidden nowait

; Launch tray application
Filename: "{app}\sankey-copier-tray.exe"; Description: "Launch SANKEY Copier Tray Application"; Flags: nowait postinstall skipifsilent; Tasks: trayapp

; Open web interface
Filename: "{code:GetWebUIUrl}"; Description: "Open SANKEY Copier Web Interface"; Flags: shellexec postinstall skipifsilent

[UninstallRun]
; Stop tray application
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM sankey-copier-tray.exe"; Flags: runhidden; RunOnceId: "StopTray"

; Stop services before uninstalling
Filename: "{app}\nssm.exe"; Parameters: "stop SankeyCopierWebUI"; Flags: runhidden; RunOnceId: "StopWebUI"
Filename: "{app}\nssm.exe"; Parameters: "stop SankeyCopierServer"; Flags: runhidden; RunOnceId: "StopServer"
Filename: "{sys}\timeout.exe"; Parameters: "/t 3 /nobreak"; Flags: runhidden; RunOnceId: "WaitForStop"

; Remove services
Filename: "{app}\nssm.exe"; Parameters: "remove SankeyCopierWebUI confirm"; Flags: runhidden; RunOnceId: "RemoveWebUI"
Filename: "{app}\nssm.exe"; Parameters: "remove SankeyCopierServer confirm"; Flags: runhidden; RunOnceId: "RemoveServer"

[UninstallDelete]
; Clean up log files
Type: filesandordirs; Name: "{app}\data\logs"
; Optionally remove database (user will be prompted)
; Type: files; Name: "{app}\data\sankey_copier.db"

[Code]
var
  DataDirPage: TInputDirWizardPage;
  ServerPortPage: TInputQueryWizardPage;
  DataDirInitialized: Boolean;

function GetWebUIUrl(Param: String): String;
begin
  Result := 'http://localhost:' + ServerPortPage.Values[1];
end;

procedure InitializeWizard;
begin
  DataDirInitialized := False;

  { Create custom page for data directory }
  DataDirPage := CreateInputDirPage(wpSelectDir,
    'Select Data Directory', 'Where should application data be stored?',
    'Select the folder in which Setup should store database and log files, then click Next.',
    False, '');
  DataDirPage.Add('');
  { Default value will be set in CurPageChanged after app constant is initialized }

  { Create custom page for port configuration }
  ServerPortPage := CreateInputQueryPage(wpSelectDir,
    'Port Configuration', 'Configure network ports',
    'Please specify the port numbers for the server and web interface.');
  ServerPortPage.Add('Rust Server API Port:', False);
  ServerPortPage.Add('Web UI Port:', False);
  ServerPortPage.Values[0] := '3000';
  ServerPortPage.Values[1] := '8080';
end;

procedure CurPageChanged(CurPageID: Integer);
var
  ConfigFile: String;
  ConfigContent: TArrayOfString;
  I: Integer;
  Line: String;
  InServerSection: Boolean;
  InWebUISection: Boolean;
begin
  { Set default data directory after installation directory has been selected }
  if (CurPageID = DataDirPage.ID) and (not DataDirInitialized) then
  begin
    DataDirPage.Values[0] := ExpandConstant('{app}\data');
    DataDirInitialized := True;
  end;

  { Load existing port configuration for upgrades }
  if CurPageID = ServerPortPage.ID then
  begin
    ConfigFile := ExpandConstant('{app}\config.toml');
    if FileExists(ConfigFile) then
    begin
      LoadStringsFromFile(ConfigFile, ConfigContent);
      InServerSection := False;
      InWebUISection := False;

      for I := 0 to GetArrayLength(ConfigContent) - 1 do
      begin
        Line := Trim(ConfigContent[I]);

        { Track which section we're in }
        if Line = '[server]' then
        begin
          InServerSection := True;
          InWebUISection := False;
        end
        else if Line = '[webui]' then
        begin
          InServerSection := False;
          InWebUISection := True;
        end
        else if (Length(Line) > 0) and (Line[1] = '[') then
        begin
          InServerSection := False;
          InWebUISection := False;
        end;

        { Extract port values }
        if InServerSection and (Pos('port = ', Line) > 0) then
          ServerPortPage.Values[0] := Trim(Copy(Line, Pos('=', Line) + 1, Length(Line)));

        if InWebUISection and (Pos('port = ', Line) > 0) then
          ServerPortPage.Values[1] := Trim(Copy(Line, Pos('=', Line) + 1, Length(Line)));
      end;
    end;
  end;
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  { Skip custom pages in silent mode }
  Result := False;
  if (PageID = DataDirPage.ID) or (PageID = ServerPortPage.ID) then
    Result := WizardSilent();
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ConfigFile: String;
  ConfigContent: TArrayOfString;
  I: Integer;
  Line: String;
  InServerSection: Boolean;
  InWebUISection: Boolean;
  InCorsSection: Boolean;
  ServerPortUpdated: Boolean;
  WebUIPortUpdated: Boolean;
  WebUIUrlUpdated: Boolean;
  CorsOriginsUpdated: Boolean;
  WebUIPort: String;
  ServerPort: String;
  ResultCode: Integer;
  NssmPath: String;
begin
  if CurStep = ssPostInstall then
  begin
    { Update config.toml with custom settings }
    ConfigFile := ExpandConstant('{app}\config.toml');
    WebUIPort := ServerPortPage.Values[1];
    ServerPort := ServerPortPage.Values[0];
    NssmPath := ExpandConstant('{app}\nssm.exe');

    if FileExists(ConfigFile) then
    begin
      LoadStringsFromFile(ConfigFile, ConfigContent);
      InServerSection := False;
      InWebUISection := False;
      InCorsSection := False;
      ServerPortUpdated := False;
      WebUIPortUpdated := False;
      WebUIUrlUpdated := False;
      CorsOriginsUpdated := False;

      for I := 0 to GetArrayLength(ConfigContent) - 1 do
      begin
        Line := Trim(ConfigContent[I]);

        { Track which section we're in }
        if Line = '[server]' then
        begin
          InServerSection := True;
          InWebUISection := False;
          InCorsSection := False;
        end
        else if Line = '[webui]' then
        begin
          InServerSection := False;
          InWebUISection := True;
          InCorsSection := False;
        end
        else if Line = '[cors]' then
        begin
          InServerSection := False;
          InWebUISection := False;
          InCorsSection := True;
        end
        else if (Length(Line) > 0) and (Line[1] = '[') then
        begin
          InServerSection := False;
          InWebUISection := False;
          InCorsSection := False;
        end;

        { Update port values }
        if InServerSection and (Pos('port = ', ConfigContent[I]) > 0) then
        begin
          ConfigContent[I] := 'port = ' + ServerPort;
          ServerPortUpdated := True;
        end;

        if InWebUISection and (Pos('port = ', ConfigContent[I]) > 0) then
        begin
          ConfigContent[I] := 'port = ' + WebUIPort;
          WebUIPortUpdated := True;
        end;

        if InWebUISection and (Pos('url = ', ConfigContent[I]) > 0) then
        begin
          ConfigContent[I] := 'url = "http://localhost:' + WebUIPort + '"';
          WebUIUrlUpdated := True;
        end;

        if InCorsSection and (Pos('allowed_origins = ', ConfigContent[I]) > 0) then
        begin
          ConfigContent[I] := 'allowed_origins = ["http://localhost:' + WebUIPort + '"]';
          CorsOriginsUpdated := True;
        end;
      end;

      SaveStringsToFile(ConfigFile, ConfigContent, False);
    end;

    { Set NSSM environment variables for Web UI service }
    { Set both PORT and NEXT_PUBLIC_API_URL in a single call }
    Exec(NssmPath, 'set SankeyCopierWebUI AppEnvironmentExtra PORT=' + WebUIPort + #13#10 + 'NEXT_PUBLIC_API_URL=http://localhost:' + ServerPort, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;

function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;

  { Ask user if they want to keep data }
  if MsgBox('Do you want to remove the database and log files?', mbConfirmation, MB_YESNO) = IDYES then
  begin
    { User wants to remove data - will be handled by [UninstallDelete] section }
    Result := True;
  end;
end;
