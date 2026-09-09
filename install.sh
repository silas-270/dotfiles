#!/bin/bash
set -e

echo "======================================"
echo "🚀 Silas Dotfiles Installer"
echo "======================================"

# 1. Sicherstellen, dass .config und .cache existieren
mkdir -p ~/.config
mkdir -p ~/.cache

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

DOTFILES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

link_config "$DOTFILES_DIR/theme" ~/.config/theme
link_config "$DOTFILES_DIR/wallust" ~/.config/wallust
link_config "$DOTFILES_DIR/hypr" ~/.config/hypr
link_config "$DOTFILES_DIR/sway" ~/.config/sway
link_config "$DOTFILES_DIR/waybar" ~/.config/waybar
link_config "$DOTFILES_DIR/fastfetch" ~/.config/fastfetch
link_config "$DOTFILES_DIR/rofi" ~/.config/rofi
link_config "$DOTFILES_DIR/kitty" ~/.config/kitty
link_config "$DOTFILES_DIR/gtk/gtk-3.0" ~/.config/gtk-3.0
link_config "$DOTFILES_DIR/gtk/gtk-4.0" ~/.config/gtk-4.0

link_config "$DOTFILES_DIR/themes" ~/.config/themes

# Shell (Nicht komplett überschreiben, sondern nur einbinden!)
for rc in ~/.bashrc ~/.zshrc; do
    if ! grep -q "source $DOTFILES_DIR/shell/custom_bash.sh" "$rc" 2>/dev/null; then
        echo "" >> "$rc"
        echo "# Lade Ricing & Custom Configs" >> "$rc"
        echo "source $DOTFILES_DIR/shell/custom_bash.sh" >> "$rc"
    fi
done

# Wallpaper
rm -f ~/.cache/wallpaper-home.jpg && ln -s "$DOTFILES_DIR/themes/savanna-dusk/wallpaper1.jpg" ~/.cache/wallpaper-home.jpg

echo "🦀 Kompiliere Custom Rust Shell (rustbar + control-center)..."
if command -v cargo &> /dev/null; then
    cd "$DOTFILES_DIR/hypr/scripts"
    cargo build --release
    echo "Rust Projekte erfolgreich kompiliert."

    echo "🦀 Kompiliere fidget-rs (eigenständiges Projekt, eigenes Toolchain)..."
    (cd "$DOTFILES_DIR/hypr/scripts/fidget-rs" && cargo build --release)
    echo "fidget-rs erfolgreich kompiliert."
else
    echo "⚠️ 'cargo' nicht gefunden! Bitte Rust installieren und manuell kompilieren."
fi

echo "🛡️ Richte Systemdateien ein (Passwort wird benötigt)..."
# SDDM & Keyd einrichten
sudo mkdir -p /etc/keyd
sudo mkdir -p /etc/sddm.conf.d
sudo mkdir -p /usr/share/sddm/themes

sudo cp -r "$DOTFILES_DIR/system/keyd/default.conf" /etc/keyd/
sudo cp -r "$DOTFILES_DIR/system/sddm/kde_settings.conf" /etc/sddm.conf.d/
sudo cp -r "$DOTFILES_DIR/system/sddm/themes/silas-theme" /usr/share/sddm/themes/
sudo cp -r "$DOTFILES_DIR/system/sddm/themes/africa" /usr/share/sddm/themes/

echo "======================================"
echo "✅ Installation abgeschlossen!"
echo "Bitte starte den PC neu, damit Keyd und SDDM greifen."
echo "======================================"
