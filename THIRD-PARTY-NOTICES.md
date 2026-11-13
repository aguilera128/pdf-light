# Third-party software

## Ghostscript

The PDF Light installer includes, unmodified, the official **GPL Ghostscript 10.08.0** installer
for 64-bit Windows (`gs10080w64.exe`), by Artifex Software, Inc. PDF Light runs it as a separate
program (`gswin64c.exe`) to convert PostScript to PDF.

- License: GNU Affero General Public License, version 3 (the same as PDF Light; text in `LICENSE`).
  Once installed, its copy of the license is at `C:\Program Files\gs\gs10.08.0\doc\COPYING`.
- Binary: <https://github.com/ArtifexSoftware/ghostpdl-downloads/releases/tag/gs10080>
- Corresponding source code: `ghostscript-10.08.0.tar.gz`, on that same page.
- Website: <https://ghostscript.com>

Ghostscript is a trademark of Artifex Software, Inc. PDF Light is not affiliated with or endorsed
by Artifex.

## Rust libraries

`pdflight.exe` statically links these libraries (see `Cargo.lock` for exact versions), all under
the MIT or Apache-2.0 license at your option: `windows-sys`, `serde`, `serde_json` and their
dependencies.
