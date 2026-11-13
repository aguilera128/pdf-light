# PDF Light

[Español](README.es.md)

A virtual PDF printer for Windows 10 and 11 (64-bit). Print from any program to the **PDF Light**
printer and save the result as a PDF.

- Save window with name, folder and quality.
- Merges several print jobs into a single PDF, in the order you choose.
- Optional automatic saving, without asking.
- A single executable of about 400 KB written in Rust, with no runtime; it lives in the system tray.
- Interface in English, Spanish, French, German, Portuguese (Brazil) and Italian. It follows the
  Windows display language.

## Install

Download `PDFLight-Setup-<version>.exe` from the releases page and run it. It asks for
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

PDF Light does not send any data to other systems: it has no telemetry and does not check for
updates. Settings are stored in `%AppData%\PDFLight\settings.json`.

To force a language, add `"Language": "en"` (or `es`, `fr`, `de`, `pt`, `it`) to that file.

## Limitations

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
