# PDF Light

[Español](README.es.md)

A virtual PDF printer for Windows 10 and 11 (64-bit). Print from any program to the **PDF Light**
printer and save the result as a PDF.

- Save window with name, folder and quality.
- Merges several print jobs into a single PDF, in the order you choose.
- Optional password to open the PDF.
- Optional automatic saving, without asking, with a file name pattern (`{title}`, `{date}`, `{time}`).
- A single executable of about 400 KB written in Rust, with no runtime; it lives in the system tray.
- Tells you when a new version is available (optional).
- Interface in English, Spanish, French, German, Portuguese (Brazil) and Italian. It follows the
  Windows display language and can be changed in Options (tray menu).

## Install

Download `PDFLight-Setup-<version>.exe` from the
[latest release](https://github.com/aguilera128/pdf-light/releases/latest) and run it. It asks for
administrator rights and makes these changes to the computer:

- Copies the application to `C:\Program Files\PDF Light`.
- Installs Ghostscript 10.08.0 to `C:\Program Files\gs` if the computer does not have it.
- Creates the “PDF Light” printer (Windows `MS Publisher Color Printer` driver) with a local port
  that writes to `C:\ProgramData\PDFLight\spool\trabajo.ps`.
- Makes PDF Light start when you sign in (`HKLM\...\Run`).

The installer is not signed, so Windows SmartScreen will show a warning: “More info” → “Run anyway”.

## Uninstall

From “Installed apps” in Windows. It removes the application, the printer and the automatic start.
Ghostscript stays installed; uninstall it separately from the same place.

## Privacy

PDF Light has no telemetry and never sends your documents, their names or any setting anywhere.

The only network request it makes is to check for a new version: once a day (and one minute after
starting) it asks `api.github.com` for the latest release of this project. That request carries the
application name and version, and GitHub sees the computer's IP address, as with any web request.
If there is a newer version it shows a notification and a tray menu entry that open the download
page; it never downloads or installs anything by itself. You can turn the check off during
installation or later in Options.

Settings are stored in `%AppData%\PDFLight\settings.json`.

## Limitations

- Password protection uses 128-bit RC4 (PDF encryption revision 3), the strongest Ghostscript
  offers. It keeps the file from being opened casually; do not rely on it for sensitive documents.
  Passwords are limited to 32 unaccented (ASCII) characters.
- The application has to be running: if it is not, only the last printed job is kept.
- Meant for one user per computer at a time (the job folder is shared).
- Document names are received in the system's ANSI code page, so characters outside it
  (for example Japanese on a Western European system) arrive as `?`.

## Build

You need Rust (MSVC toolchain) and Inno Setup 7.

```powershell
.\scripts\compilar.ps1
```

It runs the tests, builds, downloads the official Ghostscript installer to `vendor\` (checking its
SHA-256) and leaves the installer in `dist\`.

## License

PDF Light is distributed under the [GNU Affero General Public License v3.0 or later](LICENSE).
It bundles the Ghostscript installer, by Artifex Software; see
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
