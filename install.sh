#!/bin/bash
set -e

echo "======================================"
echo "🚀 Silas Dotfiles Installer"
echo "======================================"

# 1. Sicherstellen, dass .config existiert
mkdir -p ~/.config
mkdir -p ~/Bilder/Wallpaper

echo "🔗 Verlinke Konfigurationen..."

# Ordner in ~/.config sichern (falls vorhanden) und neu verlinken
link_config() {
    local src="$1"
    local dest="$2"
    if [ -e "$dest" ] && [ ! -L "$dest" ]; then
        echo "  📦 Sichere bestehenden Ordner: $dest -> $dest.bak"
        mv "$dest" "$dest.bak"
    elif [ -L "$dest" ]; then
        rm "$dest"
    fi
    ln -s "$src" "$dest"
}

link_config ~/dotfiles/theme ~/.config/theme
link_config ~/dotfiles/hypr ~/.config/hypr
link_config ~/dotfiles/waybar ~/.config/waybar
link_config ~/dotfiles/fastfetch ~/.config/fastfetch
link_config ~/dotfiles/rofi ~/.config/rofi
link_config ~/dotfiles/kitty ~/.config/kitty
link_config ~/dotfiles/gtk/gtk-3.0 ~/.config/gtk-3.0
link_config ~/dotfiles/gtk/gtk-4.0 ~/.config/gtk-4.0

# Shell (Nicht komplett überschreiben, sondern nur einbinden!)
if ! grep -q "source ~/dotfiles/shell/custom_bash.sh" ~/.bashrc; then
    echo "" >> ~/.bashrc
    echo "# Lade Ricing & Custom Configs" >> ~/.bashrc
    echo "source ~/dotfiles/shell/custom_bash.sh" >> ~/.bashrc
fi

# Wallpaper
rm -f ~/Bilder/Wallpaper/wallpaper-home.jpg && ln -s ~/dotfiles/wallpapers/wallpaper-home.jpg ~/Bilder/Wallpaper/wallpaper-home.jpg

echo "🦀 Kompiliere Custom Rust Control-Center..."
if command -v cargo &> /dev/null; then
    cd ~/dotfiles/hypr/scripts/control-center
    cargo build --release
    echo "Rust Projekt erfolgreich kompiliert."
else
    echo "⚠️ 'cargo' nicht gefunden! Bitte Rust installieren und manuell kompilieren."
fi

echo "🛡️ Richte Systemdateien ein (Passwort wird benötigt)..."
# SDDM & Keyd einrichten
sudo mkdir -p /etc/keyd
sudo mkdir -p /etc/sddm.conf.d
sudo mkdir -p /usr/share/sddm/themes

sudo cp -r ~/dotfiles/system/keyd/default.conf /etc/keyd/
sudo cp -r ~/dotfiles/system/sddm/kde_settings.conf /etc/sddm.conf.d/
sudo cp -r ~/dotfiles/system/sddm/themes/silas-theme /usr/share/sddm/themes/

echo "======================================"
echo "✅ Installation abgeschlossen!"
echo "Bitte starte den PC neu, damit Keyd und SDDM greifen."
echo "======================================"
