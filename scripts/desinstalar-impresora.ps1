#Requires -RunAsAdministrator
# Quita la impresora "PDF Light" y su puerto. No toca Ghostscript ni el driver PostScript de Windows.
$ErrorActionPreference = 'Stop'
$puerto = Join-Path $env:ProgramData 'PDFLight\spool\trabajo.ps'
if (Get-Printer -Name 'PDF Light' -ErrorAction SilentlyContinue) { Remove-Printer -Name 'PDF Light' }
if (Get-PrinterPort -Name $puerto -ErrorAction SilentlyContinue) { Remove-PrinterPort -Name $puerto }
Write-Host 'Impresora PDF Light eliminada.'
