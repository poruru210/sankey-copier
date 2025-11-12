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

[CustomMessages]
; English
english.DataDirPageTitle=Select Data Directory
english.DataDirPageDescription=Where should application data be stored?
english.DataDirPageSubDescription=Select the folder in which Setup should store database and log files, then click Next.
english.PortConfigPageTitle=Port Configuration
english.PortConfigPageDescription=Configure network ports
english.PortConfigPageSubDescription=Please specify the port numbers for the server and web interface.
english.ServerPortLabel=Rust Server API Port:
english.WebUIPortLabel=Web UI Port:
english.TaskAutostart=Start services automatically on Windows startup
english.OpenWebInterface=Open SANKEY Copier Web Interface
english.InstallingServerService=Installing Rust server service...
english.StartingServices=Starting services...

; Japanese
japanese.DataDirPageTitle=データディレクトリの選択
japanese.DataDirPageDescription=アプリケーションデータをどこに保存しますか？
japanese.DataDirPageSubDescription=データベースとログファイルを保存するフォルダを選択し、「次へ」をクリックしてください。
japanese.PortConfigPageTitle=ポート設定
japanese.PortConfigPageDescription=ネットワークポートの設定
japanese.PortConfigPageSubDescription=サーバーとWebインターフェースのポート番号を指定してください。
japanese.ServerPortLabel=Rust Server APIポート:
japanese.WebUIPortLabel=Web UIポート:
japanese.TaskAutostart=Windows起動時にサービスを自動起動する
japanese.OpenWebInterface=SANKEY Copier Webインターフェースを開く
japanese.InstallingServerService=Rustサーバーサービスをインストールしています...
japanese.StartingServices=サービスを起動しています...

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "{cm:TaskAutostart}"; Flags: checkedonce

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
; MT5 uses 64-bit DLL in Libraries folder
Source: "..\mql-zmq-dll\target\release\sankey_copier_zmq.dll"; DestDir: "{app}\mql\MT5\Libraries"; Flags: ignoreversion
; MT4 uses 32-bit DLL in Libraries folder
Source: "..\mql-zmq-dll\target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll"; DestDir: "{app}\mql\MT4\Libraries"; Flags: ignoreversion

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
; Add tray application to Windows startup (always enabled)
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "SANKEY Copier Tray"; ValueData: """{app}\sankey-copier-tray.exe"""; Flags: uninsdeletevalue

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
Filename: "{app}\nssm.exe"; Parameters: "install SankeyCopierServer ""{app}\sankey-copier-server.exe"""; Flags: runhidden; StatusMsg: "{cm:InstallingServerService}"
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer DisplayName ""SANKEY Copier Server"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer Description ""Backend server for SANKEY Copier MT4/MT5 trade copying system"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer AppDirectory ""{app}"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer AppStdout ""{app}\data\logs\server-stdout.log"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer AppStderr ""{app}\data\logs\server-stderr.log"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer Start SERVICE_AUTO_START"; Flags: runhidden; Tasks: autostart
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierServer Start SERVICE_DEMAND_START"; Flags: runhidden; Tasks: not autostart

; Install Web UI service (Node.js standalone)
Filename: "{app}\nssm.exe"; Parameters: "install SankeyCopierWebUI node"; Flags: runhidden; StatusMsg: "{cm:InstallingServerService}"
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI Application node"; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI AppParameters \""{app}\web-ui\server.js\"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI DisplayName ""SANKEY Copier Web UI"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI Description ""Web interface for SANKEY Copier"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI AppDirectory ""{app}\web-ui"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI AppStdout ""{app}\data\logs\webui-stdout.log"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI AppStderr ""{app}\data\logs\webui-stderr.log"""; Flags: runhidden
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI Start SERVICE_AUTO_START"; Flags: runhidden; Tasks: autostart
Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI Start SERVICE_DEMAND_START"; Flags: runhidden; Tasks: not autostart

; Set WebUI to depend on Server (disabled - services are now independent)
; Filename: "{app}\nssm.exe"; Parameters: "set SankeyCopierWebUI DependOnService SankeyCopierServer"; Flags: runhidden

; Environment variables for Web UI service will be set by CurStepChanged procedure

; Start services
Filename: "{app}\nssm.exe"; Parameters: "start SankeyCopierServer"; Flags: runhidden nowait; StatusMsg: "{cm:StartingServices}"
Filename: "{app}\nssm.exe"; Parameters: "start SankeyCopierWebUI"; Flags: runhidden nowait

; Launch tray application (always)
Filename: "{app}\sankey-copier-tray.exe"; Flags: nowait skipifsilent

; Open web interface
Filename: "{code:GetWebUIUrl}"; Description: "{cm:OpenWebInterface}"; Flags: shellexec postinstall skipifsilent

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
; Clean up all data files
Type: filesandordirs; Name: "{app}\data"
Type: files; Name: "{app}\sankey_copier.db"
Type: files; Name: "{app}\config.toml"

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
    CustomMessage('DataDirPageTitle'),
    CustomMessage('DataDirPageDescription'),
    CustomMessage('DataDirPageSubDescription'),
    False, '');
  DataDirPage.Add('');
  { Default value will be set in CurPageChanged after app constant is initialized }

  { Create custom page for port configuration }
  ServerPortPage := CreateInputQueryPage(wpSelectDir,
    CustomMessage('PortConfigPageTitle'),
    CustomMessage('PortConfigPageDescription'),
    CustomMessage('PortConfigPageSubDescription'));
  ServerPortPage.Add(CustomMessage('ServerPortLabel'), False);
  ServerPortPage.Add(CustomMessage('WebUIPortLabel'), False);
  ServerPortPage.Values[0] := '3000';
  ServerPortPage.Values[1] := '8080';
end;

procedure CurPageChanged(CurPageID: Integer);
var
  ConfigFile: String;
  ConfigContent: TArrayOfString;
  I: Integer;
  Line: String;
  Value: String;
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
        if InServerSection and (Pos('port', Line) > 0) and (Pos('=', Line) > 0) then
        begin
          { Extract value after '=' and before any comment }
          Value := Copy(Line, Pos('=', Line) + 1, Length(Line));
          { Remove comments }
          if Pos('#', Value) > 0 then
            Value := Copy(Value, 1, Pos('#', Value) - 1);
          { Trim whitespace and extract port number }
          Value := Trim(Value);
          if Value <> '' then
            ServerPortPage.Values[0] := Value;
        end;

        if InWebUISection and (Pos('port', Line) > 0) and (Pos('=', Line) > 0) then
        begin
          { Extract value after '=' and before any comment }
          Value := Copy(Line, Pos('=', Line) + 1, Length(Line));
          { Remove comments }
          if Pos('#', Value) > 0 then
            Value := Copy(Value, 1, Pos('#', Value) - 1);
          { Trim whitespace and extract port number }
          Value := Trim(Value);
          if Value <> '' then
            ServerPortPage.Values[1] := Value;
        end;
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
          ConfigContent[I] := 'url = "http://127.0.0.1:' + WebUIPort + '"';
          WebUIUrlUpdated := True;
        end;

        if InCorsSection and (Pos('allowed_origins = ', ConfigContent[I]) > 0) then
        begin
          ConfigContent[I] := 'allowed_origins = ["http://127.0.0.1:' + WebUIPort + '"]';
          CorsOriginsUpdated := True;
        end;
      end;

      SaveStringsToFile(ConfigFile, ConfigContent, False);
    end;

    { Set NSSM environment variables for Web UI service }
    { Set PORT environment variable for Next.js }
    Exec(NssmPath, 'set SankeyCopierWebUI AppEnvironmentExtra PORT=' + WebUIPort, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;

