//! Lógica sin interfaz: ajustes, cola de trabajos, títulos y Ghostscript.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use windows_sys::Win32::Globalization::{CP_ACP, MultiByteToWideChar};

/// Valores de -dPDFSETTINGS de Ghostscript; sus nombres visibles están en `i18n::Strings::qualities`.
pub const QUALITIES: [&str; 5] = ["screen", "ebook", "printer", "prepress", "default"];

/// Por qué ha fallado una conversión; el texto para el usuario lo pone `main` según el idioma.
#[derive(Debug)]
pub enum ConvertError {
    NoGhostscript,
    Launch(String),
    Failed { status: String, log: String },
    CreateDir(String),
    Write { path: String, error: String },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase", default)]
pub struct Settings {
    pub output_folder: String,
    pub quality: String,
    pub open_after_save: bool,
    pub auto_save: bool,
    /// Nombre del PDF al guardar sin preguntar: admite {title}, {date} y {time}.
    pub auto_name_pattern: String,
    pub ghostscript_path: Option<String>,
    /// "en", "es", "fr", "de", "pt" o "it"; sin valor = el idioma de Windows.
    pub language: Option<String>,
    /// Preguntar a GitHub una vez al día si hay una versión nueva.
    pub check_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let docs = env_dir("USERPROFILE").join("Documents");
        Settings {
            output_folder: docs.to_string_lossy().into_owned(),
            quality: "ebook".into(),
            open_after_save: true,
            auto_save: false,
            auto_name_pattern: "{title}".into(),
            ghostscript_path: None,
            language: None,
            check_updates: true,
        }
    }
}

impl Settings {
    fn path() -> PathBuf {
        env_dir("APPDATA").join("PDFLight").join("settings.json")
    }

    pub fn load() -> Settings {
        // Sin fichero o corrupto: valores por defecto.
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        let _ = fs::create_dir_all(path.parent().unwrap());
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}

fn env_dir(var: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(var).unwrap_or_default())
}

/// Fichero donde escribe el puerto local de la impresora (lo crea el instalador).
pub fn spool_file() -> PathBuf {
    env_dir("ProgramData").join("PDFLight").join("spool").join("trabajo.ps")
}

/// Trabajos ya recogidos, a la espera de que el usuario los guarde.
pub fn jobs_dir() -> PathBuf {
    std::env::temp_dir().join("PDFLight")
}

#[derive(Clone)]
pub struct Job {
    pub ps: PathBuf,
    pub title: String,
}

impl Job {
    pub fn from_file(ps: PathBuf) -> Job {
        let title = read_title(&ps);
        Job { ps, title }
    }
}

/// Trabajos que quedaron sin guardar en una sesión anterior.
pub fn pending_jobs() -> Vec<Job> {
    let mut files: Vec<PathBuf> = fs::read_dir(jobs_dir())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ps"))
        .collect();
    files.sort();
    files.into_iter().map(Job::from_file).collect()
}

/// Si el spooler ha terminado de escribir un trabajo, lo mueve a `jobs_dir` y lo devuelve.
pub fn take_spooled_job(stamp: &str) -> Option<Job> {
    let src = spool_file();
    if fs::metadata(&src).ok()?.len() == 0 {
        return None;
    }
    // Abrirlo sin compartir solo funciona cuando el spooler ya lo ha soltado.
    drop(fs::OpenOptions::new().read(true).share_mode(0).open(&src).ok()?);
    fs::create_dir_all(jobs_dir()).ok()?;
    let dst = jobs_dir().join(format!("{stamp}.ps"));
    // Si falla es que otro trabajo ha empezado a escribirse: se recoge en la siguiente vuelta.
    move_file(&src, &dst).ok()?;
    Some(Job::from_file(dst))
}

fn move_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    // Distinto volumen: copiar y borrar.
    fs::copy(src, dst)?;
    fs::remove_file(src)
}

fn read_title(ps: &Path) -> String {
    let mut buf = vec![0u8; 8192]; // la cabecera DSC va al principio
    let n = fs::File::open(ps).and_then(|mut f| f.read(&mut buf)).unwrap_or(0);
    parse_title(&buf[..n], ansi_to_string)
}

/// Texto en la página de códigos ANSI del sistema -> String.
fn ansi_to_string(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let mut wide = vec![0u16; bytes.len()];
    let n = unsafe { MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), bytes.len() as i32, wide.as_mut_ptr(), wide.len() as i32) };
    String::from_utf16_lossy(&wide[..n.max(0) as usize])
}

/// Título del documento según el comentario DSC `%%Title`.
/// pscript5 lo escribe como cadena PostScript `(…)` en la página ANSI del sistema, con los
/// caracteres no ASCII en octal (`a\361o`); lo que no cabe en esa página llega ya como `?`.
pub fn parse_title(header: &[u8], decode_ansi: impl Fn(&[u8]) -> String) -> String {
    for line in header.split(|&b| b == b'\n') {
        let line = line.trim_ascii();
        if line.starts_with(b"%%EndComments") {
            break;
        }
        let Some(v) = line.strip_prefix(b"%%Title:") else { continue };
        let v = v.trim_ascii();
        return match v {
            [b'<', hex @ .., b'>'] => decode_hex(hex, &decode_ansi),
            [b'(', inner @ .., b')'] => decode_ansi(&unescape_ps(inner)),
            _ => decode_ansi(v),
        };
    }
    String::new()
}

/// Deshace los escapes de una cadena PostScript: `\(`, `\)`, `\\`, `\n`... y `\ooo` en octal.
fn unescape_ps(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i] != b'\\' || i + 1 == s.len() {
            out.push(s[i]);
            i += 1;
            continue;
        }
        i += 1;
        match s[i] {
            b'0'..=b'7' => {
                let mut value = 0u32;
                let mut digits = 0;
                while digits < 3 && i < s.len() && (b'0'..=b'7').contains(&s[i]) {
                    value = value * 8 + (s[i] - b'0') as u32;
                    i += 1;
                    digits += 1;
                }
                out.push(value as u8);
                continue;
            }
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            other => out.push(other), // \( \) \\ y cualquier otro: el carácter tal cual
        }
        i += 1;
    }
    out
}

fn decode_hex(hex: &[u8], decode_ansi: &impl Fn(&[u8]) -> String) -> String {
    let digits: Vec<u8> = hex.iter().filter_map(|b| (*b as char).to_digit(16).map(|d| d as u8)).collect();
    let bytes: Vec<u8> = digits.chunks(2).map(|c| c[0] << 4 | c.get(1).copied().unwrap_or(0)).collect();
    match bytes.as_slice() {
        [0xFE, 0xFF, rest @ ..] => {
            let units: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        }
        _ => decode_ansi(&bytes),
    }
}

const INVALID: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

pub fn is_valid_file_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|c| INVALID.contains(&c) || c.is_control())
}

/// Nombre de fichero (sin extensión) propuesto a partir del título del trabajo.
pub fn file_name_from_title(title: &str, fallback: &str) -> String {
    let mut t = title.trim();
    for p in ["Microsoft Word - ", "Microsoft Excel - ", "Microsoft PowerPoint - "] {
        if t.len() >= p.len() && t.is_char_boundary(p.len()) && t[..p.len()].eq_ignore_ascii_case(p) {
            t = &t[p.len()..];
        }
    }
    // Las URL y rutas llevan barras: quedarse con el último tramo.
    t = t.trim_end_matches(['/', '\\']);
    if let Some(i) = t.rfind(['/', '\\']) {
        t = &t[i + 1..];
    }
    if let Some(i) = t.rfind('.') {
        let ext = &t[i + 1..];
        if i > 0 && (1..=4).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            t = &t[..i];
        }
    }
    let clean: String = t.chars().map(|c| if INVALID.contains(&c) || c.is_control() { '_' } else { c }).collect();
    let clean: String = clean.trim_matches([' ', '.']).chars().take(120).collect();
    if clean.is_empty() { fallback.into() } else { clean }
}

/// Sube o baja un puesto los elementos seleccionados (índices en orden ascendente) y devuelve
/// dónde quedan. Los que ya están en el borde, o detrás de otro seleccionado que no puede moverse,
/// se quedan donde están.
pub fn move_selected<T>(items: &mut [T], selected: &[usize], up: bool) -> Vec<usize> {
    let mut is_selected = vec![false; items.len()];
    for &i in selected {
        if i < items.len() {
            is_selected[i] = true;
        }
    }
    let n = items.len();
    let order: Box<dyn Iterator<Item = usize>> = if up { Box::new(1..n) } else { Box::new((0..n.saturating_sub(1)).rev()) };
    for i in order {
        let neighbour = if up { i - 1 } else { i + 1 };
        if is_selected[i] && !is_selected[neighbour] {
            items.swap(i, neighbour);
            is_selected.swap(i, neighbour);
        }
    }
    (0..n).filter(|&i| is_selected[i]).collect()
}

/// Nombre (sin extensión) para el guardado automático: sustituye {title}, {date} y {time} del patrón.
/// `date` y `time` llegan ya formateados (AAAA-MM-DD y HH-MM-SS).
pub fn auto_name(pattern: &str, title_name: &str, date: &str, time: &str) -> String {
    let name = pattern.replace("{title}", title_name).replace("{date}", date).replace("{time}", time);
    let name: String = name.chars().map(|c| if INVALID.contains(&c) || c.is_control() { '_' } else { c }).collect();
    let name = name.trim_matches([' ', '.']);
    if name.is_empty() { title_name.to_string() } else { name.to_string() }
}

/// Ghostscript solo cifra con la revisión 3 del estándar (RC4 de 128 bits), que limita la contraseña
/// a 32 bytes y no define cómo codificar caracteres no ASCII: se admiten solo los ASCII imprimibles.
pub fn is_valid_password(password: &str) -> bool {
    password.len() <= 32 && password.chars().all(|c| c == ' ' || c.is_ascii_graphic())
}

/// Devuelve `path`, o `path` con sufijo _1, _2... si ya existe.
pub fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    (1..)
        .map(|i| path.with_file_name(format!("{stem}_{i}.pdf")))
        .find(|p| !p.exists())
        .unwrap()
}

pub fn find_ghostscript(settings: &Settings) -> Option<PathBuf> {
    if let Some(p) = settings.ghostscript_path.as_deref().map(PathBuf::from) {
        if p.is_file() {
            return Some(p);
        }
    }
    // NOTA: solo se mira la carpeta por defecto del instalador oficial (Program Files\gs\gsX.Y.Z);
    // para otra ubicación está GhostscriptPath en settings.json.
    let root = env_dir("ProgramFiles").join("gs");
    let mut found: Vec<(Vec<u32>, PathBuf)> = fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path().join("bin").join("gswin64c.exe"))
        .filter(|p| p.is_file())
        .map(|p| (version_key(&p), p))
        .collect();
    found.sort();
    found.pop().map(|(_, p)| p)
}

/// "…\gs10.08.0\bin\…" -> [10, 8, 0], para que 10.x gane a 9.x.
fn version_key(exe: &Path) -> Vec<u32> {
    let dir = exe.parent().and_then(Path::parent).and_then(Path::file_name).unwrap_or_default();
    dir.to_string_lossy().split(|c: char| !c.is_ascii_digit()).filter_map(|n| n.parse().ok()).collect()
}

pub fn ghostscript_args(inputs: &[PathBuf], output: &Path, quality: &str, title: &str, password: Option<&str>) -> Vec<String> {
    let quality = if QUALITIES.contains(&quality) { quality } else { "ebook" };
    let mut args: Vec<String> = [
        "-dNOPAUSE",
        "-dBATCH",
        "-dSAFER",
        "-q",
        "-sDEVICE=pdfwrite",
        "-dCompatibilityLevel=1.7",
        "-dEmbedAllFonts=true",
        "-dAutoRotatePages=/PageByPage",
    ]
    .map(String::from)
    .to_vec();
    args.push(format!("-dPDFSETTINGS=/{quality}"));
    args.push(format!("-sOutputFile={}", output.display()));
    args.extend(inputs.iter().map(|p| p.display().to_string()));
    // Metadatos al final para que manden sobre los del propio PostScript.
    // Cadena hex UTF-16BE: no hay que escapar nada y admite cualquier carácter.
    if let Some(password) = password.filter(|p| !p.is_empty()) {
        // Misma contraseña de usuario y de propietario: quien abre el PDF puede hacer todo con él.
        // NOTA: va en la línea de comandos de Ghostscript, visible para otros procesos del mismo
        // usuario mientras dura la conversión; es lo que admite gswin64c.
        args.insert(0, format!("-sUserPassword={password}"));
        args.insert(0, format!("-sOwnerPassword={password}"));
        args.insert(0, "-dKeyLength=128".into());
        args.insert(0, "-dEncryptionR=3".into());
    }
    let hex: String = title.encode_utf16().map(|u| format!("{u:04X}")).collect();
    args.push("-c".into());
    args.push(format!("[ /Title <FEFF{hex}> /Creator (PDF Light) /DOCINFO pdfmark"));
    args
}

/// Convierte uno o varios PostScript en un único PDF y, si sale bien, borra los PostScript.
pub fn convert(settings: &Settings, inputs: &[PathBuf], output: &Path, title: &str, password: Option<&str>) -> Result<(), ConvertError> {
    let gs = find_ghostscript(settings).ok_or(ConvertError::NoGhostscript)?;
    // Se genera en temporal y se mueve al final: nunca queda un PDF a medias en el destino.
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let tmp = jobs_dir().join(format!("salida-{}-{}.pdf", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::create_dir_all(jobs_dir());
    let result = (|| {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let out = Command::new(&gs)
            .args(ghostscript_args(inputs, &tmp, &settings.quality, title, password))
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| ConvertError::Launch(e.to_string()))?;
        let produced = fs::metadata(&tmp).map(|m| m.len() > 0).unwrap_or(false);
        if !out.status.success() || !produced {
            let log = [out.stdout, out.stderr].concat();
            let log = String::from_utf8_lossy(&log);
            let tail: String = log.trim().chars().rev().take(600).collect::<Vec<_>>().into_iter().rev().collect();
            return Err(ConvertError::Failed { status: out.status.to_string(), log: tail });
        }
        if let Some(dir) = output.parent() {
            fs::create_dir_all(dir).map_err(|e| ConvertError::CreateDir(e.to_string()))?;
        }
        let _ = fs::remove_file(output);
        move_file(&tmp, output).map_err(|e| ConvertError::Write { path: output.display().to_string(), error: e.to_string() })
    })();
    let _ = fs::remove_file(&tmp);
    if result.is_ok() {
        for ps in inputs {
            let _ = fs::remove_file(ps);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles() {
        // Para los tests, Latin-1 hace de "página ANSI del sistema".
        let title = |h: &[u8]| parse_title(h, |b| b.iter().map(|&c| c as char).collect());
        assert_eq!(title(b"%!PS-Adobe-3.0\r\n%%Title: Factura 12.docx\r\n%%EndComments\r\n"), "Factura 12.docx");
        assert_eq!(title(b"%!PS\n%%Title: (Informe)\n"), "Informe");
        assert_eq!(title(b"%!PS\n%%Title: <FEFF00430061006D006900F3006E>\n"), "Camión");
        assert_eq!(title(b"%!PS\n%%Title: a\xF1o\n"), "año");
        assert_eq!(title(b"%!PS\n%%EndComments\n%%Title: tarde\n"), "");
        // Tal como lo escribe pscript5 (capturado de una impresión real).
        assert_eq!(title(b"%%Title: (Presupuesto a\\361o 2026 ni\\361o.docx)\r\n"), "Presupuesto año 2026 niño.docx");
        assert_eq!(title(b"%%Title: (Par\\351ntesis \\(a\\) y \\\\barra)\r\n"), "Paréntesis (a) y \\barra");
        assert_eq!(title(b"%%Title: (fin\\)\r\n"), "fin\\");
    }

    #[test]
    fn file_names() {
        let name = |t: &str| file_name_from_title(t, "Documento");
        assert_eq!(name("Microsoft Word - Factura 12.docx"), "Factura 12");
        assert_eq!(name("https://example.com/a/b?x=1"), "b_x=1");
        assert_eq!(name("  "), "Documento");
        assert_eq!(name("v1.2 notas"), "v1.2 notas");
        assert_eq!(name("Ñandú.pdf"), "Ñandú");
        assert!(is_valid_file_name("a b"));
        assert!(!is_valid_file_name("a:b"));
    }

    #[test]
    fn reorder() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        assert_eq!(move_selected(&mut v, &[2], true), [1]);
        assert_eq!(v, ['a', 'c', 'b', 'd']);
        assert_eq!(move_selected(&mut v, &[0, 1], true), [0, 1]); // ya están arriba del todo
        assert_eq!(v, ['a', 'c', 'b', 'd']);
        assert_eq!(move_selected(&mut v, &[0, 2], false), [1, 3]);
        assert_eq!(v, ['c', 'a', 'd', 'b']);
        assert_eq!(move_selected(&mut v, &[3], false), [3]);
        assert_eq!(move_selected(&mut v, &[9], true), Vec::<usize>::new());
        assert_eq!(move_selected(&mut Vec::<char>::new(), &[], false), Vec::<usize>::new());
    }

    #[test]
    fn gs_args() {
        let plain = ghostscript_args(&["a.ps".into()], Path::new("o.pdf"), "ebook", "t", Some(""));
        assert!(!plain.iter().any(|a| a.contains("Password") || a.contains("Encryption")));
        let secret = ghostscript_args(&["a.ps".into()], Path::new("o.pdf"), "ebook", "t", Some("abc 1"));
        assert!(secret.contains(&"-sUserPassword=abc 1".to_string()) && secret.contains(&"-dEncryptionR=3".to_string()));
        assert!(is_valid_password("abc 1!") && !is_valid_password("año") && !is_valid_password(&"x".repeat(33)));
        assert_eq!(auto_name("{date} {title}", "Factura", "2026-11-13", "09-30-00"), "2026-11-13 Factura");
        assert_eq!(auto_name("a/b:{time}", "x", "d", "09-30-00"), "a_b_09-30-00");
        assert_eq!(auto_name("  ", "Factura", "d", "t"), "Factura");
        let args = ghostscript_args(&["a.ps".into(), "b.ps".into()], Path::new(r"C:\o ut.pdf"), "nope", "Ñ", None);
        assert!(args.contains(&"-dPDFSETTINGS=/ebook".to_string()));
        assert!(args.contains(&r"-sOutputFile=C:\o ut.pdf".to_string()));
        let pos = |s: &str| args.iter().position(|a| a == s).unwrap();
        assert!(pos("a.ps") < pos("b.ps") && pos("b.ps") < pos("-c"));
        assert_eq!(args.last().unwrap(), "[ /Title <FEFF00D1> /Creator (PDF Light) /DOCINFO pdfmark");
    }

    #[test]
    fn gs_versions() {
        let key = |d: &str| version_key(&PathBuf::from(format!(r"C:\Program Files\gs\{d}\bin\gswin64c.exe")));
        assert!(key("gs10.08.0") > key("gs9.56.1"));
    }
}
