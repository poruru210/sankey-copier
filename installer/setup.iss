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
english.PortConfigPageTitle=Port Configuration
english.PortConfigPageDescription=Configure network ports
english.PortConfigPageSubDescription=Please specify the port numbers for the server and web interface.
english.ServerPortLabel=Rust Server API Port:
english.WebUIPortLabel=Web UI Port:
english.OpenWebInterface=Open SANKEY Copier Web Interface
english.InstallingServerService=Installing Rust server service...
english.StartingServices=Starting services...
english.RepairInstallationTitle=Repair Installation
english.RepairInstallationMessage=The same version of SANKEY Copier is already installed.%n%nDo you want to repair the installation?
english.UpdateInstallationTitle=Update Installation
english.UpdateInstallationMessage=A previous version of SANKEY Copier is already installed.%n%nDo you want to update to version {#MyAppVersion}?
english.StoppingServices=Stopping existing services...
english.MergingConfig=Merging configuration file...

; Japanese
japanese.PortConfigPageTitle=ポート設定
japanese.PortConfigPageDescription=ネットワークポートの設定
japanese.PortConfigPageSubDescription=サーバーとWebインターフェースのポート番号を指定してください。
japanese.ServerPortLabel=Rust Server APIポート:
japanese.WebUIPortLabel=Web UIポート:
japanese.OpenWebInterface=SANKEY Copier Webインターフェースを開く
japanese.InstallingServerService=Rustサーバーサービスをインストールしています...
japanese.StartingServices=サービスを起動しています...
japanese.RepairInstallationTitle=インストールの修復
japanese.RepairInstallationMessage=同じバージョンのSANKEY Copierが既にインストールされています。%n%nインストールを修復しますか？
japanese.UpdateInstallationTitle=インストールの更新
japanese.UpdateInstallationMessage=以前のバージョンのSANKEY Copierが既にインストールされています。%n%nバージョン{#MyAppVersion}に更新しますか？
japanese.StoppingServices=既存のサービスを停止しています...
japanese.MergingConfig=設定ファイルをマージしています...

[Tasks]
; Removed desktop icon option

[Files]
; Rust Server
Source: "..\rust-server\target\release\sankey-copier-server.exe"; DestDir: "{app}"; Flags: ignoreversion

; System Tray Application
Source: "..\sankey-copier-tray\target\release\sankey-copier-tray.exe"; DestDir: "{app}"; Flags: ignoreversion

; Desktop Application (Tauri)
Source: "..\desktop\src-tauri\target\release\sankey-copier-desktop.exe"; DestDir: "{app}"; Flags: ignoreversion

; NSSM for service management
Source: "resources\nssm.exe"; DestDir: "{app}"; Flags: ignoreversion

; Web UI (Next.js standalone build) for Desktop App
Source: "..\web-ui\.next\standalone\*"; DestDir: "{app}\web-ui"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\web-ui\.next\static\*"; DestDir: "{app}\web-ui\.next\static"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\web-ui\public\*"; DestDir: "{app}\web-ui\public"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

; Configuration files (will be merged with existing config in code)
Source: "..\rust-server\config.toml"; DestDir: "{app}"; DestName: "config.toml.new"; Flags: ignoreversion

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
; Create directories
Name: "{app}\data"; Permissions: users-full
Name: "{app}\logs"; Permissions: users-full

[Registry]
; Add tray application to Windows startup (always enabled)
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "SANKEY Copier Tray"; ValueData: """{app}\sankey-copier-tray.exe"""; Flags: uninsdeletevalue

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\sankey-copier-desktop.exe"; IconFilename: "{app}\sankey-copier-desktop.exe"
Name: "{group}\Open Desktop App"; Filename: "{app}\sankey-copier-desktop.exe"
Name: "{group}\Server Status"; Filename: "{sys}\sc.exe"; Parameters: "query SankeyCopierServer"
Name: "{group}\Stop Server"; Filename: "{app}\nssm.exe"; Parameters: "stop SankeyCopierServer"
Name: "{group}\Start Server"; Filename: "{app}\nssm.exe"; Parameters: "start SankeyCopierServer"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

[Run]
; Services are installed and started by CurStepChanged procedure

; Launch tray application (always)
Filename: "{app}\sankey-copier-tray.exe"; Flags: nowait skipifsilent

; Launch desktop application
Filename: "{app}\sankey-copier-desktop.exe"; Description: "{cm:OpenWebInterface}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Stop desktop application
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM sankey-copier-desktop.exe"; Flags: runhidden; RunOnceId: "StopDesktop"

; Stop tray application
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM sankey-copier-tray.exe"; Flags: runhidden; RunOnceId: "StopTray"

; Stop server service before uninstalling
Filename: "{app}\nssm.exe"; Parameters: "stop SankeyCopierServer"; Flags: runhidden; RunOnceId: "StopServer"
Filename: "{sys}\timeout.exe"; Parameters: "/t 3 /nobreak"; Flags: runhidden; RunOnceId: "WaitForStop"

; Remove server service
Filename: "{app}\nssm.exe"; Parameters: "remove SankeyCopierServer confirm"; Flags: runhidden; RunOnceId: "RemoveServer"

[UninstallDelete]
; Clean up all remaining files and directories
Type: filesandordirs; Name: "{app}\*"

[Code]
var
  ServerPortPage: TInputQueryWizardPage;
  IsRepairMode: Boolean;
  IsUpdateMode: Boolean;

function GetWebUIUrl(Param: String): String;
begin
  Result := 'http://localhost:' + ServerPortPage.Values[1];
end;

procedure MergeConfigFiles(ExistingConfigPath: String; NewConfigPath: String);
var
  ExistingContent: TArrayOfString;
  NewContent: TArrayOfString;
  MergedContent: TArrayOfString;
  I, J: Integer;
  CurrentSection: String;
  ExistingSection: String;
  Line: String;
  Key: String;
  Value: String;
  FullKey: String;
  ExistingKey: String;
  Found: Boolean;
begin
  { Load both config files }
  LoadStringsFromFile(ExistingConfigPath, ExistingContent);
  LoadStringsFromFile(NewConfigPath, NewContent);

  { Build merged content from new config template }
  SetArrayLength(MergedContent, 0);
  CurrentSection := '';

  for I := 0 to GetArrayLength(NewContent) - 1 do
  begin
    Line := NewContent[I];

    { Track current section }
    if (Length(Trim(Line)) > 0) and (Trim(Line)[1] = '[') then
    begin
      CurrentSection := Trim(Line);
      { Add section header }
      SetArrayLength(MergedContent, GetArrayLength(MergedContent) + 1);
      MergedContent[GetArrayLength(MergedContent) - 1] := Line;
    end
    else if (Length(Trim(Line)) > 0) and (Pos('=', Trim(Line)) > 0) and (Pos('#', Trim(Line)) <> 1) then
    begin
      { This is a key=value line }
      Key := Trim(Copy(Trim(Line), 1, Pos('=', Trim(Line)) - 1));

      { Search for this key in existing config within the same section }
      Found := False;
      ExistingSection := '';

      for J := 0 to GetArrayLength(ExistingContent) - 1 do
      begin
        Line := Trim(ExistingContent[J]);

        { Track section in existing config }
        if (Length(Line) > 0) and (Line[1] = '[') then
        begin
          ExistingSection := Line;
        end
        else if (Length(Line) > 0) and (Pos('=', Line) > 0) and (Pos('#', Line) <> 1) then
        begin
          { Check if this is the same key in the same section }
          ExistingKey := Trim(Copy(Line, 1, Pos('=', Line) - 1));

          if (ExistingSection = CurrentSection) and (ExistingKey = Key) then
          begin
            { Use existing value from old config }
            SetArrayLength(MergedContent, GetArrayLength(MergedContent) + 1);
            MergedContent[GetArrayLength(MergedContent) - 1] := ExistingContent[J];
            Found := True;
            Break;
          end;
        end;
      end;

      if not Found then
      begin
        { New key - use default from new config }
        SetArrayLength(MergedContent, GetArrayLength(MergedContent) + 1);
        MergedContent[GetArrayLength(MergedContent) - 1] := NewContent[I];
      end;
    end
    else
    begin
      { Comment or empty line - keep from new config }
      SetArrayLength(MergedContent, GetArrayLength(MergedContent) + 1);
      MergedContent[GetArrayLength(MergedContent) - 1] := Line;
    end;
  end;

  { Save merged content }
  SaveStringsToFile(ExistingConfigPath, MergedContent, False);

  { Delete temporary new config file }
  DeleteFile(NewConfigPath);
end;

function InitializeSetup(): Boolean;
var
  ExistingPath: String;
  InstalledVersion: String;
  UninstallKey: String;
begin
  Result := True;
  IsRepairMode := False;
  IsUpdateMode := False;

  { Check if SANKEY Copier is already installed }
  ExistingPath := ExpandConstant('{autopf}\{#MyAppName}\sankey-copier-server.exe');

  if FileExists(ExistingPath) then
  begin
    { Get installed version from registry }
    UninstallKey := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#SetupSetting("AppId")}_is1';

    if RegQueryStringValue(HKLM, UninstallKey, 'DisplayVersion', InstalledVersion) or
       RegQueryStringValue(HKCU, UninstallKey, 'DisplayVersion', InstalledVersion) then
    begin
      { Compare versions }
      if InstalledVersion = '{#MyAppVersion}' then
      begin
        { Same version - Repair mode }
        IsRepairMode := True;
        if MsgBox(CustomMessage('RepairInstallationMessage'),
                  mbConfirmation, MB_YESNO or MB_DEFBUTTON1) = IDNO then
        begin
          Result := False;
        end;
      end
      else
      begin
        { Different version - Update mode }
        IsUpdateMode := True;
        if MsgBox(CustomMessage('UpdateInstallationMessage'),
                  mbConfirmation, MB_YESNO or MB_DEFBUTTON1) = IDNO then
        begin
          Result := False;
        end;
      end;
    end
    else
    begin
      { Version not found in registry, assume update mode }
      IsUpdateMode := True;
      if MsgBox(CustomMessage('UpdateInstallationMessage'),
                mbConfirmation, MB_YESNO or MB_DEFBUTTON1) = IDNO then
      begin
        Result := False;
      end;
    end;
  end;
end;

procedure InitializeWizard;
begin
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
  if PageID = ServerPortPage.ID then
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
  if CurStep = ssInstall then
  begin
    { Stop existing applications and services before installing }
    if IsRepairMode or IsUpdateMode then
    begin
      NssmPath := ExpandConstant('{autopf}\{#MyAppName}\nssm.exe');

      { Stop desktop application }
      Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM sankey-copier-desktop.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

      { Stop tray application }
      Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM sankey-copier-tray.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

      { Stop and remove services if nssm.exe exists }
      if FileExists(NssmPath) then
      begin
        { Stop web-ui service (legacy, may not exist in newer installs) }
        Exec(NssmPath, 'stop SankeyCopierWebUI', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
        Exec(NssmPath, 'stop SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
        Sleep(2000); { Wait for services to stop }

        { Remove existing services }
        Exec(NssmPath, 'remove SankeyCopierWebUI confirm', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
        Exec(NssmPath, 'remove SankeyCopierServer confirm', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
      end;
    end;
  end;

  if CurStep = ssPostInstall then
  begin
    { Merge config.toml }
    ConfigFile := ExpandConstant('{app}\config.toml');
    WebUIPort := ServerPortPage.Values[1];
    ServerPort := ServerPortPage.Values[0];
    NssmPath := ExpandConstant('{app}\nssm.exe');

    { Merge configuration files }
    if FileExists(ConfigFile) then
    begin
      { Existing config.toml found - merge with new version }
      MergeConfigFiles(ConfigFile, ExpandConstant('{app}\config.toml.new'));
    end
    else
    begin
      { No existing config - rename new config }
      RenameFile(ExpandConstant('{app}\config.toml.new'), ConfigFile);
    end;

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

    { Install Windows services }
    NssmPath := ExpandConstant('{app}\nssm.exe');

    { Server service - always create }
    Exec(NssmPath, 'install SankeyCopierServer "' + ExpandConstant('{app}\sankey-copier-server.exe') + '"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

    { Configure Server service }
    Exec(NssmPath, 'set SankeyCopierServer DisplayName "SANKEY Copier Server"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec(NssmPath, 'set SankeyCopierServer Description "Backend server for SANKEY Copier MT4/MT5 trade copying system"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec(NssmPath, 'set SankeyCopierServer AppDirectory "' + ExpandConstant('{app}') + '"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    { Server uses config.toml [logging] settings - no NSSM log redirection needed }
    Exec(NssmPath, 'set SankeyCopierServer Start SERVICE_AUTO_START', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

    { Start server service }
    Exec(NssmPath, 'start SankeyCopierServer', '', SW_HIDE, ewNoWait, ResultCode);
  end;
end;

