Multithreaded Rust rebuild of the S.T.A.L.K.E.R. Anomaly launcher (v1.0.5) with the same window, settings and behaviour.

- Opens instantly with large modpacks (e.g. GAMMA through Mod Organizer 2): the addon check no longer lists all of `gamedata` and no longer runs on the window's thread.
- "Verify Installed Version" hashes several files in parallel.
- Native Win32 executable, no .NET runtime needed.

**Install:** back up your current `AnomalyLauncher.exe` in the Anomaly folder, then replace it with this one. Mod Organizer 2 (e.g. GAMMA) picks it up as usual.
