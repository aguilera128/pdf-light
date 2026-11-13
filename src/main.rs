//! PDF Light: icono de bandeja + diálogo de guardado. Interfaz Win32 pura (plantilla en res/pdflight.rc).
#![windows_subsystem = "windows"]

mod i18n;
mod logic;

use i18n::{fill, t};
use logic::{ConvertError, Job, Settings};
use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, Ordering};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::Controls::{BST_CHECKED, CheckDlgButton, EM_SETCUEBANNER, EM_SETSEL, IsDlgButtonChecked};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// Identificadores de res/pdflight.rc.
const IDC_LIST: i32 = 101;
const IDC_NAME: i32 = 102;
const IDC_FOLDER: i32 = 103;
const IDC_BROWSE: i32 = 104;
const IDC_QUALITY: i32 = 105;
const IDC_OPEN: i32 = 106;
const IDC_AUTO: i32 = 107;
const IDC_DISCARD: i32 = 108;
const IDC_INTRO: i32 = 110;
const IDC_LBL_NAME: i32 = 111;
const IDC_LBL_FOLDER: i32 = 112;
const IDC_LBL_QUALITY: i32 = 113;
const IDC_LBL_PASSWORD: i32 = 116;
const IDC_PASSWORD: i32 = 117;
// Diálogo de opciones (recurso 2).
const IDO_LBL_LANGUAGE: i32 = 201;
const IDO_LANGUAGE: i32 = 202;
const IDO_LBL_PATTERN: i32 = 203;
const IDO_PATTERN: i32 = 204;
const IDO_HELP: i32 = 205;
const IDM_OPTIONS: i32 = 206; // orden «Opciones» del menú de la bandeja
const IDC_UP: i32 = 114;
const IDC_DOWN: i32 = 115;
const ID_SAVE: i32 = 1; // IDOK
const PROJECT_URL: &str = env!("CARGO_PKG_REPOSITORY");
const ID_CLOSE: i32 = 2; // IDCANCEL (Esc)

const WM_TRAY: u32 = WM_APP + 1;
const WM_DONE: u32 = WM_APP + 2;

struct App {
    settings: Settings,
    jobs: Vec<Job>,
    busy: bool,
    options_open: bool,
    taskbar_created: u32,
}

/// Resultado de una conversión, enviado del hilo de trabajo a la ventana con WM_DONE.
struct Done {
    jobs: Vec<Job>,
    target: PathBuf,
    result: Result<(), ConvertError>,
    auto: bool,
}

thread_local! {
    static APP: RefCell<App> = RefCell::new(App { settings: Settings::load(), jobs: Vec::new(), busy: false, options_open: false, taskbar_created: 0 });
}

/// Acceso corto al estado. No llamar a nada modal (MessageBox, menús) dentro de `f`:
/// el temporizador volvería a entrar aquí con el estado ya prestado.
fn app<R>(f: impl FnOnce(&mut App) -> R) -> R {
    APP.with(|a| f(&mut a.borrow_mut()))
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn copy_w(s: &str, dst: &mut [u16]) {
    let n = dst.len() - 1;
    for (d, c) in dst.iter_mut().zip(s.encode_utf16().take(n).chain(Some(0))) {
        *d = c;
    }
}

fn main() {
    unsafe {
        CreateMutexW(null(), 1, w("Local\\PDFLight.SingleInstance").as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return; // ya hay una instancia vigilando la cola
        }
        CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32);
        i18n::init(app(|a| a.settings.language.clone()).as_deref());
        let hwnd = CreateDialogParamW(GetModuleHandleW(null()), 1 as *const u16, null_mut(), Some(dlg_proc), 0);
        if hwnd.is_null() {
            return;
        }
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

unsafe extern "system" fn dlg_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    unsafe {
        match msg {
            WM_INITDIALOG => {
                apply_texts(hwnd);
                sync_from_settings(hwnd);
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, app_icon(SM_CXSMICON) as isize);
                SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, app_icon(SM_CXICON) as isize);
                app(|a| a.taskbar_created = RegisterWindowMessageW(w("TaskbarCreated").as_ptr()));
                tray(hwnd, NIM_ADD, None);
                // El instalador arranca la app con --welcome: sin este aviso no se ve que haya pasado nada.
                if std::env::args().any(|a| a == "--welcome") {
                    tray(hwnd, NIM_MODIFY, Some((t().ready_title, t().ready_text, false)));
                }
                SetTimer(hwnd, 1, 400, None);
                for job in logic::pending_jobs() {
                    on_job(hwnd, job);
                }
                return 1;
            }
            // NOTA: sondeo cada 400 ms de un único fichero; no hace falta vigilar la carpeta.
            WM_TIMER => {
                static SEQ: AtomicU32 = AtomicU32::new(0);
                let secs = std::time::UNIX_EPOCH.elapsed().map(|d| d.as_secs()).unwrap_or(0);
                let stamp = format!("{secs}-{:04}", SEQ.fetch_add(1, Ordering::Relaxed) % 10000);
                if let Some(job) = logic::take_spooled_job(&stamp) {
                    on_job(hwnd, job);
                }
            }
            WM_COMMAND => match ((wparam & 0xFFFF) as i32, ((wparam >> 16) & 0xFFFF) as u32) {
                (ID_SAVE, _) => save(hwnd),
                (ID_CLOSE, _) => hide(hwnd),
                (IDC_BROWSE, _) => browse(hwnd),
                (IDC_DISCARD, _) => discard(hwnd),
                (IDM_OPTIONS, _) => options(hwnd),
                (IDC_UP, _) => move_jobs(hwnd, true),
                (IDC_DOWN, _) => move_jobs(hwnd, false),
                (IDC_LIST, LBN_SELCHANGE) => selection_changed(hwnd),
                _ => return 0,
            },
            // Cerrar solo oculta: los trabajos pendientes siguen ahí (icono de la bandeja).
            WM_CLOSE => hide(hwnd),
            WM_TRAY => match lparam as u32 & 0xFFFF {
                WM_LBUTTONDBLCLK => reveal(hwnd),
                WM_RBUTTONUP => tray_menu(hwnd),
                _ => {}
            },
            WM_DONE => on_done(hwnd, *Box::from_raw(lparam as *mut Done)),
            WM_DESTROY => {
                tray(hwnd, NIM_DELETE, None);
                PostQuitMessage(0);
            }
            // El Explorador se ha reiniciado: hay que volver a poner el icono.
            m if m != 0 && m == app(|a| a.taskbar_created) => tray(hwnd, NIM_ADD, None),
            _ => return 0,
        }
        1
    }
}

/// Pone los textos del idioma activo. La plantilla del .rc va en inglés solo como reserva.
unsafe fn apply_texts(hwnd: HWND) {
    unsafe {
        for (id, text) in [
            (IDC_INTRO, t().intro),
            (IDC_LBL_NAME, t().name),
            (IDC_LBL_FOLDER, t().folder),
            (IDC_LBL_QUALITY, t().quality),
            (IDC_LBL_PASSWORD, t().password),
            (IDC_BROWSE, t().browse),
            (IDC_OPEN, t().open_after),
            (IDC_AUTO, t().auto_save),
            (IDC_DISCARD, t().discard),
            (ID_SAVE, t().save),
            (IDC_UP, "▲"),
            (IDC_DOWN, "▼"),
        ] {
            set_text(hwnd, id, text);
        }
        SendMessageW(GetDlgItem(hwnd, IDC_PASSWORD), EM_SETCUEBANNER, 1, w(t().password_hint).as_ptr() as isize);
        let combo = GetDlgItem(hwnd, IDC_QUALITY);
        SendMessageW(combo, CB_RESETCONTENT, 0, 0);
        for label in t().qualities {
            SendMessageW(combo, CB_ADDSTRING, 0, w(label).as_ptr() as isize);
        }
    }
}

/// Fecha y hora locales para los nombres automáticos: ("AAAA-MM-DD", "HH-MM-SS").
fn now() -> (String, String) {
    let mut t = unsafe { zeroed() };
    unsafe { GetLocalTime(&mut t) };
    (format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay), format!("{:02}-{:02}-{:02}", t.wHour, t.wMinute, t.wSecond))
}

unsafe fn options(hwnd: HWND) {
    unsafe {
        if app(|a| std::mem::replace(&mut a.options_open, true)) {
            return; // ya está abierta
        }
        let changed = DialogBoxParamW(GetModuleHandleW(null()), 2 as *const u16, null_mut(), Some(options_proc), 0) == 1;
        app(|a| a.options_open = false);
        if changed {
            i18n::init(app(|a| a.settings.language.clone()).as_deref());
            apply_texts(hwnd);
            sync_from_settings(hwnd);
            selection_changed(hwnd);
        }
    }
}

unsafe extern "system" fn options_proc(hwnd: HWND, msg: u32, wparam: WPARAM, _lparam: LPARAM) -> isize {
    unsafe {
        match msg {
            WM_INITDIALOG => {
                for (id, text) in [
                    (IDO_LBL_LANGUAGE, t().opt_language),
                    (IDO_LBL_PATTERN, t().opt_pattern),
                    (IDO_HELP, t().opt_pattern_help),
                    (ID_SAVE, t().ok),
                    (ID_CLOSE, t().cancel),
                ] {
                    set_text(hwnd, id, text);
                }
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, app_icon(SM_CXSMICON) as isize);
                let (language, pattern) = app(|a| (a.settings.language.clone(), a.settings.auto_name_pattern.clone()));
                let combo = GetDlgItem(hwnd, IDO_LANGUAGE);
                SendMessageW(combo, CB_ADDSTRING, 0, w(t().opt_auto_lang).as_ptr() as isize);
                for (_, name) in i18n::LANGUAGES {
                    SendMessageW(combo, CB_ADDSTRING, 0, w(name).as_ptr() as isize);
                }
                // 0 = automático; el resto, en el orden de i18n::LANGUAGES.
                let current = language.and_then(|l| i18n::LANGUAGES.iter().position(|(code, _)| l.eq_ignore_ascii_case(code))).map_or(0, |i| i + 1);
                SendMessageW(combo, CB_SETCURSEL, current, 0);
                set_text(hwnd, IDO_PATTERN, &pattern);
                SetForegroundWindow(hwnd);
                1
            }
            WM_COMMAND => match (wparam & 0xFFFF) as i32 {
                ID_SAVE => {
                    let choice = SendMessageW(GetDlgItem(hwnd, IDO_LANGUAGE), CB_GETCURSEL, 0, 0);
                    let pattern = get_text(hwnd, IDO_PATTERN).trim().to_string();
                    app(|a| {
                        a.settings.language = (choice > 0).then(|| i18n::LANGUAGES[(choice as usize - 1).min(i18n::LANGUAGES.len() - 1)].0.to_string());
                        a.settings.auto_name_pattern = if pattern.is_empty() { "{title}".to_string() } else { pattern };
                        a.settings.save();
                    });
                    EndDialog(hwnd, 1);
                    1
                }
                ID_CLOSE => {
                    EndDialog(hwnd, 0);
                    1
                }
                _ => 0,
            },
            _ => 0,
        }
    }
}

unsafe fn tray(hwnd: HWND, action: NOTIFY_ICON_MESSAGE, balloon: Option<(&str, &str, bool)>) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = app_icon(SM_CXSMICON);
        copy_w("PDF Light", &mut nid.szTip);
        if let Some((title, text, error)) = balloon {
            nid.uFlags |= NIF_INFO;
            copy_w(title, &mut nid.szInfoTitle);
            copy_w(text, &mut nid.szInfo);
            nid.dwInfoFlags = if error { NIIF_ERROR } else { NIIF_INFO };
        }
        Shell_NotifyIconW(action, &nid);
    }
}

/// Icono de la app (recurso 1 de res/pdflight.rc) al tamaño que pide el sistema.
unsafe fn app_icon(metric: SYSTEM_METRICS_INDEX) -> HICON {
    unsafe {
        let size = GetSystemMetrics(metric);
        LoadImageW(GetModuleHandleW(null()), 1 as *const u16, IMAGE_ICON, size, size, LR_SHARED)
    }
}

unsafe fn tray_menu(hwnd: HWND) {
    unsafe {
        let auto = app(|a| a.settings.auto_save);
        let menu = CreatePopupMenu();
        AppendMenuW(menu, MF_STRING, 1, w(t().tray_pending).as_ptr());
        AppendMenuW(menu, MF_STRING, 2, w(t().tray_open_folder).as_ptr());
        AppendMenuW(menu, MF_STRING | if auto { MF_CHECKED } else { 0 }, 3, w(t().tray_auto).as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, 6, w(t().tray_options).as_ptr());
        AppendMenuW(menu, MF_STRING, 5, w(t().tray_about).as_ptr());
        AppendMenuW(menu, MF_STRING, 4, w(t().tray_exit).as_ptr());
        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd); // sin esto el menú no se cierra al pulsar fuera
        let cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, null());
        DestroyMenu(menu);
        match cmd {
            1 => reveal(hwnd),
            2 => open(Path::new(&app(|a| a.settings.output_folder.clone()))),
            3 => {
                app(|a| {
                    a.settings.auto_save = !auto;
                    a.settings.save();
                });
                sync_from_settings(hwnd);
            }
            4 => {
                DestroyWindow(hwnd);
            }
            6 => {
                PostMessageW(hwnd, WM_COMMAND, IDM_OPTIONS as usize, 0);
            }
            5 => {
                let text = fill(t().about, &[env!("CARGO_PKG_VERSION"), PROJECT_URL]);
                if message(hwnd, &text, MB_YESNO | MB_ICONINFORMATION | MB_SETFOREGROUND) == IDYES {
                    open(Path::new(PROJECT_URL));
                }
            }
            _ => {}
        }
    }
}

/// Sube o baja los trabajos seleccionados: el PDF unido sigue el orden de la lista.
unsafe fn move_jobs(hwnd: HWND, up: bool) {
    unsafe {
        let sel = selected(hwnd);
        if sel.is_empty() || app(|a| a.busy) {
            return;
        }
        let moved = app(|a| logic::move_selected(&mut a.jobs, &sel, up));
        refresh_list(hwnd, &moved);
    }
}

unsafe fn open(path: &Path) {
    unsafe {
        ShellExecuteW(null_mut(), w("open").as_ptr(), w(&path.to_string_lossy()).as_ptr(), null(), null(), SW_SHOWNORMAL);
    }
}

unsafe fn message(hwnd: HWND, text: &str, style: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    unsafe { MessageBoxW(hwnd, w(text).as_ptr(), w("PDF Light").as_ptr(), style) }
}

unsafe fn get_text(hwnd: HWND, id: i32) -> String {
    unsafe {
        let ctl = GetDlgItem(hwnd, id);
        let mut buf = vec![0u16; GetWindowTextLengthW(ctl) as usize + 1];
        let n = GetWindowTextW(ctl, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

unsafe fn set_text(hwnd: HWND, id: i32, text: &str) {
    unsafe {
        SetDlgItemTextW(hwnd, id, w(text).as_ptr());
    }
}

unsafe fn sync_from_settings(hwnd: HWND) {
    unsafe {
        let s = app(|a| a.settings.clone());
        set_text(hwnd, IDC_FOLDER, &s.output_folder);
        let quality = logic::QUALITIES.iter().position(|q| *q == s.quality).unwrap_or(1);
        SendMessageW(GetDlgItem(hwnd, IDC_QUALITY), CB_SETCURSEL, quality, 0);
        CheckDlgButton(hwnd, IDC_OPEN, s.open_after_save as u32);
        CheckDlgButton(hwnd, IDC_AUTO, s.auto_save as u32);
    }
}

unsafe fn reveal(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_SHOWNORMAL);
        // El trabajo llega mientras el usuario está en otra app y Windows no deja robar el foco:
        // subirla un instante a "siempre visible" la pone delante sin dejarla fija.
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        SetForegroundWindow(hwnd);
        let name = GetDlgItem(hwnd, IDC_NAME);
        SetFocus(name);
        SendMessageW(name, EM_SETSEL, 0, -1);
    }
}

unsafe fn hide(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_HIDE);
    }
}

/// Índices seleccionados, en el orden de la lista.
unsafe fn selected(hwnd: HWND) -> Vec<usize> {
    unsafe {
        let list = GetDlgItem(hwnd, IDC_LIST);
        let n = SendMessageW(list, LB_GETSELCOUNT, 0, 0).max(0) as usize;
        let mut items = vec![0i32; n];
        if n > 0 {
            SendMessageW(list, LB_GETSELITEMS, n, items.as_mut_ptr() as isize);
        }
        items.sort();
        items.into_iter().map(|i| i as usize).collect()
    }
}

/// Vuelve a pintar la lista desde `App::jobs` y deja seleccionados los índices de `select`.
unsafe fn refresh_list(hwnd: HWND, select: &[usize]) {
    unsafe {
        let list = GetDlgItem(hwnd, IDC_LIST);
        SendMessageW(list, LB_RESETCONTENT, 0, 0);
        for title in app(|a| a.jobs.iter().map(|j| j.title.clone()).collect::<Vec<_>>()) {
            let label = if title.is_empty() { t().untitled.to_string() } else { title };
            SendMessageW(list, LB_ADDSTRING, 0, w(&label).as_ptr() as isize);
        }
        for &i in select {
            SendMessageW(list, LB_SETSEL, 1, i as isize);
        }
        selection_changed(hwnd);
    }
}

unsafe fn selection_changed(hwnd: HWND) {
    unsafe {
        let sel = selected(hwnd);
        let busy = app(|a| a.busy);
        let enabled = (!sel.is_empty() && !busy) as i32;
        EnableWindow(GetDlgItem(hwnd, ID_SAVE), enabled);
        EnableWindow(GetDlgItem(hwnd, IDC_DISCARD), enabled);
        EnableWindow(GetDlgItem(hwnd, IDC_LIST), !busy as i32);
        EnableWindow(GetDlgItem(hwnd, IDC_UP), enabled);
        EnableWindow(GetDlgItem(hwnd, IDC_DOWN), enabled);
        let label = if busy {
            t().saving.to_string()
        } else if sel.len() > 1 {
            fill(t().merge_save, &[&sel.len().to_string()])
        } else {
            t().save.to_string()
        };
        set_text(hwnd, ID_SAVE, &label);
        if let Some(title) = sel.first().and_then(|&i| app(|a| a.jobs.get(i).map(|j| j.title.clone()))) {
            set_text(hwnd, IDC_NAME, &logic::file_name_from_title(&title, t().default_name));
        }
    }
}

/// Llega un trabajo de la impresora (o uno pendiente de una sesión anterior).
unsafe fn on_job(hwnd: HWND, job: Job) {
    unsafe {
        if app(|a| a.settings.auto_save) {
            let folder = app(|a| a.settings.output_folder.clone());
            let title = logic::file_name_from_title(&job.title, t().default_name);
            let (date, time) = now();
            let name = logic::auto_name(&app(|a| a.settings.auto_name_pattern.clone()), &title, &date, &time);
            let target = logic::unique_path(Path::new(&folder).join(format!("{name}.pdf")));
            spawn_convert(hwnd, vec![job], target, title, None, true);
        } else {
            add_to_list(hwnd, job);
        }
    }
}

unsafe fn add_to_list(hwnd: HWND, job: Job) {
    unsafe {
        let first = app(|a| {
            a.jobs.push(job.clone());
            a.jobs.len() == 1
        });
        if first {
            refresh_list(hwnd, &[0]);
        } else {
            // No se toca la selección: el usuario puede estar a medias de elegir.
            let label = if job.title.is_empty() { t().untitled } else { &job.title };
            SendMessageW(GetDlgItem(hwnd, IDC_LIST), LB_ADDSTRING, 0, w(label).as_ptr() as isize);
        }
        reveal(hwnd);
    }
}

/// Convierte en un hilo aparte (Ghostscript puede tardar) y avisa con WM_DONE.
unsafe fn spawn_convert(hwnd: HWND, jobs: Vec<Job>, target: PathBuf, title: String, password: Option<String>, auto: bool) {
    let settings = app(|a| a.settings.clone());
    let hwnd = hwnd as isize;
    std::thread::spawn(move || {
        let inputs: Vec<PathBuf> = jobs.iter().map(|j| j.ps.clone()).collect();
        let result = logic::convert(&settings, &inputs, &target, &title, password.as_deref());
        let done = Box::into_raw(Box::new(Done { jobs, target, result, auto }));
        unsafe { PostMessageW(hwnd as HWND, WM_DONE, 0, done as isize) };
    });
}

unsafe fn on_done(hwnd: HWND, done: Done) {
    unsafe {
        let open_after = app(|a| a.settings.open_after_save);
        if !done.auto {
            app(|a| a.busy = false);
        }
        match (&done.result, done.auto) {
            (Ok(()), true) => tray(hwnd, NIM_MODIFY, Some((t().saved, &done.target.to_string_lossy(), false))),
            (Ok(()), false) => {
                let left = app(|a| {
                    a.jobs.retain(|j| !done.jobs.iter().any(|d| d.ps == j.ps));
                    a.jobs.len()
                });
                set_text(hwnd, IDC_PASSWORD, ""); // la contraseña vale para un solo PDF
                refresh_list(hwnd, &[0]);
                if left == 0 {
                    hide(hwnd);
                }
            }
            (Err(e), true) => {
                // No se pierde: pasa a la ventana para guardarlo a mano.
                for job in done.jobs.iter().cloned() {
                    add_to_list(hwnd, job);
                }
                tray(hwnd, NIM_MODIFY, Some((t().auto_failed, &error_text(e), true)));
            }
            (Err(e), false) => {
                // Los trabajos siguen en la lista: se puede reintentar.
                selection_changed(hwnd);
                message(hwnd, &format!("{}\n\n{}", t().create_failed, error_text(e)), MB_OK | MB_ICONERROR);
            }
        }
        if done.result.is_ok() && open_after {
            open(&done.target);
        }
    }
}

fn error_text(e: &ConvertError) -> String {
    match e {
        ConvertError::NoGhostscript => t().err_no_gs.to_string(),
        ConvertError::Launch(error) => fill(t().err_launch, &[error]),
        ConvertError::Failed { status, log } => fill(t().err_gs_failed, &[status, log]),
        ConvertError::CreateDir(error) => fill(t().err_mkdir, &[error]),
        ConvertError::Write { path, error } => fill(t().err_write, &[path, error]),
    }
}

unsafe fn save(hwnd: HWND) {
    unsafe {
        let sel = selected(hwnd);
        if sel.is_empty() || app(|a| a.busy) {
            return;
        }
        let typed = get_text(hwnd, IDC_NAME);
        let mut name = typed.trim();
        if name.len() > 4 && name.is_char_boundary(name.len() - 4) && name[name.len() - 4..].eq_ignore_ascii_case(".pdf") {
            name = name[..name.len() - 4].trim_end();
        }
        if !logic::is_valid_file_name(name) {
            message(hwnd, t().invalid_name, MB_OK | MB_ICONWARNING);
            return;
        }
        let folder = get_text(hwnd, IDC_FOLDER).trim().to_string();
        if !Path::new(&folder).is_absolute() {
            message(hwnd, t().invalid_folder, MB_OK | MB_ICONWARNING);
            return;
        }
        let password = get_text(hwnd, IDC_PASSWORD);
        if !logic::is_valid_password(&password) {
            message(hwnd, t().invalid_password, MB_OK | MB_ICONWARNING);
            return;
        }
        let target = Path::new(&folder).join(format!("{name}.pdf"));
        if target.exists()
            && message(hwnd, &fill(t().exists, &[&format!("{name}.pdf")]), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2) != IDYES
        {
            return;
        }
        let quality = SendMessageW(GetDlgItem(hwnd, IDC_QUALITY), CB_GETCURSEL, 0, 0).max(0) as usize;
        let open_after = IsDlgButtonChecked(hwnd, IDC_OPEN) == BST_CHECKED;
        let auto = IsDlgButtonChecked(hwnd, IDC_AUTO) == BST_CHECKED;
        let jobs = app(|a| {
            a.settings.output_folder = folder;
            a.settings.quality = logic::QUALITIES[quality.min(logic::QUALITIES.len() - 1)].to_string();
            a.settings.open_after_save = open_after;
            a.settings.auto_save = auto;
            a.settings.save();
            a.busy = true;
            sel.iter().filter_map(|&i| a.jobs.get(i).cloned()).collect::<Vec<_>>()
        });
        let name = name.to_string();
        selection_changed(hwnd); // desactiva los botones mientras convierte
        set_text(hwnd, IDC_NAME, &name);
        spawn_convert(hwnd, jobs, target, name, Some(password), false);
    }
}

unsafe fn discard(hwnd: HWND) {
    unsafe {
        let sel = selected(hwnd);
        if sel.is_empty() || app(|a| a.busy) {
            return;
        }
        let question = if sel.len() == 1 { t().discard_one.to_string() } else { fill(t().discard_many, &[&sel.len().to_string()]) };
        if message(hwnd, &question, MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2) != IDYES {
            return;
        }
        let left = app(|a| {
            for &i in sel.iter().rev() {
                if i < a.jobs.len() {
                    let _ = std::fs::remove_file(&a.jobs.remove(i).ps);
                }
            }
            a.jobs.len()
        });
        refresh_list(hwnd, &[0]);
        if left == 0 {
            hide(hwnd);
        }
    }
}

unsafe fn browse(hwnd: HWND) {
    unsafe {
        // NOTA: selector de carpetas clásico (limitado a MAX_PATH); pasar a IFileOpenDialog si se queda corto.
        let title = w(t().browse_title);
        let mut bi: BROWSEINFOW = zeroed();
        bi.hwndOwner = hwnd;
        bi.lpszTitle = title.as_ptr();
        bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
        let pidl = SHBrowseForFolderW(&bi);
        if pidl.is_null() {
            return;
        }
        let mut buf = [0u16; 260];
        if SHGetPathFromIDListW(pidl, buf.as_mut_ptr()) != 0 {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
            set_text(hwnd, IDC_FOLDER, &String::from_utf16_lossy(&buf[..len]));
        }
        CoTaskMemFree(pidl as _);
    }
}
