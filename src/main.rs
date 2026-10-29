// Cursor Ring: a thin ring that trails the Windows mouse cursor, stretches with its speed and turns gold
// over links (when Windows shows the hand cursor). A port of the cursor on zenolitee.github.io/portfolio-zen.
//
// The ring lives in a small click-through, always-on-top layered window that follows the pointer; the real
// normal arrow is swapped for the site's glowing dot while running (Windows draws it, so no lag) and put
// back on Pause / Quit / sign-out. Tray icon: Ring settings
// (settings.ini, applied live on save), Customize cursors, Pause, Start with Windows, Quit.
#![windows_subsystem = "windows"]

mod ring;

use std::{cell::Cell, mem::{size_of, zeroed}, os::windows::ffi::OsStrExt, path::{Path, PathBuf}, ptr::null_mut, time::{Duration, Instant, SystemTime}};
use windows_sys::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::DwmFlush, Gdi::*},
        System::{LibraryLoader::GetModuleHandleW, Registry::*, Threading::CreateMutexW},
        UI::{HiDpi::*, Shell::*, WindowsAndMessaging::*},
    },
};

const WM_TRAY: u32 = WM_APP + 1;
const ID_PAUSE: usize = 1;
const ID_STARTUP: usize = 2;
const ID_QUIT: usize = 3;
const ID_EDITOR: usize = 4;
const ID_SETTINGS: usize = 5;

thread_local! {
    static PAUSED: Cell<bool> = const { Cell::new(false) };
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
    static DOT: Cell<bool> = const { Cell::new(true) };      // settings.ini "dot"
}

// Swap the normal arrow for the dot. Not saved anywhere: a restart or sign-out also undoes it.
unsafe fn use_dot() { SetSystemCursor(ring::dot_cursor(SCALE.get()), OCR_NORMAL); }
// Reload the user's own cursor scheme.
unsafe fn restore_cursors() { SystemParametersInfoW(SPI_SETCURSORS, 0, null_mut(), 0); }
unsafe fn apply_cursor() { if DOT.get() && !PAUSED.get() { use_dot() } else { restore_cursors() } }

// %APPDATA%\Cursor Ring\settings.ini
fn settings_path() -> PathBuf {
    PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("Cursor Ring").join("settings.ini")
}

// Reads the settings (creating the commented default file on first run) and its modified time, for live reload.
fn load_settings() -> (ring::Config, Option<SystemTime>) {
    let path = settings_path();
    if !path.exists() {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = std::fs::write(&path, ring::DEFAULT_INI.replace('\n', "\r\n"));   // CRLF for Notepad
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    (ring::Config::parse(&text), std::fs::metadata(&path).and_then(|m| m.modified()).ok())
}

// Drawing surface: a top-down 32-bit DIB of `size` x `size`, selected into `mem`, that we write premultiplied BGRA into.
// The caller deletes the previously selected DIB once this one replaces it.
unsafe fn surface(mem: HDC, size: i32) -> (HBITMAP, &'static mut [u32]) {
    let mut bmi: BITMAPINFO = zeroed();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: size, biHeight: -size, biPlanes: 1,
        biBitCount: 32, biCompression: BI_RGB as u32, ..zeroed()
    };
    let mut bits: *mut core::ffi::c_void = null_mut();
    let dib = CreateDIBSection(mem, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    SelectObject(mem, dib);
    (dib, std::slice::from_raw_parts_mut(bits as *mut u32, (size * size) as usize))
}

fn main() {
    unsafe {
        // one copy at a time
        CreateMutexW(null_mut(), 0, w!("Local\\CursorRingSingleInstance"));
        if GetLastError() == ERROR_ALREADY_EXISTS { return; }

        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let scale = GetDpiForSystem() as f32 / 96.0;
        SCALE.set(scale);
        std::panic::set_hook(Box::new(|_| restore_cursors()));
        let (cfg, mut settings_time) = load_settings();
        DOT.set(cfg.dot);
        if cfg.dot { use_dot(); }
        let mut state = ring::State::new(scale, cfg);
        let mut boxsz = state.box_size();

        let hinst = GetModuleHandleW(null_mut());
        let class = w!("CursorRingOverlay");
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst, lpszClassName: class, ..zeroed() };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class, w!("Cursor Ring"), WS_POPUP, 0, 0, boxsz, boxsz, null_mut(), null_mut(), hinst, null_mut(),
        );

        // tray icon
        let icon = ring::tray_icon();
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = icon;
        let tip: Vec<u16> = "Cursor Ring".encode_utf16().collect();
        nid.szTip[..tip.len()].copy_from_slice(&tip);
        Shell_NotifyIconW(NIM_ADD, &nid);

        let screen = GetDC(null_mut());
        let mem = CreateCompatibleDC(screen);
        let (mut dib, mut pixels) = surface(mem, boxsz);

        let hand = LoadCursorW(null_mut(), IDC_HAND);
        let mut settings_at = Instant::now();
        let mut last = Instant::now();
        let mut topmost_at = Instant::now();
        let mut visible = false;
        let mut msg: MSG = zeroed();

        'run: loop {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT { break 'run; }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if DwmFlush() != 0 { std::thread::sleep(Duration::from_millis(8)); }   // paced to the display
            let now = Instant::now();
            let dt = (now - last).as_secs_f32().min(0.05);
            last = now;

            // settings.ini saved? apply it (checked twice a second)
            if settings_at.elapsed() >= Duration::from_millis(500) {
                settings_at = Instant::now();
                let time = std::fs::metadata(settings_path()).and_then(|m| m.modified()).ok();
                if time != settings_time {
                    let (cfg, t) = load_settings();
                    settings_time = t;
                    if cfg.dot != DOT.get() { DOT.set(cfg.dot); apply_cursor(); }
                    state.cfg = cfg;
                    if state.box_size() != boxsz {
                        boxsz = state.box_size();
                        let old = dib;
                        (dib, pixels) = surface(mem, boxsz);
                        DeleteObject(old);
                    }
                    if visible { ShowWindow(hwnd, SW_HIDE); visible = false; }   // forces a redraw below
                }
            }

            let mut ci: CURSORINFO = zeroed();
            ci.cbSize = size_of::<CURSORINFO>() as u32;
            let showing = GetCursorInfo(&mut ci) != 0 && ci.flags & CURSOR_SHOWING != 0;
            if PAUSED.get() || !showing {
                if visible { ShowWindow(hwnd, SW_HIDE); visible = false; }
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }

            let target = (ci.ptScreenPos.x as f32, ci.ptScreenPos.y as f32);
            if !state.step(target, ci.hCursor == hand, dt) && visible { continue; }   // settled: nothing to redraw

            state.draw(pixels, boxsz);
            GdiFlush();
            let pos = POINT { x: state.x.round() as i32 - boxsz / 2, y: state.y.round() as i32 - boxsz / 2 };
            let size = SIZE { cx: boxsz, cy: boxsz };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            UpdateLayeredWindow(hwnd, screen, &pos, &size, mem, &src, 0, &blend, ULW_ALPHA);
            if !visible { ShowWindow(hwnd, SW_SHOWNOACTIVATE); visible = true; }

            // other topmost windows (taskbar, menus) can push us down; reassert now and then
            if topmost_at.elapsed() >= Duration::from_secs(2) {
                SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                topmost_at = Instant::now();
            }
        }

        restore_cursors();
        Shell_NotifyIconW(NIM_DELETE, &nid);
        DestroyIcon(icon);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY if matches!(lp as u32, WM_RBUTTONUP | WM_LBUTTONUP | WM_CONTEXTMENU) => {
            show_menu(hwnd);
            0
        }
        WM_ENDSESSION if wp != 0 => { restore_cursors(); 0 }
        WM_DESTROY => { PostQuitMessage(0); 0 }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    let check = |on: bool| if on { MF_CHECKED } else { MF_UNCHECKED };
    AppendMenuW(menu, MF_STRING, ID_SETTINGS, w!("Ring settings..."));
    AppendMenuW(menu, MF_STRING, ID_EDITOR, w!("Customize cursors..."));
    AppendMenuW(menu, MF_STRING | check(PAUSED.get()), ID_PAUSE, w!("Pause"));
    AppendMenuW(menu, MF_STRING | check(startup_enabled()), ID_STARTUP, w!("Start with Windows"));
    AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
    AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quit"));
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);   // lets the menu close when you click elsewhere
    let cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY, pt.x, pt.y, 0, hwnd, null_mut());
    DestroyMenu(menu);
    match cmd as usize {
        ID_PAUSE => {
            PAUSED.set(!PAUSED.get());
            apply_cursor();
        }
        ID_STARTUP => set_startup(!startup_enabled()),
        ID_EDITOR => open_editor(),
        ID_SETTINGS => { load_settings(); open(&settings_path()); }   // load first: recreates the file if it was deleted
        ID_QUIT => { DestroyWindow(hwnd); }
        _ => {}
    }
}

// The cursor scheme editor is one HTML page baked into the exe: written to %TEMP% and opened in the default browser.
unsafe fn open_editor() {
    let path = std::env::temp_dir().join("cursor-ring-editor.html");
    if std::fs::write(&path, include_str!("../tools/editor.html")).is_ok() { open(&path); }
}

// Opens a file with its default app (browser for .html, Notepad for .ini).
unsafe fn open(path: &Path) {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    ShellExecuteW(null_mut(), w!("open"), wide.as_ptr(), null_mut(), null_mut(), SW_SHOWNORMAL);
}

const RUN_KEY: windows_sys::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_NAME: windows_sys::core::PCWSTR = w!("CursorRing");

unsafe fn startup_enabled() -> bool {
    RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME, RRF_RT_REG_SZ, null_mut(), null_mut(), null_mut()) == ERROR_SUCCESS
}

// "Start with Windows": a per-user Run entry pointing at this exe (removed again when unticked)
unsafe fn set_startup(on: bool) {
    if !on {
        RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME);
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    let value: Vec<u16> = format!("\"{}\"", exe.display()).encode_utf16().chain([0]).collect();
    RegSetKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME, REG_SZ, value.as_ptr().cast(), (value.len() * 2) as u32);
}
