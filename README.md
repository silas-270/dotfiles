# Silas Ricing & Dotfiles 🚀

Dieses Repository enthält mein komplettes Hyprland-Setup (inklusive UI, Statusleiste, Login-Screen und eigenen Rust-Modulen). Durch Symlinks können all diese Konfigurationen mit einem Klick auf einem neuen System hergestellt werden.

## Vorbereitungen auf einem frischen System (Fedora oder Arch)

Bevor du dieses Repo installierst, musst du die Kern-Programme über deinen Paketmanager (`dnf` bei Fedora, `pacman` bei Arch) installieren:

### 📦 Benötigte Pakete:
- **Basis:** `hyprland`, `waybar`, `kitty`, `rofi` (bzw. `rofi-wayland`), `swaync`, `swaybg`, `fastfetch`
- **System:** `sddm` (Login Manager), `keyd` (Tastatur-Remapping)
- **Theming:** `papirus-icon-theme` (wird nicht im Repo gespeichert!)
- **Entwicklung:** `rust` & `cargo` (um das Control-Center zu kompilieren)

## 🛠️ Installation

1. **Repository klonen**
   Das Repo **muss** im Home-Verzeichnis unter `~/dotfiles` abgelegt werden:
   ```bash
   git clone https://github.com/DEIN_USERNAME/dotfiles.git ~/dotfiles
   cd ~/dotfiles
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
Anstatt Config-Dateien versteckt im System zu haben, liegen sie alle hier in `~/dotfiles`. 
Das Skript `install.sh` legt am Original-Ort (z.B. `~/.config/hypr`) einfach eine Verknüpfung an, die hier auf das Repo zeigt.
Wenn du etwas am Ricing änderst, änderst du automatisch die Dateien in diesem Git-Repo. Du musst danach nur noch pushen!
