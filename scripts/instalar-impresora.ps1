#Requires -RunAsAdministrator
# Crea la impresora virtual "PDF Light": driver PostScript de Windows + puerto local a fichero.
# La app (PdfLight.exe) vigila ese fichero y convierte cada trabajo con Ghostscript.
# Uso (PowerShell como administrador):  .\instalar-impresora.ps1 [-GhostscriptInstaller ruta\gsXXXXXw64.exe]
param([string]$GhostscriptInstaller)

$ErrorActionPreference = 'Stop'
$nombre = 'PDF Light'
# Driver v3 (pscript5) incluido en Windows. El "Microsoft PS Class Driver" (v4) no sirve:
# pone "%%Title: MSxpsPS" en vez del nombre del documento.
$driver = 'MS Publisher Color Printer'
$spool  = Join-Path $env:ProgramData 'PDFLight\spool'
$puerto = Join-Path $spool 'trabajo.ps'

New-Item -ItemType Directory -Force $spool | Out-Null
Start-Transcript -Path (Join-Path $env:ProgramData 'PDFLight\instalar.log') -Force | Out-Null
try {
    if ($GhostscriptInstaller) {
        Write-Host "Instalando Ghostscript desde $GhostscriptInstaller"
        $p = Start-Process $GhostscriptInstaller -ArgumentList '/S' -Wait -PassThru
        if ($p.ExitCode -ne 0) { throw "El instalador de Ghostscript terminó con código $($p.ExitCode)" }
    }

    # El spooler escribe el trabajo aquí y la app (sin privilegios) lo mueve: Usuarios necesita Modificar.
    # NOTA: carpeta compartida por todos los usuarios del equipo; con varias sesiones a la vez,
    # pasar a un monitor de puerto que entregue el trabajo a la sesión que imprime.
    icacls $spool /grant '*S-1-5-32-545:(OI)(CI)M' | Out-Null

    if (-not (Get-PrinterPort -Name $puerto -ErrorAction SilentlyContinue)) { Add-PrinterPort -Name $puerto }
    if (-not (Get-PrinterDriver -Name $driver -ErrorAction SilentlyContinue)) { Add-PrinterDriver -Name $driver }
    if (-not (Get-Printer -Name $nombre -ErrorAction SilentlyContinue)) {
        Add-Printer -Name $nombre -DriverName $driver -PortName $puerto
    }
    else { Set-Printer -Name $nombre -DriverName $driver -PortName $puerto }
    Get-Printer -Name $nombre | Format-List Name, DriverName, PortName
    Write-Host 'OK'
}
catch { Write-Host "ERROR: $_"; throw }
finally { Stop-Transcript | Out-Null }
