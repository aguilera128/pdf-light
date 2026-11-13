//! Aviso de versiones nuevas: pregunta a GitHub cuál es la última publicada. No descarga ni instala nada.

use std::ptr::{null, null_mut};
use windows_sys::Win32::Networking::WinHttp::*;

const HOST: &str = "api.github.com";

/// Ruta de la API a partir de la dirección del repositorio ("https://github.com/usuario/repo").
fn api_path(repository: &str) -> Option<String> {
    let path = repository.trim_end_matches('/').strip_prefix("https://github.com")?;
    Some(format!("/repos{path}/releases/latest"))
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// "v1.2.3" o "1.2.3" -> (1, 2, 3).
pub fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.trim().trim_start_matches(['v', 'V']).split('.').map(|p| p.parse::<u32>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// Versión de la respuesta de la API si es posterior a `current`.
pub fn newer_version(response: &[u8], current: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_slice(response).ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    let latest = parse_version(tag)?;
    (latest > parse_version(current)?).then(|| format!("{}.{}.{}", latest.0, latest.1, latest.2))
}

/// Devuelve la versión publicada más reciente si es posterior a la de esta app.
/// Cualquier fallo (sin red, límite de la API, respuesta rara) se trata como "no hay novedad".
pub fn check(repository: &str, current: &str) -> Option<String> {
    newer_version(&get(&api_path(repository)?, current)?, current)
}

fn get(path: &str, current: &str) -> Option<Vec<u8>> {
    // Cierra los identificadores de WinHTTP al salir por cualquier camino.
    struct Handle(*mut core::ffi::c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { WinHttpCloseHandle(self.0) };
        }
    }
    fn open(raw: *mut core::ffi::c_void) -> Option<Handle> {
        (!raw.is_null()).then_some(Handle(raw))
    }

    unsafe {
        // GitHub exige un User-Agent; solo lleva el nombre y la versión de la app.
        let agent = wide(&format!("PDFLight/{current}"));
        let session = open(WinHttpOpen(agent.as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, null(), null(), 0))?;
        WinHttpSetTimeouts(session.0, 10_000, 10_000, 10_000, 10_000);
        let connection = open(WinHttpConnect(session.0, wide(HOST).as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0))?;
        let request = open(WinHttpOpenRequest(
            connection.0,
            wide("GET").as_ptr(),
            wide(path).as_ptr(),
            null(),
            null(),
            null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        if WinHttpSendRequest(request.0, null(), 0, null(), 0, 0, 0) == 0 || WinHttpReceiveResponse(request.0, null_mut()) == 0 {
            return None;
        }
        let mut status = 0u32;
        let mut size = size_of::<u32>() as u32;
        let queried = WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            null(),
            &mut status as *mut u32 as *mut _,
            &mut size,
            null_mut(),
        );
        if queried == 0 || status != 200 {
            return None;
        }
        let mut body = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let mut read = 0u32;
            if WinHttpReadData(request.0, chunk.as_mut_ptr() as *mut _, chunk.len() as u32, &mut read) == 0 {
                return None;
            }
            if read == 0 {
                return Some(body);
            }
            body.extend_from_slice(&chunk[..read as usize]);
            if body.len() > 1 << 20 {
                return None; // la respuesta real ocupa unos pocos KB
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.5.0"), Some((0, 5, 0)));
        assert_eq!(parse_version("1.10.2"), Some((1, 10, 2)));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("v1.2.x"), None);
        assert_eq!(newer_version(br#"{"tag_name":"v0.10.0","name":"x"}"#, "0.9.3").as_deref(), Some("0.10.0"));
        assert_eq!(newer_version(br#"{"tag_name":"v0.5.0"}"#, "0.5.0"), None);
        assert_eq!(newer_version(br#"{"tag_name":"v0.4.9"}"#, "0.5.0"), None);
        assert_eq!(newer_version(b"<html>rate limit</html>", "0.5.0"), None);
        assert_eq!(newer_version(br#"{"message":"Not Found"}"#, "0.5.0"), None);
    }

    /// Consulta real a GitHub: `cargo test --release -- --ignored`.
    #[test]
    #[ignore = "necesita red"]
    fn live_check() {
        let repository = env!("CARGO_PKG_REPOSITORY");
        assert!(check(repository, "0.0.1").is_some(), "debería haber una versión posterior a la 0.0.1");
        assert_eq!(check(repository, "999.0.0"), None);
    }

    #[test]
    fn api_paths() {
        assert_eq!(api_path("https://github.com/usuario/repo").as_deref(), Some("/repos/usuario/repo/releases/latest"));
        assert_eq!(api_path("https://example.com/x"), None);
    }
}
