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

On first launch, a default `config.json` is created. The app may refresh bundled weapon icon paths from built-in defaults when older entries used remote URLs.

### Games and UI

- Profiles for **PUBG**, **Rust**, and **Counter-Strike 2** (plus sample macros) ship with the default config.
- Weapon icons are loaded from embedded files under `assets/weapons/` (no network required for icons after build).
- **Global hotkeys and low-level input hooks run on Windows only.** On macOS and Linux the UI and config work, but background hook behavior is not active the same way as on Windows.

### Troubleshooting

- If the window shows broken images, run a fresh build and ensure you start the app via **Tauri** or the packaged binary (not a raw `file://` copy of `index.html` opened alone).
- If the UI fails to load configuration, check the developer console (when running in dev mode) and that `config.json` is readable.

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

`src-tauri/tauri.conf.json` sets `frontendDist` to **`../web-root`**. That directory contains symlinks to the real UI entry and assets:

- `web-root/index.html` → `../index.html`
- `web-root/assets` → `../assets`

This limits what Tauri embeds at compile time and keeps builds faster than pointing at the whole repo. If `web-root` is missing, recreate those links (see [Tauri `frontendDist`](https://v2.tauri.app/reference/config/#frontenddist)).

### Run in development

```bash
npm run dev
```

This runs `tauri dev` (webview loads embedded assets; the Tauri IPC API is available to `index.html`).

### Production build

```bash
npm run build
```

Artifacts appear under `src-tauri/target/release/` and platform-specific bundle folders (e.g. `src-tauri/target/release/bundle/`).

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
| `assets/` | Game logos, weapon images (`assets/weapons/{pubg,rust,cs2}/`) |
| `web-root/` | Symlink bundle consumed as `frontendDist` |
| `src-tauri/` | Rust app: config, commands, macro engine, **Windows** input listener |

### License

See `src-tauri/Cargo.toml` / repository files for authorship and license.
