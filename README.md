# MacrosManager (macro-manager)

Desktop macro and weapon-profile helper built with **Tauri 2** and a static **HTML/JS** UI. It stores per-game weapon bindings, hotkeys, and macro definitions in a local JSON config.

## For users

### Install and run

- **Release build:** install the package produced by `tauri build` for your OS (e.g. `.dmg`, `.msi`, or `.AppImage`), then open **MacrosManager** like any other app.
- **From source (developers):** see [For developers](#for-developers) and run `npm run dev`.

### Where settings are stored

Configuration lives in your OS user config directory, under a **`MacrosManager`** folder:

| OS      | Typical path |
|--------|----------------|
| macOS  | `~/Library/Application Support/MacrosManager/config.json` |
| Windows | `%APPDATA%\MacrosManager\config.json` |
| Linux  | `~/.config/MacrosManager/config.json` |

On first launch, a default `config.json` is created. The app may refresh weapon `iconUrl` values from built-in defaults when older entries used `assets/...` paths or remote URLs.

### Games and UI

- Profiles for **PUBG**, **Rust**, and **Counter-Strike 2** (plus sample macros) ship with the default config. Built-in lists **do not** include AWP, Kar98k, or UMP45; if an older `config.json` still had those entries, they are **removed on load** so the UI matches the app.
- Weapon and game-tab icons are embedded as **`data:` URLs** at **Rust compile time** (see `src-tauri/build.rs`) from `assets/` sources, so the WebView does not need to load separate image files.
- **Global hotkeys and low-level input hooks run on Windows only.** On macOS and Linux the UI and config work, but background hook behavior is not active the same way as on Windows.

### Troubleshooting

- If `npm run build` fails on Linux or WSL with **`failed to run linuxdeploy`**, AppImage tooling often needs FUSE or `APPIMAGE_EXTRACT_AND_RUN=1`. This repo’s `bundle.targets` skips **AppImage** so **`deb` / `rpm`** (and Windows/macOS targets on those hosts) still build; add `appimage` back only if you need it and can run `linuxdeploy` successfully.
- If the window shows broken images, run a fresh build and ensure you start the app via **Tauri** or the packaged binary (not a raw `file://` copy of `index.html` opened alone).
- If **`npm run build`** fails with **`failed to remove ... target/release/app.exe` / Access denied**, close **MacrosManager** if it is running (Task Manager → end **app**). The build script tries to stop **`app.exe`** automatically; if it still fails, close other handles on that file (e.g. **preview tab** of `app.exe` in the IDE, antivirus scan).

## For developers

### Prerequisites

- **Rust** toolchain (see `rust-version` in `src-tauri/Cargo.toml`, currently **1.77.2+**).
- **Node.js** and npm (for the Tauri CLI).
- On **macOS:** Xcode Command Line Tools (for compiling the native shell).

### Setup

```bash
cd macro-manager
npm install
```

### `web-root` and the frontend

`src-tauri/tauri.conf.json` sets `frontendDist` to **`../web-root`** so Tauri embeds only the UI (not the whole repo). On **Windows**, Git often checks out Unix symlinks as tiny text files (e.g. containing `../index.html`), which breaks the packaged app — the window would show that path as plain text.

**Fix:** `beforeDevCommand` / `beforeBuildCommand` run **`scripts/sync-web-root.cjs`**, which copies the real **`index.html`** and **`assets/`** from the repo root into `web-root/`. After cloning, run `npm install` then `npm run dev` or `npm run build`; you can also run `npm run sync-web-root` manually.

On macOS/Linux you can still use symlinks in `web-root/` if you prefer; the sync script overwrites those copies when you build.

### Run in development

```bash
npm run dev
```

This runs `tauri dev` (webview loads embedded assets; the Tauri IPC API is available to `index.html`).

### Production build (installers)

**`npm run build` runs only on Windows** (NSIS/MSI installers and release binary). On Linux or macOS it exits with a short message so you do not assume a Linux `.deb`/`.rpm` build from that script.

- **On Windows:** from the repo root run `npm run build`. Artifacts: `src-tauri/target/release/bundle/nsis/*.exe`, `…/msi/*.msi`, etc.
- **From Linux / macOS / WSL / CI:** use the GitHub Actions workflow below for Windows `.exe` / `.msi`, or run `cd src-tauri && cargo build --release` if you only need a **local non-bundled** binary on that OS (no Tauri bundler step).

`cargo tauri build` can still be invoked manually on non-Windows if you need bundles there; the **npm** `build` script is intentionally Windows-only.

### Windows `.exe` from Linux / macOS / WSL (recommended)

Cross-compiling Tauri for Windows from non-Windows hosts is fragile (toolchains, WebView2). The practical approach is **CI on a Windows runner**:

1. Push this repository to **GitHub** (or fork it).
2. Open **Actions** → workflow **“Build Windows installers”**.
3. Choose **Run workflow** (the workflow is manual-only so it does not spend CI minutes on every push).
4. When it finishes, download the artifacts **MacrosManager-windows-nsis** (setup `.exe`) and/or **MacrosManager-windows-msi** (`.msi`).

You need a Windows machine only in the cloud; your local OS can stay Linux, WSL, or macOS.

### Rust-only checks

```bash
cd src-tauri
cargo check
```

If `cargo` errors around `generate_context!` or missing `.rlib` after interrupted builds, try `cargo clean` and build again.

### Project layout (short)

| Path | Role |
|------|------|
| `index.html` | Main UI, invokes Tauri commands |
| `assets/` | Source images for weapons and game logos (read by **`build.rs`** at compile time) |
| `src-tauri/build.rs` | Encodes those files as `data:` URLs into the Rust binary (weapon + tab icons) |
| `web-root/` | Copy of `index.html` + `assets/` for `frontendDist` (filled by `scripts/sync-web-root.cjs`) |
| `src-tauri/` | Rust app: config, commands, macro engine, **Windows** input listener |

### License

See `src-tauri/Cargo.toml` / repository files for authorship and license.
