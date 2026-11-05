# AnomalyLauncherRust

A multithreaded Rust rewrite of `AnomalyLauncher.exe` v1.0.5 from [S.T.A.L.K.E.R. Anomaly](https://www.moddb.com/mods/stalker-anomaly).
It has the same window, settings and behaviour, but opens instantly with large modpacks.
It's a native Win32 executable, so no .NET runtime is needed.

## Install

1. Download `AnomalyLauncher.exe` from [Releases](../../releases).
2. Back up the original `AnomalyLauncher.exe` in your Anomaly folder and replace it with this one.
3. Run it as usual, directly or through Mod Organizer 2.

It uses the same `AnomalyLauncher.cfg`, `appdata\user.ltx` and `commandline.txt` as the original, so you can switch back at any time.

## Changes from the original

- **Addon check:** stops at the first addon file instead of listing all of `gamedata\`, and runs on a background thread, so the window never freezes.
- **Verify Installed Version:** hashes 2–6 files in parallel. Results are listed in the same order as the original's.

Nothing else is different.

## Building

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

You need MinGW-w64: `windres` on `PATH` on Windows, or `gcc-mingw-w64-x86-64` on Linux. Add `--features trace` to write start-up timings to `AnomalyLauncher_trace.log`.

Pushes to `main` build automatically. Bump `version` in `Cargo.toml` to publish a new release.

## Credits

S.T.A.L.K.E.R. Anomaly and the original launcher are by the Anomaly team. S.T.A.L.K.E.R. is a trademark of GSC Game World.
This project isn't affiliated with either. The launcher artwork in `assets/` belongs to its original owners.

The original launcher was decompiled and then rebuilt in Rust by [Claude](https://claude.ai), Anthropic's AI.

## License

The code is under the [MIT License](LICENSE). The artwork in `assets/` isn't covered by it.
