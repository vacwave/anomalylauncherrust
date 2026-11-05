//! Launcher behaviour ported 1:1 from the original C# (Program.cs / SelectionUI.cs).
//! Everything here is plain std so it can be unit-tested on any OS.

use md5::{Digest, Md5};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

pub const CFG_FILE: &str = "AnomalyLauncher.cfg";
pub const APPDATA: &str = "appdata";
pub const BIN: &str = "bin";
pub const GAMEDATA: &str = "gamedata";
pub const CHECKSUMS_MD5: &str = "tools\\checksums.md5";
pub const USER_LTX: &str = "appdata\\user.ltx";
pub const USER_LTX_OLD: &str = "appdata\\user.ltx.old";
pub const USER_LTX_UPDATE: &str = "appdata\\user.ltx.update";
pub const SHADERS_CACHE: &str = "appdata\\shaders_cache";
pub const ALSOFT_INI: &str = "bin\\alsoft.ini";
pub const ALSOFT_INI_BAK: &str = "bin\\alsoft.ini.bak";
pub const COMMANDLINE_TXT: &str = "commandline.txt";

const ALLOWED_FILES: [&str; 6] = [
    "configs\\atmosfear_default_settings.ltx",
    "configs\\atmosfear_options.ltx",
    "configs\\axr_options.ltx",
    "configs\\cache_dbg.ltx",
    "configs\\localization.ltx",
    "configs\\warfare_options.ltx",
];

const NL: &str = "\r\n";

/// Renderers offered in the "Renderer" box, top to bottom.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Renderer {
    Dx8,
    Dx9,
    Dx10,
    Dx11,
}

impl Renderer {
    pub const ALL: [Renderer; 4] = [Self::Dx8, Self::Dx9, Self::Dx10, Self::Dx11];

    /// Token stored on line 1 of AnomalyLauncher.cfg.
    pub fn cfg_token(self) -> &'static str {
        match self {
            Self::Dx8 => "DX8",
            Self::Dx9 => "DX9",
            Self::Dx10 => "DX10",
            Self::Dx11 => "DX11",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Dx8 => "DirectX 8 (R1)",
            Self::Dx9 => "DirectX 9 (R2)",
            Self::Dx10 => "DirectX 10 (R3)",
            Self::Dx11 => "DirectX 11 (R4)",
        }
    }
    /// Executable base name in bin\ (AVX suffix and .exe are appended).
    pub fn exe_base(self) -> &'static str {
        match self {
            Self::Dx8 => "AnomalyDX8",
            Self::Dx9 => "AnomalyDX9",
            Self::Dx10 => "AnomalyDX10",
            Self::Dx11 => "AnomalyDX11",
        }
    }
    pub fn from_token(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.cfg_token() == s)
    }
}

pub const WINDOW_MODES: [&str; 3] = ["Borderless Windowed", "Fullscreen", "Windowed"];
pub const SHADOW_MAPS: [&str; 5] = [
    "Shadow Map 1536",
    "Shadow Map 2048",
    "Shadow Map 2560",
    "Shadow Map 3072",
    "Shadow Map 4096",
];

/// State of every setting shown in the launcher window.
#[derive(Clone, Debug)]
pub struct Settings {
    pub renderer: Option<Renderer>,
    pub avx: bool,
    pub avx_enabled: bool,
    pub window_mode: usize,
    pub res_width: String,
    pub res_height: String,
    pub debug: bool,
    pub shadow_map: usize,
    pub sound_fix: bool,
    pub prefetch_sounds: bool,
    pub force_gfx: bool,
    pub reset_user_ltx: bool,
    pub delete_shader_cache: bool,
}

pub fn read_lines(path: &str) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(s) => s.lines().map(|l| l.to_string()).collect(),
        Err(_) => Vec::new(),
    }
}

/// C# int.Parse tolerates surrounding whitespace.
fn parse_int(s: &str) -> Option<i32> {
    s.trim().parse::<i32>().ok()
}

/// SelectionUI constructor defaults + ReadConfig().
pub fn load_settings(has_avx: bool, screen_w: i32, screen_h: i32) -> Settings {
    let a = read_lines(CFG_FILE);
    let mut s = Settings {
        renderer: Some(Renderer::Dx11), // designer default
        avx: true,
        avx_enabled: true,
        window_mode: 0,
        res_width: String::new(),
        res_height: String::new(),
        debug: false,
        shadow_map: 1,
        sound_fix: false,
        prefetch_sounds: false,
        force_gfx: true,
        reset_user_ltx: false,
        delete_shader_cache: false,
    };
    if !a.is_empty() {
        // Unknown token leaves no radio checked, like the original.
        s.renderer = Renderer::from_token(&a[0]);
    }
    if a.len() > 1 {
        s.avx = a[1] == "AVX";
    }
    s.window_mode = match a.get(2) {
        None => 0,
        Some(v) => match parse_int(v) {
            Some(n) if (0..=2).contains(&n) => n as usize,
            _ => 0,
        },
    };
    let (mut w, mut h) = (screen_w, screen_h);
    if a.len() > 4 {
        if let (Some(mut rw), Some(mut rh)) = (parse_int(&a[3]), parse_int(&a[4])) {
            if !(0..=4096).contains(&rw) {
                rw = 1920;
            }
            if !(0..=4096).contains(&rh) {
                rh = 1080;
            }
            w = rw;
            h = rh;
        }
    }
    s.res_width = w.to_string();
    s.res_height = h.to_string();
    if a.len() > 5 {
        s.debug = a[5] == "DBG";
    }
    s.shadow_map = match a.get(6) {
        None => 1,
        Some(v) => match parse_int(v) {
            Some(n) if (0..=4).contains(&n) => n as usize,
            _ => 1,
        },
    };
    if a.len() > 7 {
        s.sound_fix = a[7] == "SNDFIX";
    }
    if a.len() > 8 {
        s.prefetch_sounds = a[8] == "SNDPREFETCH";
    }
    if !has_avx {
        s.avx = false;
        s.avx_enabled = false;
    }
    s.force_gfx = !Path::new(USER_LTX).exists();
    s
}

/// WriteConfig(): returns false when the resolution is invalid.
pub fn save_settings(s: &Settings) -> bool {
    let mut ok = true;
    let mut a: Vec<String> = vec![String::new(); 9];
    a[0] = s.renderer.unwrap_or(Renderer::Dx9).cfg_token().into();
    a[1] = if s.avx { "AVX" } else { "NOAVX" }.into();
    a[2] = s.window_mode.to_string();
    match (parse_int(&s.res_width), parse_int(&s.res_height)) {
        (Some(w), Some(h)) => {
            if !(0..=4096).contains(&w) || !(0..=4096).contains(&h) {
                ok = false;
            }
            a[3] = w.to_string();
            a[4] = h.to_string();
        }
        _ => {
            a[3] = "-1".into();
            a[4] = "-1".into();
            ok = false;
        }
    }
    a[5] = if s.debug { "DBG" } else { "NODBG" }.into();
    a[6] = s.shadow_map.to_string();
    a[7] = if s.sound_fix { "SNDFIX" } else { "NOSNDFIX" }.into();
    a[8] = if s.prefetch_sounds { "SNDPREFETCH" } else { "NOSNDPREFETCH" }.into();
    let mut out = a.join(NL);
    out.push_str(NL);
    let _ = fs::write(CFG_FILE, out);
    ok
}

fn write_all_lines(path: &str, lines: &[&str]) -> std::io::Result<()> {
    let mut out = String::new();
    for l in lines {
        out.push_str(l);
        out.push_str(NL);
    }
    fs::write(path, out)
}

pub fn reset_user_ltx() -> std::io::Result<()> {
    if Path::new(USER_LTX).exists() {
        if Path::new(USER_LTX_OLD).exists() {
            fs::remove_file(USER_LTX_OLD)?;
        }
        fs::rename(USER_LTX, USER_LTX_OLD)?;
    }
    fs::create_dir_all(APPDATA)?;
    write_all_lines(USER_LTX, crate::default_user_ltx::LINES)
}

pub fn write_to_user_ltx(s: &Settings) -> std::io::Result<()> {
    let existing = fs::read_to_string(USER_LTX).unwrap_or_default();
    let w = parse_int(&s.res_width).unwrap_or(0);
    let h = parse_int(&s.res_height).unwrap_or(0);
    let mut sb = String::new();
    let mut line = |t: &str| {
        sb.push_str(t);
        sb.push_str(NL);
    };
    line(&existing);
    line(&format!("vid_mode {}x{}", w, h));
    match s.window_mode {
        1 => {
            line("rs_borderless 0");
            line("rs_fullscreen on");
            line("rs_screenmode fullscreen");
        }
        2 => {
            line("rs_borderless 0");
            line("rs_fullscreen off");
            line("rs_screenmode windowed");
        }
        _ => {
            line("rs_borderless 1");
            line("rs_fullscreen off");
            line("rs_screenmode borderless");
        }
    }
    fs::create_dir_all(APPDATA)?;
    fs::write(USER_LTX, sb)
}

pub fn write_command_line(s: &Settings) -> std::io::Result<()> {
    let mut sb = String::new();
    let smap = match s.shadow_map {
        0 => "-smap1536",
        2 => "-smap2560",
        3 => "-smap3072",
        4 => "-smap4096",
        _ => "-smap2048",
    };
    sb.push_str(smap);
    sb.push_str(NL);
    if s.debug {
        sb.push_str("-dbg");
        sb.push_str(NL);
    }
    if s.prefetch_sounds {
        sb.push_str("-prefetch_sounds");
        sb.push_str(NL);
    }
    fs::write(COMMANDLINE_TXT, sb)
}

pub fn apply_sound_fix(s: &Settings) -> std::io::Result<()> {
    if s.sound_fix {
        if Path::new(ALSOFT_INI).exists() {
            if Path::new(ALSOFT_INI_BAK).exists() {
                fs::remove_file(ALSOFT_INI_BAK)?;
            }
            fs::rename(ALSOFT_INI, ALSOFT_INI_BAK)?;
        }
    } else if !Path::new(ALSOFT_INI).exists() && Path::new(ALSOFT_INI_BAK).exists() {
        fs::rename(ALSOFT_INI_BAK, ALSOFT_INI)?;
    }
    Ok(())
}

/// Play button: Err(msg) means "Invalid screen resolution".
pub fn on_play(s: &Settings) -> Result<(), String> {
    if !save_settings(s) {
        return Err("Invalid screen resolution".into());
    }
    let ltx_exists = Path::new(USER_LTX).exists();
    if s.reset_user_ltx || !ltx_exists {
        reset_user_ltx().map_err(|e| e.to_string())?;
    }
    if s.force_gfx || s.reset_user_ltx || !Path::new(USER_LTX).exists() {
        write_to_user_ltx(s).map_err(|e| e.to_string())?;
    }
    write_command_line(s).map_err(|e| e.to_string())?;
    apply_sound_fix(s).map_err(|e| e.to_string())?;
    Ok(())
}

/// Program.Main after the window closes: picks bin\<exe> from the saved config.
pub fn resolve_game_exe(cfg: &[String], has_avx: bool) -> String {
    let mut name = String::from("AnomalyDX9");
    if let Some(first) = cfg.first() {
        if let Some(r) = Renderer::from_token(first) {
            name = r.exe_base().into();
        }
    }
    if has_avx && (cfg.len() < 2 || cfg[1] == "AVX") {
        name.push_str("AVX");
    }
    name.push_str(".exe");
    name
}

/// Merges appdata\user.ltx.update into user.ltx; returns true if it did (forces shader cache wipe).
pub fn merge_user_ltx_update() -> bool {
    if Path::new(APPDATA).is_dir() && Path::new(USER_LTX_UPDATE).exists() {
        let a = fs::read_to_string(USER_LTX).unwrap_or_default();
        let b = fs::read_to_string(USER_LTX_UPDATE).unwrap_or_default();
        let _ = fs::write(USER_LTX, format!("{}\n{}", a, b));
        let _ = fs::remove_file(USER_LTX_UPDATE);
        return true;
    }
    false
}

/// Finds the first file under `dir` that isn't one of the stock config files.
/// Returns as soon as one is found, so it never lists the whole tree. With a
/// large modpack (e.g. GAMMA through MO2's virtual gamedata) the original
/// enumerated every file before checking any of them.
fn find_addon_file(dir: &str, visited: &mut usize) -> Option<String> {
    let rd = fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for e in rd.flatten() {
        *visited += 1;
        let sep = std::path::MAIN_SEPARATOR_STR;
        let p = format!("{}{}{}", dir, sep, e.file_name().to_string_lossy());
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            subdirs.push(p);
        } else if !ALLOWED_FILES
            .iter()
            .any(|a| format!("{}{}{}", GAMEDATA, sep, a.replace('\\', sep)) == p)
        {
            return Some(p);
        }
    }
    // Only files count (an empty folder isn't an addon), so descend lazily.
    subdirs.into_iter().find_map(|d| find_addon_file(&d, visited))
}

/// Program.CheckForAddons: Some(path) of the first non-stock file in gamedata\.
pub fn find_addon() -> Option<String> {
    if !Path::new(GAMEDATA).is_dir() {
        return None;
    }
    let mut visited = 0;
    let r = find_addon_file(GAMEDATA, &mut visited);
    crate::trace::log(&format!("addon scan visited {} entries, found {:?}", visited, r));
    r
}

pub struct VerifyResult {
    pub message: String,
    pub corrupt: Vec<String>,
    pub missing: Vec<String>,
}

struct Entry {
    expected: String,
    file: String,
}

fn parse_checksums() -> Vec<Entry> {
    let mut out = Vec::new();
    for line in read_lines(CHECKSUMS_MD5) {
        let line = line.trim_start_matches(' ');
        let Some((hash, rest)) = line.split_once(' ') else { continue };
        let rest = rest.trim_start_matches(' ');
        if rest.is_empty() || !rest.starts_with('*') || hash.starts_with('#') || hash.starts_with(';') {
            continue;
        }
        out.push(Entry { expected: hash.to_ascii_lowercase(), file: rest[1..].to_string() });
    }
    out
}

enum Outcome {
    Ok,
    Missing,
    Corrupt,
}

fn hash_file(path: &str, buf: &mut [u8], done_bytes: &AtomicU64) -> Option<String> {
    let mut f = fs::File::open(path).ok()?;
    let mut hasher = Md5::new();
    loop {
        let n = match f.read(buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Some(String::new()), // unreadable counts as corrupt
        };
        hasher.update(&buf[..n]);
        done_bytes.fetch_add(n as u64, Ordering::Relaxed);
    }
    Some(hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}

/// Number of files hashed at once. Several readers help on SSDs; on a spinning
/// disk too many readers seek-thrash, so this stays modest.
fn verify_threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 6)
}

/// Program.VerifyInstall: addon check + MD5 check against tools\checksums.md5.
/// Files are hashed on several threads; results keep the checksum-file order.
pub fn verify_install(status: &(dyn Fn(String) + Sync)) -> VerifyResult {
    let t0 = std::time::Instant::now();
    let mut text = String::new();
    let add = |text: &mut String, msg: &str| {
        if !text.is_empty() {
            text.push_str(" | ");
        }
        text.push_str(msg);
    };
    status("Checking for addons".into());
    if let Some(f) = find_addon() {
        status(format!("Detected {}", f));
        add(&mut text, "* installed addons can cause problems or crashes *");
    }
    let mut missing = Vec::new();
    let mut corrupt = Vec::new();
    let checksums_missing = !Path::new(CHECKSUMS_MD5).exists();
    if !checksums_missing {
        status("Reading checksums".into());
        let entries = parse_checksums();
        let total_bytes: u64 = entries
            .iter()
            .map(|e| fs::metadata(&e.file).map(|m| m.len()).unwrap_or(0))
            .sum();
        let next = AtomicUsize::new(0);
        let files_done = AtomicUsize::new(0);
        let done_bytes = AtomicU64::new(0);
        let finished = AtomicBool::new(false);
        let results: Vec<Mutex<Option<Outcome>>> = entries.iter().map(|_| Mutex::new(None)).collect();
        std::thread::scope(|s| {
            for _ in 0..verify_threads().min(entries.len().max(1)) {
                s.spawn(|| {
                    let mut buf = vec![0u8; 1 << 20];
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(e) = entries.get(i) else { break };
                        let outcome = match hash_file(&e.file, &mut buf, &done_bytes) {
                            None => Outcome::Missing,
                            Some(h) if h == e.expected => Outcome::Ok,
                            Some(_) => Outcome::Corrupt,
                        };
                        *results[i].lock().unwrap() = Some(outcome);
                        files_done.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
            // Progress reporter: one status update every 100 ms, not one per chunk.
            s.spawn(|| {
                while !finished.load(Ordering::Relaxed) {
                    let done = done_bytes.load(Ordering::Relaxed);
                    let pct = if total_bytes > 0 { (done * 100 / total_bytes).min(100) } else { 0 };
                    status(format!(
                        "Verifying install | {}% | {} of {} files",
                        pct,
                        files_done.load(Ordering::Relaxed),
                        entries.len()
                    ));
                    if files_done.load(Ordering::Relaxed) >= entries.len() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            });
        });
        finished.store(true, Ordering::Relaxed);
        for (e, r) in entries.iter().zip(results) {
            match r.into_inner().unwrap() {
                Some(Outcome::Missing) => missing.push(e.file.clone()),
                Some(Outcome::Corrupt) => corrupt.push(e.file.clone()),
                _ => {}
            }
        }
        crate::trace::log(&format!(
            "verify: {} files, {} MB, {} threads, {:.2?}",
            entries.len(),
            total_bytes >> 20,
            verify_threads(),
            t0.elapsed()
        ));
    }
    if checksums_missing {
        add(&mut text, &format!("* {} missing *", CHECKSUMS_MD5));
    }
    if !corrupt.is_empty() {
        add(&mut text, "* checksum verification failed, please reinstall *");
    }
    if !missing.is_empty() {
        add(&mut text, "* files missing, please reinstall *");
    }
    VerifyResult { message: text, corrupt, missing }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests change the process working directory, so they must not overlap.
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn exe_names() {
        let cfg = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(resolve_game_exe(&cfg(&["DX11", "NOAVX"]), true), "AnomalyDX11.exe");
        assert_eq!(resolve_game_exe(&cfg(&["DX10", "AVX"]), false), "AnomalyDX10.exe");
        assert_eq!(resolve_game_exe(&cfg(&["DX11", "AVX"]), true), "AnomalyDX11AVX.exe");
        assert_eq!(resolve_game_exe(&cfg(&["DX8"]), true), "AnomalyDX8AVX.exe");
        assert_eq!(resolve_game_exe(&[], false), "AnomalyDX9.exe");
        assert_eq!(resolve_game_exe(&cfg(&["junk", "NOAVX"]), true), "AnomalyDX9.exe");
    }

    #[test]
    fn roundtrip_cfg() {
        let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("al_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let mut s = load_settings(true, 2560, 1440);
        assert_eq!(s.renderer, Some(Renderer::Dx11));
        assert_eq!(s.res_width, "2560");
        s.renderer = Some(Renderer::Dx10);
        s.shadow_map = 4;
        s.window_mode = 2;
        assert!(save_settings(&s));
        let raw = fs::read_to_string(CFG_FILE).unwrap();
        assert_eq!(raw, "DX10\r\nAVX\r\n2\r\n2560\r\n1440\r\nNODBG\r\n4\r\nNOSNDFIX\r\nNOSNDPREFETCH\r\n");
        let t = load_settings(true, 1, 1);
        assert_eq!(t.renderer, Some(Renderer::Dx10));
        assert_eq!(t.shadow_map, 4);
        assert_eq!(t.window_mode, 2);
        assert_eq!(resolve_game_exe(&read_lines(CFG_FILE), true), "AnomalyDX10AVX.exe");
        let n = load_settings(false, 1, 1);
        assert!(!n.avx && !n.avx_enabled);
        s.res_width = "5000".into();
        assert!(!save_settings(&s));
        s.res_width = "abc".into();
        assert!(!save_settings(&s));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn addon_scan_stops_early() {
        let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("al_scan_{}", std::process::id()));
        let cfgs = dir.join("gamedata").join("configs");
        fs::create_dir_all(&cfgs).unwrap();
        std::env::set_current_dir(&dir).unwrap();
        for f in ["axr_options.ltx", "localization.ltx"] {
            fs::write(cfgs.join(f), "x").unwrap();
        }
        fs::create_dir_all(dir.join("gamedata").join("empty").join("deeper")).unwrap();
        assert_eq!(find_addon(), None, "stock files and empty folders are not addons");
        let deep = dir.join("gamedata").join("scripts").join("mod");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("a.script"), "x").unwrap();
        assert!(find_addon().unwrap().ends_with("a.script"));
        fs::remove_dir_all(&dir).ok();
    }

    /// Simulates a large modpack: the old scan listed every file first; the new one
    /// stops at the first non-stock file. Run with: cargo test --release -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bench_addon_scan() {
        let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("al_bench_{}", std::process::id()));
        let gd = dir.join("gamedata");
        let mut n = 0;
        for a in 0..40 {
            for b in 0..25 {
                let d = gd.join(format!("mod{a}")).join(format!("sub{b}"));
                fs::create_dir_all(&d).unwrap();
                for c in 0..200 {
                    fs::write(d.join(format!("f{c}.dds")), "").unwrap();
                    n += 1;
                }
            }
        }
        std::env::set_current_dir(&dir).unwrap();
        fn walk_all(p: &Path, out: &mut usize) {
            for e in fs::read_dir(p).unwrap().flatten() {
                if e.file_type().unwrap().is_dir() { walk_all(&e.path(), out) } else { *out += 1 }
            }
        }
        let t = std::time::Instant::now();
        let mut all = 0;
        walk_all(&gd, &mut all);
        let old = t.elapsed();
        let t = std::time::Instant::now();
        assert!(find_addon().is_some());
        let new = t.elapsed();
        println!("{n} files: full listing (original) {old:.2?}, early-exit scan {new:.2?}");
        fs::remove_dir_all(&dir).ok();
    }
}
