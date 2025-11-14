; SANKEY Copier Unified Installer
; Installs rust-server (Windows Service) + Desktop App + Tray App + MT4/MT5 Components

#define MyAppName "SANKEY Copier"
#ifndef MyAppVersion
  #define MyAppVersion "1.0.0"
#endif
#ifndef MyFileVersion
  #define MyFileVersion "1.0.0.0"
#endif
#ifndef MyBuildInfo
  #define MyBuildInfo "1.0.0+build.0.unknown"
#endif
#define MyAppPublisher "SANKEY Copier Team"
#define MyAppURL "https://github.com/poruru210/sankey-copier"
#define MyAppCopyright "Copyright (C) 2025 SANKEY Copier Team"
#define MyAppExeName "sankey-copier-desktop.exe"
#define MyServerExeName "sankey-copier-server.exe"
#define MyTrayExeName "sankey-copier-tray.exe"

[Setup]
; NOTE: The value of AppId uniquely identifies this application. Do not use the same AppId value in installers for other applications.
AppId={{8F9B3C2E-5D7A-4B1C-9E2F-6A8D3C5B7E9F}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
AppCopyright={#MyAppCopyright}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\LICENSE
InfoBeforeFile=..\installer\README.md
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
; Version information
VersionInfoVersion={#MyFileVersion}
VersionInfoDescription={#MyAppName} Setup
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoCopyright={#MyAppCopyright}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Messages]
; English
english.WelcomeLabel2=This will install [name/ver] on your computer.%n%nBuild: {#MyBuildInfo}%n%nIt is recommended that you close all other applications before continuing.

; Japanese
japanese.WelcomeLabel2=[name/ver] をコンピュータにインストールします。%n%nビルド: {#MyBuildInfo}%n%n続行する前に、他のすべてのアプリケーションを閉じることをお勧めします。

[CustomMessages]
; English
english.PortConfigPageTitle=Port Configuration
english.PortConfigPageDescription=Configure network port
english.PortConfigPageSubDescription=Please specify the port number for the server API.
english.ServerPortLabel=Rust Server API Port:
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
japanese.PortConfigPageSubDescription=サーバーAPIのポート番号を指定してください。
japanese.ServerPortLabel=Rust Server APIポート:
japanese.InstallingServerService=Rustサーバーサービスをインストールしています...
japanese.StartingServices=サービスを起動しています...
japanese.RepairInstallationTitle=インストールの修復
japanese.RepairInstallationMessage=同じバージョンのSANKEY Copierが既にインストールされています。%n%nインストールを修復しますか？
japanese.UpdateInstallationTitle=インストールの更新
japanese.UpdateInstallationMessage=以前のバージョンのSANKEY Copierが既にインストールされています。%n%nバージョン{#MyAppVersion}に更新しますか？
japanese.StoppingServices=既存のサービスを停止しています...
japanese.MergingConfig=設定ファイルをマージしています...

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
; Desktop App (Tauri - includes web-ui embedded as static files)
Source: "..\desktop-app\src-tauri\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

; rust-server
Source: "..\rust-server\target\release\{#MyServerExeName}"; DestDir: "{app}"; Flags: ignoreversion
; Configuration file (will be merged with existing config in code)
Source: "..\rust-server\config.toml"; DestDir: "{app}"; DestName: "config.toml.new"; Flags: ignoreversion

; Tray App (System tray for service management)
Source: "..\sankey-copier-tray\target\release\{#MyTrayExeName}"; DestDir: "{app}"; Flags: ignoreversion

; NSSM (Windows Service Manager for Tray App)
Source: "resources\nssm.exe"; DestDir: "{app}"; Flags: ignoreversion

; MT4/MT5 Components (if built)
Source: "..\mql\build\mt4\Experts\*.ex4"; DestDir: "{app}\mql\mt4\Experts"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\mql\build\mt4\Libraries\*.dll"; DestDir: "{app}\mql\mt4\Libraries"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\mql\build\mt5\Experts\*.ex5"; DestDir: "{app}\mql\mt5\Experts"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\mql\build\mt5\Libraries\*.dll"; DestDir: "{app}\mql\mt5\Libraries"; Flags: ignoreversion skipifsourcedoesntexist

; Icon
Source: "..\app.ico"; DestDir: "{app}"; Flags: ignoreversion

; NOTE: Don't use "Flags: ignoreversion" on any shared system files

[Dirs]
; Create directories with proper permissions
Name: "{app}\data"; Permissions: users-full
Name: "{app}\logs"; Permissions: users-full

[Registry]
; Save detailed build information for troubleshooting and version tracking
Root: HKLM; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#SetupSetting('AppId')}_is1"; ValueType: string; ValueName: "BuildInfo"; ValueData: "{#MyBuildInfo}"; Flags: uninsdeletevalue
Root: HKLM; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#SetupSetting('AppId')}_is1"; ValueType: string; ValueName: "FileVersion"; ValueData: "{#MyFileVersion}"; Flags: uninsdeletevalue

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"; Tasks: desktopicon
Name: "{userstartup}\{#MyAppName} Tray"; Filename: "{app}\{#MyTrayExeName}"

[Run]
; Services are installed and configured by CurStepChanged procedure

; Launch tray application (always)
Filename: "{app}\{#MyTrayExeName}"; Flags: nowait skipifsilent

; Optionally launch Desktop App
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Stop tray application
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#MyTrayExeName}"; Flags: runhidden; RunOnceId: "StopTray"

; Stop and remove service before uninstall (using NSSM)
Filename: "{app}\nssm.exe"; Parameters: "stop SankeyCopierServer"; Flags: runhidden; RunOnceId: "StopServer"
Filename: "{sys}\timeout.exe"; Parameters: "/t 3 /nobreak"; Flags: runhidden; RunOnceId: "WaitForStop"
Filename: "{app}\nssm.exe"; Parameters: "remove SankeyCopierServer confirm"; Flags: runhidden; RunOnceId: "RemoveServer"

[UninstallDelete]
; Clean up all remaining files and directories
Type: filesandordirs; Name: "{app}\*"

[Code]
var
  ServerPortPage: TInputQueryWizardPage;
  IsRepairMode: Boolean;
  IsUpdateMode: Boolean;

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
  ResultCode: Integer;
  NssmPath: String;
begin
  Result := True;
  IsRepairMode := False;
  IsUpdateMode := False;

  { Check if SANKEY Copier is already installed }
  ExistingPath := ExpandConstant('{autopf}\{#MyAppName}\{#MyServerExeName}');

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
          Exit;
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
          Exit;
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
        Exit;
      end;
    end;

    { Stop existing services and tray app }
    NssmPath := ExpandConstant('{autopf}\{#MyAppName}\nssm.exe');

    { Stop tray application }
    Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#MyTrayExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

    { Stop service using nssm if available, otherwise sc.exe }
    if FileExists(NssmPath) then
    begin
      Exec(NssmPath, 'stop SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
      Sleep(2000);
      Exec(NssmPath, 'remove SankeyCopierServer confirm', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    end
    else
    begin
      { Fallback to sc.exe }
      if Exec('sc.exe', 'query SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
      begin
        if ResultCode = 0 then
        begin
          Exec('sc.exe', 'stop SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
          Sleep(2000);
          Exec('sc.exe', 'delete SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
        end;
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
  ServerPortPage.Values[0] := '3000';
end;

procedure CurPageChanged(CurPageID: Integer);
var
  ConfigFile: String;
  ConfigContent: TArrayOfString;
  I: Integer;
  Line: String;
  Value: String;
  InServerSection: Boolean;
begin
  { Load existing port configuration for upgrades }
  if CurPageID = ServerPortPage.ID then
  begin
    ConfigFile := ExpandConstant('{app}\config.toml');
    if FileExists(ConfigFile) then
    begin
      LoadStringsFromFile(ConfigFile, ConfigContent);
      InServerSection := False;

      for I := 0 to GetArrayLength(ConfigContent) - 1 do
      begin
        Line := Trim(ConfigContent[I]);

        { Track which section we're in }
        if Line = '[server]' then
        begin
          InServerSection := True;
        end
        else if (Length(Line) > 0) and (Line[1] = '[') then
        begin
          InServerSection := False;
        end;

        { Extract port value }
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
  InCorsSection: Boolean;
  ServerPortUpdated: Boolean;
  CorsOriginsUpdated: Boolean;
  ServerPort: String;
  ResultCode: Integer;
  NssmPath: String;
begin
  if CurStep = ssPostInstall then
  begin
    { Merge config.toml }
    ConfigFile := ExpandConstant('{app}\config.toml');
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

    { Update port in config.toml }
    if FileExists(ConfigFile) then
    begin
      LoadStringsFromFile(ConfigFile, ConfigContent);
      InServerSection := False;
      InCorsSection := False;
      ServerPortUpdated := False;
      CorsOriginsUpdated := False;

      for I := 0 to GetArrayLength(ConfigContent) - 1 do
      begin
        Line := Trim(ConfigContent[I]);

        { Track which section we're in }
        if Line = '[server]' then
        begin
          InServerSection := True;
          InCorsSection := False;
        end
        else if Line = '[cors]' then
        begin
          InServerSection := False;
          InCorsSection := True;
        end
        else if (Length(Line) > 0) and (Line[1] = '[') then
        begin
          InServerSection := False;
          InCorsSection := False;
        end;

        { Update port value }
        if InServerSection and (Pos('port = ', ConfigContent[I]) > 0) then
        begin
          ConfigContent[I] := 'port = ' + ServerPort;
          ServerPortUpdated := True;
        end;

        { Update CORS allowed_origins if present }
        if InCorsSection and (Pos('allowed_origins = ', ConfigContent[I]) > 0) then
        begin
          ConfigContent[I] := 'allowed_origins = ["http://127.0.0.1:' + ServerPort + '"]';
          CorsOriginsUpdated := True;
        end;
      end;

      SaveStringsToFile(ConfigFile, ConfigContent, False);
    end;

    { Install Windows service using NSSM }
    Exec(NssmPath, 'install SankeyCopierServer "' + ExpandConstant('{app}\{#MyServerExeName}') + '"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

    { Configure Server service }
    Exec(NssmPath, 'set SankeyCopierServer DisplayName "SANKEY Copier Server"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec(NssmPath, 'set SankeyCopierServer Description "Backend server for SANKEY Copier MT4/MT5 trade copying system"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec(NssmPath, 'set SankeyCopierServer AppDirectory "' + ExpandConstant('{app}') + '"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    { Server uses config.toml [logging] settings - no NSSM log redirection needed }
    Exec(NssmPath, 'set SankeyCopierServer Start SERVICE_AUTO_START', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

    { Start service (non-blocking to prevent hang) }
    Exec(NssmPath, 'start SankeyCopierServer', '', SW_HIDE, ewNoWait, ResultCode);
  end;
end;

function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
  NssmPath: String;
begin
  Result := True;

  { Try to use nssm.exe if available, otherwise fall back to sc.exe }
  NssmPath := ExpandConstant('{app}\nssm.exe');

  if FileExists(NssmPath) then
  begin
    { Use nssm to stop service }
    Exec(NssmPath, 'stop SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Sleep(2000);
  end
  else
  begin
    { Fallback to sc.exe }
    if Exec('sc.exe', 'query SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
    begin
      if ResultCode = 0 then
      begin
        Exec('sc.exe', 'stop SankeyCopierServer', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
        Sleep(2000);
      end;
    end;
  end;
end;
