; Instalador de PDF Light (Inno Setup 7).
; Antes:     cargo build --release   y   scripts\descargar-ghostscript.ps1
; Compilar:  ISCC.exe installer\pdflight.iss
; Salida:    dist\PDFLight-Setup-<versión>.exe

#define AppVersion "0.3.1"
; Instalador oficial de Ghostscript (AGPL), empaquetado tal cual desde vendor\.
#define GsFile "gs10080w64.exe"

[Setup]
AppId={{6B0E7E0C-5D0B-4E0B-9C55-2F6C1B7A9D41}
AppName=PDF Light
AppVersion={#AppVersion}
AppPublisher=Javier Aguilera
AppPublisherURL=https://github.com/aguilera128/pdf-light
DefaultDirName={autopf}\PDF Light
DisableProgramGroupPage=yes
DisableDirPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern dynamic
LicenseFile=..\LICENSE
SetupIconFile=..\res\pdflight.ico
OutputDir=..\dist
OutputBaseFilename=PDFLight-Setup-{#AppVersion}
UninstallDisplayIcon={app}\pdflight.exe
Compression=lzma2
SolidCompression=no
CloseApplications=no

[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"

[Files]
Source: "..\target\release\pdflight.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\scripts\instalar-impresora.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\scripts\desinstalar-impresora.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\THIRD-PARTY-NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
; Solo se extrae si el equipo no tiene ya Ghostscript. Ya viene comprimido: no se recomprime.
Source: "..\vendor\{#GsFile}"; DestDir: "{tmp}"; Flags: deleteafterinstall nocompression; Check: not HasGhostscript

[Icons]
Name: "{autoprograms}\PDF Light"; Filename: "{app}\pdflight.exe"

[Registry]
; La app tiene que estar siempre en marcha: el puerto de la impresora sobrescribe el trabajo anterior.
Root: HKLM; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "PDFLight"; ValueData: """{app}\pdflight.exe"""; Flags: uninsdeletevalue
; Entrada por usuario que dejaba la versión 0.1.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "PDFLight"; Flags: deletevalue dontcreatekey

[Run]
Filename: "{app}\pdflight.exe"; Flags: nowait runasoriginaluser

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/f /im pdflight.exe"; Flags: runhidden; RunOnceId: "CerrarApp"
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\desinstalar-impresora.ps1"""; Flags: runhidden; RunOnceId: "QuitarImpresora"

[Code]
function HasGhostscript: Boolean;
var
  Rec: TFindRec;
  Root: String;
begin
  Result := False;
#ifdef ForceGs
  // Solo para probar la instalación de Ghostscript en un equipo que ya lo tiene: ISCC /DForceGs
  Exit;
#endif
  Root := ExpandConstant('{commonpf64}\gs\');
  if FindFirst(Root + '*', Rec) then
  try
    repeat
      if FileExists(Root + Rec.Name + '\bin\gswin64c.exe') then
        Result := True;
    until Result or not FindNext(Rec);
  finally
    FindClose(Rec);
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
begin
  Result := '';
  // La app no guarda estado en memoria: los trabajos pendientes están en disco.
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/f /im pdflight.exe', '', SW_HIDE, ewWaitUntilTerminated, Code);
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
  Params, Gs: String;
begin
  if CurStep <> ssPostInstall then
    Exit;
  WizardForm.StatusLabel.Caption := 'Instalando Ghostscript y creando la impresora PDF Light...';
  Params := ExpandConstant('-NoProfile -ExecutionPolicy Bypass -File "{app}\instalar-impresora.ps1"');
  // El instalador de Ghostscript lo lanza el script: ejecutado directamente con Exec desde aquí se queda colgado.
  Gs := ExpandConstant('{tmp}\{#GsFile}');
  if FileExists(Gs) then
    Params := Params + ' -GhostscriptInstaller "' + Gs + '"';
  if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Params,
       '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
    SuppressibleMsgBox('No se pudo instalar Ghostscript o crear la impresora PDF Light (código ' + IntToStr(Code) + ').' + #13#10 +
      'Detalles en C:\ProgramData\PDFLight\instalar.log', mbError, MB_OK, IDOK);
end;
