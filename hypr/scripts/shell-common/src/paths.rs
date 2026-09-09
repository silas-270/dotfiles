//! Filesystem and socket locations used by both surfaces.
//!
//! Everything here resolves through `$HOME` so the shell is not tied to one
//! account, falling back to the author's home directory only when `$HOME` is
//! unset — which in practice means a stripped environment, not a different user.

use std::path::PathBuf;

/// Last-resort home directory when `$HOME` is missing from the environment.
const FALLBACK_HOME: &str = "/home/silas270";

/// The user's home directory.
pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(FALLBACK_HOME))
}

/// The dotfiles checkout. `~/.config/hypr` is a symlink into it, so either route
/// reaches the same files; this one does not depend on the symlink existing.
pub fn dotfiles() -> PathBuf {
    let cfg = home().join(".config/dotfiles");
    if cfg.exists() {
        cfg
    } else {
        home().join("dotfiles")
    }
}

/// The generated theme palette both surfaces read.
pub fn colors_json() -> PathBuf {
    dotfiles().join("theme/generated/colors.json")
}

/// The directory holding the compositor helper scripts.
pub fn scripts_dir() -> PathBuf {
    dotfiles().join("hypr/scripts")
}

/// A helper script by filename, e.g. `script("rofi-wifi-menu.sh")`.
pub fn script(name: &str) -> PathBuf {
    scripts_dir().join(name)
}

/// The release binary directory for the workspace.
fn bin_dir() -> PathBuf {
    scripts_dir().join("target/release")
}

/// The control-center binary, which rustbar spawns to toggle the panel.
pub fn control_center_bin() -> PathBuf {
    bin_dir().join("control-center")
}

/// A user-local executable, e.g. `user_bin("fedora-mount")`.
pub fn user_bin(name: &str) -> PathBuf {
    home().join(".local/bin").join(name)
}

/// rustbar's IPC socket.
///
/// Scoped by uid under `/tmp` — matching control-center's socket — so runtime
/// state stays out of the dotfiles checkout and two accounts on one machine do
/// not collide.
pub fn rustbar_socket() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/rustbar-{}.sock", uid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_hang_off_home() {
        let home = home();
        assert!(colors_json().starts_with(&home));
        assert!(script("rofi-wifi-menu.sh").starts_with(&home));
        assert!(control_center_bin().starts_with(&home));
        assert!(user_bin("fedora-mount").starts_with(&home));
    }

    #[test]
    fn script_and_bin_names_land_at_the_end() {
        assert!(script("rofi-wifi-menu.sh").ends_with("rofi-wifi-menu.sh"));
        assert!(control_center_bin().ends_with("target/release/control-center"));
        assert!(user_bin("fedora-mount").ends_with(".local/bin/fedora-mount"));
    }

    /// The socket must be uid-scoped and outside the repo, so two accounts on
    /// one machine cannot collide and `git status` stays clean.
    #[test]
    fn socket_is_uid_scoped_under_tmp() {
        let sock = rustbar_socket();
        assert!(sock.starts_with("/tmp"), "socket must not live in the checkout: {sock:?}");
        let uid = unsafe { libc::getuid() };
        assert_eq!(sock.file_name().unwrap(), format!("rustbar-{uid}.sock").as_str());
    }
}
