# Silas Dotfiles
Mein Ricing Setup (Hyprland, Waybar, SDDM, Rust Control-Center)

## System-Dateien manuell anwenden
Systemdateien sind nicht verlinkt, um Berechtigungsprobleme (z.B. beim Login Screen) zu vermeiden.
Wenn du sie in diesem Repo änderst, musst du sie mit folgendem Befehl ins System kopieren:

```bash
sudo cp -r system/keyd/default.conf /etc/keyd/
sudo cp -r system/sddm/kde_settings.conf /etc/sddm.conf.d/
sudo cp -r system/sddm/themes/silas-theme /usr/share/sddm/themes/
```
