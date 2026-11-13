# PDF Light

[English](README.md)

Impresora virtual a PDF para Windows 10 y 11 (64 bits). Imprime desde cualquier programa en la
impresora **PDF Light** y guarda el resultado como PDF.

- Ventana de guardado con nombre, carpeta y calidad.
- Une varios trabajos de impresión en un solo PDF, en el orden que elijas.
- Contraseña opcional para abrir el PDF.
- Guardado automático opcional, sin preguntar, con patrón de nombre (`{title}`, `{date}`, `{time}`).
- Un ejecutable de unos 400 KB escrito en Rust, sin runtime; vive en la bandeja del sistema.
- Avisa cuando hay una versión nueva (opcional).
- Interfaz en inglés, español, francés, alemán, portugués (Brasil) e italiano. Sigue el idioma
  de Windows y se puede cambiar en Opciones (menú de la bandeja).

## Instalar

Descarga `PDFLight-Setup-<versión>.exe` de la
[última versión](https://github.com/aguilera128/pdf-light/releases/latest) y ejecútalo. Pide permisos de
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

PDF Light no tiene telemetría y nunca envía a ningún sitio tus documentos, sus nombres ni tus ajustes.

La única petición de red que hace es para saber si hay una versión nueva: una vez al día (y al
minuto de arrancar) pregunta a `api.github.com` cuál es la última versión de este proyecto. Esa
petición lleva el nombre y la versión de la aplicación, y GitHub ve la dirección IP del equipo, como
en cualquier petición web. Si hay una versión posterior, muestra un aviso y una entrada en el menú de
la bandeja que abren la página de descarga; nunca descarga ni instala nada por su cuenta. La consulta
se puede desactivar durante la instalación o después en Opciones.

Los ajustes se guardan en `%AppData%\PDFLight\settings.json`.

## Limitaciones

- La contraseña usa RC4 de 128 bits (revisión 3 del cifrado PDF), lo más fuerte que ofrece
  Ghostscript. Evita que el fichero se abra sin más; no te fíes de ella para documentos sensibles.
  Admite hasta 32 caracteres sin acentos (ASCII).
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
