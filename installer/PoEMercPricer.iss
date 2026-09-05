; Compile through scripts/build-installer.ps1. This installer is fully offline.
#if Ver != EncodeVer(6, 7, 3)
  #error The pinned Inno Setup 6.7.3 compiler is required.
#endif
#ifndef AppVersion
  #error AppVersion must come from Cargo metadata.
#endif
#ifndef AppIdentity
  #define AppIdentity "PoEMercPricer"
#endif
#ifndef AppTitle
  #define AppTitle "PoEMercPricer"
#endif
#define AppExe "poemercpricer.exe"

[Setup]
AppId={#AppIdentity}
AppName={#AppTitle}
AppVersion={#AppVersion}
AppVerName={#AppTitle} {#AppVersion}
AppPublisher=Lisood
AppPublisherURL=https://github.com/Lisood/PoEMercPricer
AppSupportURL=https://github.com/Lisood/PoEMercPricer/issues
AppUpdatesURL=https://github.com/Lisood/PoEMercPricer/releases
DefaultDirName={localappdata}\Programs\{#AppTitle}
DefaultGroupName={#AppTitle}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
UsePreviousAppDir=yes
DisableDirPage=auto
UninstallDisplayName={#AppTitle}
UninstallDisplayIcon={app}\{#AppExe}
AppMutex=Local\PoEMercPricer.Running,Global\PoEMercPricer.Running
SetupMutex=Local\{#AppIdentity}.Setup
CloseApplications=no
RestartApplications=no
SetupLogging=yes
Compression=lzma2/normal
SolidCompression=yes
LZMAUseSeparateProcess=yes
OutputDir={#OutputPath}
OutputBaseFilename=poemercpricer-setup-windows-x64
SetupIconFile=..\assets\branding\app-icon.ico
WizardStyle=modern dark
WizardSizePercent=110
WizardSmallImageFile=..\assets\branding\icons\app-icon-128.png
WizardImageFile=..\assets\branding\icons\app-icon-256.png
WizardImageBackColor=$08090A
WizardSmallImageBackColor=$08090A
WizardBackColor=$08090A
DisableWelcomePage=no
DisableReadyPage=no
SignedUninstaller=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Messages]
WelcomeLabel1=Your overlay. Ready to stay.
WelcomeLabel2=Install {#AppTitle} for your Windows account.%n%nScan Mercenary Warrants, inspect their skills, and keep the overlay close at hand.%n%nNo administrator password or extra runtime needed. Updates stay built in.
FinishedHeadingLabel=Ready for your next Warrant.
FinishedLabel={#AppTitle} is installed. Open it from the Start menu whenever you play.%n%nPress Ctrl+Shift+M to scan. You choose when to restart after an update.
SelectDirLabel3=Choose a permanent folder for the app. Keep it under your user account so automatic updates can be installed.
ReadyLabel1=Setup is ready to give {#AppTitle} a permanent home.
UninstalledAll={#AppTitle} was removed.%n%nYour settings and saved debug captures have been kept.

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "{#PayloadPath}"; DestDir: "{app}"; DestName: "{#AppExe}"; Flags: ignoreversion
Source: "{#NoticesPath}"; DestDir: "{app}"; DestName: "THIRD_PARTY_NOTICES-{#AppVersion}.html"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppTitle}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"; AppUserModelID: "{#AppIdentity}"
Name: "{autodesktop}\{#AppTitle}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"; Tasks: desktopicon; AppUserModelID: "{#AppIdentity}"

[Run]
Filename: "{app}\{#AppExe}"; Description: "Open {#AppTitle}"; Flags: nowait postinstall skipifsilent unchecked

[UninstallDelete]
; Only updater-owned files. Never recursively delete the app or settings folder.
Type: files; Name: "{app}\poemercpricer-previous.exe"
Type: files; Name: "{app}\poemercpricer.exe.*.update"
Type: files; Name: "{app}\THIRD_PARTY_NOTICES-*.html"

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  InstalledVersion, SetupVersion: Int64;
  RegisteredDir: String;
begin
  Result := '';
  if RegQueryStringValue(HKCU64, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#AppIdentity}_is1', 'InstallLocation', RegisteredDir) then
    if CompareText(RemoveBackslashUnlessRoot(RegisteredDir), RemoveBackslashUnlessRoot(ExpandConstant('{app}'))) <> 0 then begin
      Result := 'This account already has an installation in ' + RegisteredDir + '. Use that folder, or uninstall it before choosing another location.';
      Exit;
    end;
  if GetPackedVersion(ExpandConstant('{app}\{#AppExe}'), InstalledVersion) then begin
    if not StrToVersion('{#AppVersion}.0', SetupVersion) then begin
      Result := 'Setup contains an invalid version.';
      Exit;
    end;
    if ComparePackedVersion(InstalledVersion, SetupVersion) > 0 then
      Result := 'A newer version is already installed. Download the latest setup from the Releases page. Your installed app has been kept.';
  end;
end;

procedure InitializeWizard;
begin
  WizardForm.WizardBitmapImage.SetBounds(ScaleX(28), ScaleY(32), ScaleX(112), ScaleY(112));
  WizardForm.WizardBitmapImage2.SetBounds(ScaleX(28), ScaleY(32), ScaleX(112), ScaleY(112));
  WizardForm.WelcomeLabel1.Font.Size := 18;
  WizardForm.WelcomeLabel1.Height := ScaleY(72);
  WizardForm.WelcomeLabel2.Top := WizardForm.WelcomeLabel1.Top + WizardForm.WelcomeLabel1.Height + ScaleY(12);
  WizardForm.WelcomeLabel2.Height := WizardForm.WelcomePage.Height - WizardForm.WelcomeLabel2.Top - ScaleY(24);
  WizardForm.WelcomeLabel1.Font.Color := $5BA0C7;
  WizardForm.FinishedHeadingLabel.Font.Color := $5BA0C7;
  WizardForm.BeveledLabel.Caption := '  MERCENARY WARRANT SCREENER  ';
end;
