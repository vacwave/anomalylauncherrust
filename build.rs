// Embeds the application icon and version info into the Windows executable.
use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/Stalker.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("app_res.o");
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "gnu" {
        // MinGW: windres produces a COFF object we link directly.
        let windres = env::var("WINDRES").unwrap_or_else(|_| {
            if cfg!(windows) { "windres".into() } else { "x86_64-w64-mingw32-windres".into() }
        });
        let st = Command::new(windres)
            .args(["--include-dir", "assets", "assets/app.rc", "-O", "coff", "-o"])
            .arg(&out)
            .status()
            .expect("failed to run windres");
        assert!(st.success(), "windres failed");
        println!("cargo:rustc-link-arg={}", out.display());
    } else {
        // MSVC: rc.exe produces a .res file that link.exe accepts directly.
        let res = out.with_extension("res");
        let st = Command::new("rc.exe")
            .args(["/nologo", "/fo"])
            .arg(&res)
            .arg("assets/app.rc")
            .status()
            .expect("failed to run rc.exe (run from a VS developer prompt, or use the gnu toolchain)");
        assert!(st.success(), "rc.exe failed");
        println!("cargo:rustc-link-arg={}", res.display());
    }
}
