# Swatchbar

A menu-bar / system-tray color sampler and palette builder for macOS, Windows and Linux (X11), written in Rust (egui).

## Build & run

    ./bundle.sh            # builds target/Swatchbar.app (menu-bar only, no Dock icon)
    open target/Swatchbar.app

**Windows** (PowerShell): `./bundle_windows.ps1` builds `target\Swatchbar-windows.zip`.
Needs no permissions. The icon lives in the notification area (check the overflow `^`).

**Linux**: `./bundle_linux.sh` builds `target/swatchbar-linux-<arch>.tar.gz` (binary, `.desktop`, icon).
Build dependencies (Debian/Ubuntu): `libgtk-3-dev libayatana-appindicator3-dev libxdo-dev libxkbcommon-dev`.
Needs an X11 session (XWayland works for the window, but Wayland forbids reading the screen and
global cursor, so sampling is disabled there). The tray icon needs a status-notifier host
(GNOME needs the AppIndicator extension).

Hotkeys default to `Ctrl+Alt+C` / `Ctrl+Alt+P` on Windows and Linux. Window positions are
computed from the monitor under the cursor; mixed-DPI multi-monitor setups may be slightly off.

**macOS**: on first sample, grant **Screen Recording** (System Settings > Privacy & Security),
then quit and reopen the app. Run from the `.app` so the permission sticks to Swatchbar
rather than your terminal.

## Features

- **Sample hotkey** (default `⌃⌥C`, changeable in Settings): opens a zoom loupe next to the
  cursor showing a magnified pixel grid with the center pixel outlined. Press the hotkey
  again (or Return) to take the color; Esc cancels; arrow keys nudge the cursor 1px
  (Shift = 10px). Colors are converted to sRGB. The hex is copied to the clipboard and
  added to Samples.
- **Quick Picker** (default `⌃⌥P`): floating always-on-top window with an HSV picker,
  hex entry, eyedropper, recent samples and your working palette.
- **Palette Studio** (menu bar > Open Palette Studio):
  - Generator: Space for a new batch, lock colors, edit hex, adjust HSL, reorder,
    insert between columns, per-color shade viewer, undo/redo (Cmd+Z / Cmd+Shift+Z),
    save to Library, export hex / CSS variables / JSON.
  - Explore: harmonies (complementary, analogous, triadic, split, tetradic, square,
    mono), shade/tint/tone ladders, contrast checks, RGB/HSL/HSV/OKLCH readouts.
  - Library: saved palettes. Settings: hotkeys, loupe size, formats, permissions.
- **Menu bar icon** with Sample, Quick Picker, Studio, Recent Colors (click to copy), Settings, Quit.

Data lives in `state.json` under the OS config dir: `~/Library/Application Support/Swatchbar/` (macOS), `%APPDATA%\Swatchbar\` (Windows), `~/.config/Swatchbar/` (Linux).
