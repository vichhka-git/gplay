<div align="center">

# gplay-cli

[![Crates.io](https://img.shields.io/crates/v/gplay-cli.svg)](https://crates.io/crates/gplay-cli)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)](https://github.com)
</div>

A fast, standalone command-line tool and interactive wizard written in Rust to **search**, **download**, and **install** Android applications directly from the Google Play Store—**without needing a personal Google account**.


---

## ✨ Features

- ⚡ **Zero-Login Required**: Anonymous session negotiation via public Aurora token dispenser with silent auto-refresh.
- 🔒 **100% Authentic Signatures**: Downloads official Google Play binaries with original cryptographic developer signatures.
- 📜 **Smart Version Resolution**: Download by **version name** (e.g. `8.22.2`), **version code** (e.g. `173301`), or `latest` with automatic version suggestion fallback.
- 📦 **Standard `.apks` Bundle Export**: Automatically packages Split APKs into `.apks` bundles saved directly in your current directory.
- 📲 **Optional One-Step ADB Installation**: Seamlessly stream and install APKs or split bundles to connected devices with `gplay-cli install`.
- 🎨 **Interactive TUI**: Easy-to-use terminal wizard for quick searching, browsing versions, and downloading.

---

## 🚀 Installation

### Via Cargo (Recommended)

```bash
cargo install gplay-cli
```

### From Source

Ensure you have [Rust & Cargo](https://rustup.rs/) installed:

```bash
# Clone the repository
git clone https://github.com/vichhka-git/gplay-cli.git
cd gplay-cli

# Build optimized release binary
cargo build --release

# Install binary to Cargo bin path (~/.cargo/bin)
cargo install --path .
```

> **Note on ADB**: ADB is **completely optional**. It is only required if you use direct device installation (`gplay-cli install`). Searching, version lookup, and downloading work standalone without ADB or Android SDK.

---

## 🛠️ Usage

> **Tip**: You can use either `gplay-cli` or `gplay` as your terminal command.

### Interactive Wizard (Default)

Launch the interactive prompt by running without arguments:

```bash
gplay-cli
```

---

### Command Line Interface

#### 🔍 Search Apps
```bash
gplay-cli search telegram
gplay-cli search "signal private messenger" --limit 10
```

#### ℹ️ App Details
```bash
gplay-cli info org.thoughtcrime.securesms
```

#### 📜 List Historical Versions
```bash
gplay-cli versions org.thoughtcrime.securesms
```

#### ⬇️ Download App
Files are automatically saved to your current working directory (or custom path with `-o`):
```bash
# Download latest version (saves to ./<pkg>_<ver>.apk or .apks)
gplay-cli download org.thoughtcrime.securesms

# Download specific version by Version Name
gplay-cli download org.thoughtcrime.securesms 8.22.2

# Download specific version by Version Code
gplay-cli download org.thoughtcrime.securesms 173301

# Download to a specific output folder
gplay-cli download org.thoughtcrime.securesms 8.22.2 -o ./my_folder
```

#### 📲 Direct Install via ADB
Download from Google Play or pass a local file to install directly onto a connected Android device:
```bash
# 1. Download from Google Play and install in one step:
gplay-cli install org.thoughtcrime.securesms
gplay-cli install org.thoughtcrime.securesms 8.22.2

# 2. Install a local .apks bundle:
gplay-cli install ./app.apks

# 3. Install a local folder containing split APKs:
gplay-cli install ./app_splits/

# 4. Install a local standalone .apk:
gplay-cli install ./app.apk
```

#### 🔑 Authentication & Session Management

`gplay-cli` supports both **Zero-Setup Anonymous Mode** and **Custom Google Account Login**:

##### 1. Anonymous Mode (Default — Zero Setup)
By default, `gplay-cli` automatically negotiates an anonymous session using Aurora token dispensers (`auroraoss.com`), registers a simulated Android device profile (Pixel 7a), performs check-in, and caches the session locally in `~/.config/gplay-cli/session.json`.

```bash
gplay-cli auth status     # View active session type (Anonymous vs Custom)
gplay-cli auth refresh    # Acquire a fresh anonymous session
gplay-cli auth logout     # Clear cached credentials
```

##### 2. Custom Google Account Login
To download apps linked to your personal Google account (e.g. your purchased apps, beta tracks, or account-specific licensing), simply use the native `login` command:

```bash
# Interactive sign-in (guides you to sign in via browser):
gplay-cli auth login

# Or pass credentials directly:
gplay-cli auth login --email "your_email@gmail.com" --token "oauth2_4/..."
```

To switch back to anonymous mode at any time, run `gplay-cli auth logout` or `gplay-cli auth refresh`.


## Project References

`gplay-cli` is based on these projects:

- [AuroraStore](https://github.com/whyorean/AuroraStore)
- [YalpStore](https://github.com/yeriomin/YalpStore)
- [AppCrawler](https://github.com/Akdeniz/google-play-crawler)
- [Raccoon](https://github.com/onyxbits/raccoon4)
- [SAI](https://github.com/Aefyr/SAI)

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
