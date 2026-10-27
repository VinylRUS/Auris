; Установщик Auris (Inno Setup 6).
;
; Сборка (из корня репозитория, Windows):
;   set AURIS_VERSION=0.0.1b
;   iscc installer\auris.iss
;
; Ключевые решения:
;  * установка per-user в {localappdata}\Auris — не требует прав администратора
;    и не ломает автообновление (приложение само скачивает и запускает сетап);
;  * CloseApplications=force — установщик сам закрывает запущенный Auris
;    при обновлении;
;  * иконка и версия берутся из assets/icon.ico и переменной AURIS_VERSION.

#ifndef AURIS_VERSION
#define AURIS_VERSION GetEnv("AURIS_VERSION")
#endif
#if AURIS_VERSION == ""
#define AURIS_VERSION "0.0.1b"
#endif

#define MyAppName "Auris"
#define MyAppPublisher "VinylRUS"
#define MyAppURL "https://github.com/VinylRUS/Auris"
#define MyAppExeName "auris.exe"

[Setup]
AppId={{8F3A2C71-5B64-4E9A-9D1F-3A7B6C2E4A58}
AppName={#MyAppName}
AppVersion={#AURIS_VERSION}
AppVerName={#MyAppName} {#AURIS_VERSION}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
; per-user: без UAC, обновления ставятся молча
DefaultDirName={localappdata}\{#MyAppName}
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
ShowLanguageDialog=auto
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
MinVersion=10.0
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\installer\out
OutputBaseFilename=Auris-Setup-{#AURIS_VERSION}-x64
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
; При обновлении закрываем запущенное приложение
CloseApplications=force
RestartApplications=no

[Languages]
Name: "ru"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion; Languages: ru

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Конфиг и кеш остаются у пользователя (в %LOCALAPPDATA%/auris),
; удаляем только каталог программы.
Type: filesandordirs; Name: "{app}"
