//! Anomaly Launcher — 1:1 Rust port of AnomalyLauncher.exe v1.0.5.
//! Place the built exe in the Anomaly root folder (next to bin\, appdata\, gamedata\).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod default_user_ltx;
mod logic;
mod trace;
#[cfg(windows)]
mod gdip;
#[cfg(windows)]
mod ui;

#[cfg(windows)]
fn has_avx_support() -> bool {
    // Same check as the original: XSTATE_MASK_AVX (bit 2) enabled by the OS.
    #[link(name = "kernel32")]
    extern "system" {
        fn GetEnabledXStateFeatures() -> u64;
    }
    unsafe { GetEnabledXStateFeatures() & 4 != 0 }
}

#[cfg(windows)]
fn main() {
    use std::path::{Path, PathBuf};
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let _ = std::env::set_current_dir(&dir);
    let _ = std::fs::create_dir_all(logic::APPDATA);
    let _ = std::fs::create_dir_all(logic::GAMEDATA);

    trace::init();
    trace::log("launcher started");
    let has_avx = has_avx_support();
    let (start, mut delete_cache) = ui::run(has_avx);
    trace::log(&format!("window closed, start game: {}", start));
    if !start {
        return;
    }
    if logic::merge_user_ltx_update() {
        delete_cache = true;
    }
    if delete_cache && Path::new(logic::SHADERS_CACHE).is_dir() {
        let _ = std::fs::remove_dir_all(logic::SHADERS_CACHE);
    }

    let cfg = logic::read_lines(logic::CFG_FILE);
    let game = dir.join(logic::BIN).join(logic::resolve_game_exe(&cfg, has_avx));
    let args: Vec<String> = std::env::args().skip(1).collect();
    match std::process::Command::new(&game).args(&args).current_dir(&dir).spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(_) => {
            let body: Vec<u16> = format!("Could not start {}", game.display())
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let cap: Vec<u16> = "ERROR".encode_utf16().chain(Some(0)).collect();
            unsafe { MessageBoxW(std::ptr::null_mut(), body.as_ptr(), cap.as_ptr(), MB_OK | MB_ICONERROR) };
        }
    }
}

#[cfg(not(windows))]
fn main() {
    let _ = (default_user_ltx::LINES, logic::CFG_FILE);
    eprintln!("AnomalyLauncher is a Windows program; build with --target x86_64-pc-windows-gnu.");
}
