# 🛠️ Building Ultimate Game Backup Manager

## 🦀 Standard build

**Prerequisites**
- [Rust](https://rust-lang.org/)
- A C compiler

**Steps**
```sh
git clone https://github.com/petter1/UltimateGameBackupManager.git
cd UltimateGameBackupManager
cargo build --release
```

The binary will be at `./target/release/UltimateGameBackupManager`

## 📦 Distribution build

This will be built with linker-plugin-lto for more in-depth optimization. \
Windows builds will be compatible with Windows XP. \
Linux builds can be built on manylinux2014 or newer.

**Prerequisites**
- [just](https://just.systems/)
- [Rust 1.99](https://rust-lang.org/) with the `rust-src` component
- [LLVM 23](https://llvm.org/)
- [NASM](https://www.nasm.us/) (for build-windows-x86)
- [appimagetool](https://github.com/AppImage/appimagetool) (for package-linux-*)
- [node-appdmg](https://github.com/LinusU/node-appdmg) (for package-macos-universal)

For linux builds, dockerfiles with all required tools are provided in `./dockerfiles`).

**Steps**
```sh
git clone https://github.com/petter1/UltimateGameBackupManager.git
cd UltimateGameBackupManager
just build-<os>-<arch>
just package-<os>-<arch>
```

The available os-arch pairs are:
- windows-x64
- windows-arm64
- windows-x86
- linux-x86_64
- linux-aarch64
- linux-i686
- macos-x86_64 (only build)
- macos-arm64 (only build)
- macos-universal (only package)

The binary will be in `./out` \
The distributable artifact will be in `./dist`


## Icons

All icons are made from `assets/icon.svg`. After changing it, run `just icons` (needs
[resvg](https://github.com/linebender/resvg) – `cargo install resvg` – and ImageMagick). It writes
the PNGs, `package/windows/icon.ico`, the window icon, the Linux AppDir sizes and the macOS iconset;
the `.icns` is only made on macOS, where `iconutil` exists. The generated files are committed.

A newer ImageMagick may write `icon.ico` with different bytes but the same pixels (the 256 px
layer as BMP instead of PNG, about 300 KB instead of 46 KB); keep the committed one then.

## Trying a build on macOS

`cargo run` starts the bare binary: the Dock shows a generic icon, and system notifications and
the `.app`'s name can't be checked. For those, put the binary into a copy of the bundle:

```sh
rm -rf /tmp/ugbm && mkdir /tmp/ugbm
cp -R package/macos/App /tmp/ugbm/UltimateGameBackupManager.app
mkdir -p /tmp/ugbm/UltimateGameBackupManager.app/Contents/MacOS
cargo build
cp target/debug/UltimateGameBackupManager /tmp/ugbm/UltimateGameBackupManager.app/Contents/MacOS/
codesign --force --deep -s - /tmp/ugbm/UltimateGameBackupManager.app
open /tmp/ugbm/UltimateGameBackupManager.app
```

Things to know:

- **Every rebuild is a new app for macOS.** The ad-hoc signature (its cdhash) changes with the binary, so
  macOS asks again for access to removable volumes (and other folders). Until that prompt is
  answered, reading the drive waits. Started from a terminal instead
  (`/tmp/ugbm/UltimateGameBackupManager.app/Contents/MacOS/UltimateGameBackupManager`), the app
  has the terminal's permissions and nothing is asked.
- The app uses the real data folder (`~/Library/Application Support/io.github.petter1.UltimateGameBackupManager`),
  so a test build sees the real library, drives and consoles, and writes to them.
- Whether the computer is kept awake: `pmset -g assertions` while a task runs. Other apps
  (Amphetamine, an open Claude or Electron app) may hold the same assertion, so a "Mac left alone
  until it would sleep" test needs them closed.
