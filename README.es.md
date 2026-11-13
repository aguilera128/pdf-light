# PDF Light

[English](README.md)

Impresora virtual a PDF para Windows 10 y 11 (64 bits). Imprime desde cualquier programa en la
impresora **PDF Light** y guarda el resultado como PDF.

- Ventana de guardado con nombre, carpeta y calidad.
- Une varios trabajos de impresión en un solo PDF, en el orden que elijas.
- Guardado automático opcional, sin preguntar.
- Un ejecutable de unos 400 KB escrito en Rust, sin runtime; vive en la bandeja del sistema.
- Interfaz en inglés, español, francés, alemán, portugués (Brasil) e italiano. Sigue el idioma
  de Windows.

## Instalar

Descarga `PDFLight-Setup-<versión>.exe` de la página de versiones y ejecútalo. Pide permisos de
administrador y hace estos cambios en el equipo:

- Copia la aplicación a `C:\Program Files\PDF Light`.
- Instala Ghostscript 10.08.0 en `C:\Program Files\gs` si el equipo no lo tiene.
- Crea la impresora «PDF Light» (driver `MS Publisher Color Printer` de Windows) con un puerto local
  al fichero `C:\ProgramData\PDFLight\spool\trabajo.ps`.
- Hace que PDF Light arranque al iniciar sesión (`HKLM\...\Run`).

El instalador no está firmado, así que Windows SmartScreen mostrará un aviso: «Más información» →
«Ejecutar de todas formas».

## Desinstalar

Desde «Aplicaciones instaladas» de Windows. Quita la aplicación, la impresora y el arranque
automático. Ghostscript se queda instalado; se desinstala aparte desde el mismo sitio.

## Privacidad

PDF Light no envía ningún dato a otros sistemas: no tiene telemetría ni comprueba actualizaciones.
Los ajustes se guardan en `%AppData%\PDFLight\settings.json`.

Para forzar un idioma, añade `"Language": "es"` (o `en`, `fr`, `de`, `pt`, `it`) a ese fichero.

## Limitaciones

- La aplicación tiene que estar en marcha: si no lo está, solo se conserva el último trabajo impreso.
- Pensada para un usuario por equipo a la vez (la carpeta de trabajos es común).
- El nombre del documento llega en la página de códigos ANSI del sistema, así que los caracteres
  que no caben en ella (por ejemplo japonés en un Windows de Europa occidental) llegan como `?`.

## Compilar

Hacen falta Rust (toolchain MSVC) e Inno Setup 7.

```powershell
.\scripts\compilar.ps1
```

Pasa los tests, compila, descarga el instalador oficial de Ghostscript a `vendor\` (comprobando su
SHA-256) y deja el instalador en `dist\`.

## Licencia

PDF Light se distribuye bajo la [GNU Affero General Public License v3.0 o posterior](LICENSE).
Incluye el instalador de Ghostscript, de Artifex Software; ver
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
