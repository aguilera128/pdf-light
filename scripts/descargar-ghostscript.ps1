# Descarga a vendor\ el instalador oficial de Ghostscript que se empaqueta en el instalador de PDF Light.
# No se guarda en el repositorio (65 MB); se comprueba contra el SHA-256 fijado aquí.
# Al subir de versión: cambiar $archivo, $url y $sha256, y GsFile/GsVersion en installer\pdflight.iss.
$ErrorActionPreference = 'Stop'
$archivo = 'gs10080w64.exe'
$url     = "https://github.com/ArtifexSoftware/ghostpdl-downloads/releases/download/gs10080/$archivo"
$sha256  = '52a91b8bf09298788d7a57b9206127026c23eacd75405f0a131e26dc381dce50'

$destino = Join-Path (Split-Path $PSScriptRoot -Parent) "vendor\$archivo"
New-Item -ItemType Directory -Force (Split-Path $destino) | Out-Null
if (-not (Test-Path $destino)) {
    Write-Host "Descargando $url"
    Invoke-WebRequest -Uri $url -OutFile $destino -UseBasicParsing
}
$real = (Get-FileHash $destino -Algorithm SHA256).Hash.ToLower()
if ($real -ne $sha256) {
    Remove-Item $destino -Force
    throw "SHA-256 inesperado en $archivo ($real). Se ha borrado la descarga."
}
Write-Host "OK: $destino"
