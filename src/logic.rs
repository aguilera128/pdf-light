//! Lógica sin interfaz: ajustes, cola de trabajos, títulos y Ghostscript.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

pub const QUALITIES: [(&str, &str); 5] = [
    ("screen", "Pantalla (72 ppp, mínimo tamaño)"),
    ("ebook", "Estándar (150 ppp)"),
    ("printer", "Alta (300 ppp)"),
    ("prepress", "Imprenta (300 ppp, color fiel)"),
    ("default", "Sin recomprimir imágenes"),
];

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase", default)]
pub struct Settings {
    pub output_folder: String,
    pub quality: String,
    pub open_after_save: bool,
    pub auto_save: bool,
    pub ghostscript_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        let docs = env_dir("USERPROFILE").join("Documents");
        Settings {
            output_folder: docs.to_string_lossy().into_owned(),
            quality: "ebook".into(),
            open_after_save: true,
            auto_save: false,
            ghostscript_path: None,
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
    parse_title(&buf[..n])
}

/// Título del documento según el comentario DSC `%%Title` de pscript5.
pub fn parse_title(header: &[u8]) -> String {
    for line in header.split(|&b| b == b'\n') {
        let line = line.trim_ascii();
        if line.starts_with(b"%%EndComments") {
            break;
        }
        let Some(v) = line.strip_prefix(b"%%Title:") else { continue };
        let v = v.trim_ascii();
        if v.len() > 1 && v[0] == b'<' && v[v.len() - 1] == b'>' {
            return decode_hex(&v[1..v.len() - 1]);
        }
        let v = match v {
            [b'(', inner @ .., b')'] => inner,
            _ => v,
        };
        // NOTA: los títulos no ASCII sin hex se leen como Latin-1; si salen mal,
        // convertir desde la página de códigos ANSI con MultiByteToWideChar.
        return v.iter().map(|&b| b as char).collect();
    }
    String::new()
}

fn decode_hex(hex: &[u8]) -> String {
    let digits: Vec<u8> = hex.iter().filter_map(|b| (*b as char).to_digit(16).map(|d| d as u8)).collect();
    let bytes: Vec<u8> = digits.chunks(2).map(|c| c[0] << 4 | c.get(1).copied().unwrap_or(0)).collect();
    match bytes.as_slice() {
        [0xFE, 0xFF, rest @ ..] => {
            let units: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        }
        _ => bytes.iter().map(|&b| b as char).collect(),
    }
}

const INVALID: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

pub fn is_valid_file_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|c| INVALID.contains(&c) || c.is_control())
}

/// Nombre de fichero (sin extensión) propuesto a partir del título del trabajo.
pub fn file_name_from_title(title: &str) -> String {
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
    if clean.is_empty() { "Documento".into() } else { clean }
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

pub fn ghostscript_args(inputs: &[PathBuf], output: &Path, quality: &str, title: &str) -> Vec<String> {
    let quality = if QUALITIES.iter().any(|q| q.0 == quality) { quality } else { "ebook" };
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
    let hex: String = title.encode_utf16().map(|u| format!("{u:04X}")).collect();
    args.push("-c".into());
    args.push(format!("[ /Title <FEFF{hex}> /Creator (PDF Light) /DOCINFO pdfmark"));
    args
}

/// Convierte uno o varios PostScript en un único PDF y, si sale bien, borra los PostScript.
pub fn convert(settings: &Settings, inputs: &[PathBuf], output: &Path, title: &str) -> Result<(), String> {
    let gs = find_ghostscript(settings).ok_or(
        "No se encuentra Ghostscript (gswin64c.exe). Instálalo o indica su ruta en GhostscriptPath de %AppData%\\PDFLight\\settings.json.",
    )?;
    // Se genera en temporal y se mueve al final: nunca queda un PDF a medias en el destino.
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let tmp = jobs_dir().join(format!("salida-{}-{}.pdf", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::create_dir_all(jobs_dir());
    let result = (|| {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let out = Command::new(&gs)
            .args(ghostscript_args(inputs, &tmp, &settings.quality, title))
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("No se pudo ejecutar Ghostscript: {e}"))?;
        let produced = fs::metadata(&tmp).map(|m| m.len() > 0).unwrap_or(false);
        if !out.status.success() || !produced {
            let log = [out.stdout, out.stderr].concat();
            let log = String::from_utf8_lossy(&log);
            let tail: String = log.trim().chars().rev().take(600).collect::<Vec<_>>().into_iter().rev().collect();
            return Err(format!("Ghostscript falló ({}).\n{tail}", out.status));
        }
        if let Some(dir) = output.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("No se pudo crear la carpeta: {e}"))?;
        }
        let _ = fs::remove_file(output);
        move_file(&tmp, output).map_err(|e| format!("No se pudo escribir «{}»: {e}", output.display()))
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
        assert_eq!(parse_title(b"%!PS-Adobe-3.0\r\n%%Title: Factura 12.docx\r\n%%EndComments\r\n"), "Factura 12.docx");
        assert_eq!(parse_title(b"%!PS\n%%Title: (Informe)\n"), "Informe");
        assert_eq!(parse_title(b"%!PS\n%%Title: <FEFF00430061006D006900F3006E>\n"), "Camión");
        assert_eq!(parse_title(b"%!PS\n%%Title: a\xF1o\n"), "año");
        assert_eq!(parse_title(b"%!PS\n%%EndComments\n%%Title: tarde\n"), "");
    }

    #[test]
    fn file_names() {
        assert_eq!(file_name_from_title("Microsoft Word - Factura 12.docx"), "Factura 12");
        assert_eq!(file_name_from_title("https://example.com/a/b?x=1"), "b_x=1");
        assert_eq!(file_name_from_title("  "), "Documento");
        assert_eq!(file_name_from_title("v1.2 notas"), "v1.2 notas");
        assert_eq!(file_name_from_title("Ñandú.pdf"), "Ñandú");
        assert!(is_valid_file_name("a b"));
        assert!(!is_valid_file_name("a:b"));
    }

    #[test]
    fn gs_args() {
        let args = ghostscript_args(&["a.ps".into(), "b.ps".into()], Path::new(r"C:\o ut.pdf"), "nope", "Ñ");
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
