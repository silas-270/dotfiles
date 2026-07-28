#!/bin/bash
set -e

echo "======================================"
echo "🚀 Silas Dotfiles Installer"
echo "======================================"

# 1. Sicherstellen, dass .config existiert
mkdir -p ~/.config
mkdir -p ~/Bilder/Wallpaper

echo "🔗 Verlinke Konfigurationen..."

# Alte Standard-Ordner löschen (falls vom frischen System angelegt) und neu verlinken
rm -rf ~/.config/hypr && ln -s ~/dotfiles/hypr ~/.config/hypr
rm -rf ~/.config/waybar && ln -s ~/dotfiles/waybar ~/.config/waybar
rm -rf ~/.config/fastfetch && ln -s ~/dotfiles/fastfetch ~/.config/fastfetch
rm -rf ~/.config/rofi && ln -s ~/dotfiles/rofi ~/.config/rofi
rm -rf ~/.config/swaync && ln -s ~/dotfiles/swaync ~/.config/swaync
rm -rf ~/.config/kitty && ln -s ~/dotfiles/kitty ~/.config/kitty
rm -rf ~/.config/gtk-3.0 && ln -s ~/dotfiles/gtk-3.0 ~/.config/gtk-3.0
rm -rf ~/.config/gtk-4.0 && ln -s ~/dotfiles/gtk-4.0 ~/.config/gtk-4.0

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
