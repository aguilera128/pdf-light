# Software de terceros

## Ghostscript

El instalador de PDF Light incluye, sin modificar, el instalador oficial de
**GPL Ghostscript 10.08.0** para Windows de 64 bits (`gs10080w64.exe`), de Artifex Software, Inc.
PDF Light lo ejecuta como programa aparte (`gswin64c.exe`) para convertir PostScript en PDF.

- Licencia: GNU Affero General Public License, versión 3 (la misma que PDF Light; texto en `LICENSE`).
  Tras instalarlo, su copia de la licencia está en `C:\Program Files\gs\gs10.08.0\doc\COPYING`.
- Binario: <https://github.com/ArtifexSoftware/ghostpdl-downloads/releases/tag/gs10080>
- Código fuente correspondiente: `ghostscript-10.08.0.tar.gz`, en esa misma página.
- Web: <https://ghostscript.com>

Ghostscript es una marca de Artifex Software, Inc. PDF Light no está afiliado a Artifex ni avalado por ella.

## Bibliotecas de Rust

`pdflight.exe` enlaza estáticamente estas bibliotecas (ver `Cargo.lock` para las versiones exactas),
todas bajo licencia MIT o Apache-2.0 a elección: `windows-sys`, `serde`, `serde_json` y sus dependencias.
