# Silas Ricing & Dotfiles 🚀

Dieses Repository enthält mein komplettes Hyprland- und Sway-Setup (inklusive UI, Statusleiste, Login-Screen und eigenen Rust-Modulen). Durch Symlinks können all diese Konfigurationen mit einem Klick auf einem neuen System hergestellt werden.

## Vorbereitungen auf einem frischen System (Arch)

Bevor du dieses Repo installierst, musst du die Kern-Programme über `pacman` installieren:

### 📦 Benötigte Pakete (offizielle Repos)
- **Basis:** `hyprland` (≥ 0.55, für natives `hyprland.lua`), `sway`, `waybar`, `kitty`, `rofi-wayland`, `swaybg`, `fastfetch`
- **System:** `sddm` (Login Manager), `keyd` (Tastatur-Remapping)
- **Theming:** `papirus-icon-theme` (wird nicht im Repo gespeichert!)
- **Entwicklung:** `rust`, `cargo` (Rust-Shell-Module inkl. `fidget-rs`)
- **Screenshots/Clipboard:** `grim`, `slurp`, `wl-clipboard`
- **Audio/Media:** `pipewire`, `pipewire-pulse`, `wireplumber`, `playerctl`
- **Netzwerk/Bluetooth:** `networkmanager`, `bluez`, `bluez-utils`
- **Hardware/Power:** `brightnessctl`, `power-profiles-daemon`
- **Sonstiges:** `jq`
- **Ask-AI-Overlay (`hypr/scripts/ask-ai-overlay.py`):** `python-gobject`, `python-cairo`, `webkit2gtk-4.1`, `gtk-layer-shell`

### 📦 Benötigte Pakete (AUR, z. B. via `yay`/`paru`)
- `wallust` (Farbschema-Generator, nicht in den offiziellen Repos)

> Hinweis zu Hyprland: Seit Version 0.55 lädt Hyprland automatisch `hyprland.lua` statt `hyprland.conf`, wenn beide/eine Lua-Config vorhanden ist — kein zusätzliches Plugin nötig, aber du brauchst mindestens diese Version.

## 🛠️ Installation

1. **Repository klonen**
   Das Repo **muss** unter `~/.config/dotfiles` abgelegt werden:
   ```bash
   git clone https://github.com/DEIN_USERNAME/dotfiles.git ~/.config/dotfiles
   cd ~/.config/dotfiles
   ```

2. **Setup ausführen**
   Führe einfach das mitgelieferte Installationsskript aus. Es verlinkt alle Configs, kompiliert den Rust-Code und kopiert die System-Dateien für den Login-Screen:
   ```bash
   chmod +x install.sh
   ./install.sh
   ```

3. **Neustart**
   Starte das System neu, damit SDDM (Login Screen) und der Keyd-Daemon aktiv werden.

## ⚙️ Wie funktioniert das?
Anstatt Config-Dateien versteckt im System zu haben, liegen sie alle hier in `~/.config/dotfiles`.
Das Skript `install.sh` legt am Original-Ort (z.B. `~/.config/hypr`, `~/.config/sway`) einfach eine Verknüpfung an, die hier auf das Repo zeigt.
Wenn du etwas am Ricing änderst, änderst du automatisch die Dateien in diesem Git-Repo. Du musst danach nur noch pushen!

Das Shell-Ricing (`shell/custom_bash.sh`) wird sowohl in `~/.bashrc` als auch in `~/.zshrc` eingebunden.
