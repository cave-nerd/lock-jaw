<div align="center">

# 🔒 Lock Jaw

**Fast, local-first markdown notes — built with Rust**

[![Build](https://github.com/cave-nerd/lock-jaw/actions/workflows/release.yml/badge.svg)](https://github.com/cave-nerd/lock-jaw/actions)
[![Latest Release](https://img.shields.io/github/v/release/cave-nerd/lock-jaw?color=e94560&label=download)](https://github.com/cave-nerd/lock-jaw/releases/latest)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-orange.svg)](https://www.rust-lang.org)

[**Download**](#-installation) · [**Features**](#-features) · [**Configuration**](#-configuration) · [**Plugins**](#-plugins)

</div>

---

> Lock Jaw is a **local-first** markdown note editor that keeps your notes as plain `.md` files on your own machine — no cloud account required, no subscription, no lock-in. It is lightweight, keyboard-friendly, and themeable.

---

## ✨ Features

| | |
|---|---|
| **Split editor / preview** | Edit raw markdown on the left, see rendered output on the right — live |
| **Vault-based** | Open any folder as your vault; notes are plain `.md` files |
| **Drag & drop** | Reorganise notes and folders by dragging them in the sidebar |
| **Multiple themes** | Dark, Light, Solarized, Dracula, Nord, Gruvbox, One Dark, and more |
| **WebDAV sync** | Optional one-click sync to Nextcloud, ownCloud, or any WebDAV server |
| **Plugin system** | Extend Lock Jaw with WASM plugins — no restarts needed |
| **Command palette** | `Ctrl+P` to jump between notes instantly |
| **Zero telemetry** | No analytics, no network calls except your optional WebDAV server |

---

## 🖥️ Interface

```
┌──────────────┬──────────────────────────┬──────────────────────────┐
│   Sidebar    │   Editor (raw markdown)  │   Preview (rendered)     │
│              │                          │                          │
│  📁 Notes    │  # My Note               │  My Note                 │
│   note-1.md  │                          │  ────────                │
│   note-2.md  │  Some **bold** text and  │  Some bold text and      │
│  📁 Archive  │  a [link](url).          │  a link.                 │
│   old.md     │                          │                          │
│              │  ```rust                 │  ┌──────────────────┐    │
│  [ Search ]  │  fn main() {}            │  │ fn main() {}     │    │
│              │  ```                     │  └──────────────────┘    │
└──────────────┴──────────────────────────┴──────────────────────────┘
│  ~/notes/my-note.md  |  142 words  |  Saved                        │
└───────────────────────────────────────────────────────────────────┘
```

---

## 📦 Installation

### AppImage (Linux — recommended)

Download the latest AppImage from the [releases page](https://github.com/cave-nerd/lock-jaw/releases/latest), then:

```bash
chmod +x LockJaw-*.AppImage
./LockJaw-*.AppImage
```

No installation required. Works on any modern x86-64 Linux distribution.

### Flatpak

```bash
# Build from source (requires flatpak-builder)
git clone https://github.com/cave-nerd/lock-jaw.git
cd lock-jaw
./build-flatpak.sh --install

# Run
flatpak run io.github.cave_nerd.LockJaw
```

### Build from Source

**Prerequisites:**

```bash
# Ubuntu / Debian
sudo apt-get install \
  build-essential pkg-config \
  libssl-dev libgl-dev libfontconfig-dev \
  libwayland-dev libxkbcommon-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxrandr-dev libxi-dev libudev-dev libdbus-1-dev
```

**Build:**

```bash
git clone https://github.com/cave-nerd/lock-jaw.git
cd lock-jaw
cargo build --release -p lj-ui
./target/release/lockjaw
```

---

## ⚙️ Configuration

Lock Jaw stores its config at `~/.config/lockjaw/config.toml`.

```toml
vault_path = "/home/you/notes"   # folder where your .md files live
theme      = "dark"              # dark | light | solarized | dracula | nord | gruvbox | onedark
font_size  = 14.0

[webdav]
enabled  = true
url      = "https://cloud.example.com/dav/notes/"
username = "your-username"
# password is stored in your OS keyring — never written to this file
```

> **Security note:** The WebDAV password is stored in your OS keyring (GNOME Keyring / KWallet on Linux, Keychain on macOS) and is never written to disk.

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` | Save current note |
| `Ctrl+P` | Open command palette |
| `Ctrl+N` | New note |
| `Ctrl+,` | Open settings |

---

## 🔌 Plugins

Lock Jaw supports **WASM plugins** that can extend the editor without requiring a restart.

Plugins live in `~/.config/lockjaw/plugins/`. Each plugin is a directory containing a `.wasm` file and a `plugin.toml` manifest:

```toml
# plugin.toml
name    = "my-plugin"
version = "0.1.0"
author  = "You"
description = "Does something useful"
```

Manage plugins via **Addons** in the menu bar — enable, disable, reload, or remove plugins without leaving the app.

---

## 🎨 Themes

Lock Jaw ships with 8 built-in themes:

| Theme | Style |
|-------|-------|
| **Dark** | Deep navy with red accent — the default |
| **Light** | Clean white for bright environments |
| **Solarized Dark / Light** | The classic Ethan Schoonover palette |
| **Dracula** | Purple-tinted dark theme |
| **Nord** | Arctic, north-bluish |
| **Gruvbox** | Warm retro tones |
| **One Dark** | Atom-inspired dark theme |

Custom themes can be added as `.toml` files in `~/.config/lockjaw/themes/` (coming soon).

---

## 🔄 WebDAV Sync

Lock Jaw can sync your vault with any WebDAV server:

1. Open **Settings → Sync**
2. Enter your server URL, username, and password
3. Click **Sync Now** or enable automatic sync

Compatible with **Nextcloud**, **ownCloud**, **Hetzner Storage Box**, and any standard WebDAV endpoint. Local always wins on conflict.

---

## 🤝 Contributing

Contributions are welcome! Please open an issue before submitting a large pull request.

```bash
git clone https://github.com/cave-nerd/lock-jaw.git
cd lock-jaw
cargo build --workspace
cargo run -p lj-ui
```

The workspace is split into four crates:

| Crate | Purpose |
|-------|---------|
| `lj-core` | Vault, note I/O, config, WebDAV sync |
| `lj-md` | Markdown parsing and egui rendering |
| `lj-plugin` | WASM plugin host |
| `lj-ui` | egui/eframe application and all UI code |

---

## 📄 License

GPL-3.0 © [cave-nerd](https://github.com/cave-nerd)
