
if [ -f /etc/bashrc ]; then
    . /etc/bashrc
fi

if ! [[ "$PATH" =~ "$HOME/.local/bin:$HOME/bin:" ]]; then
    PATH="$HOME/.local/bin:$HOME/bin:$PATH"
fi
export PATH

if [ -d ~/.bashrc.d ]; then
    for rc in ~/.bashrc.d/*; do
        if [ -f "$rc" ]; then
            . "$rc"
        fi
    done
fi
unset rc

export JAVA_HOME=/usr/local/jdk-25
export PATH=$JAVA_HOME/bin:$PATH

export SDKMAN_DIR="$HOME/.sdkman"
[[ -s "$HOME/.sdkman/bin/sdkman-init.sh" ]] && source "$HOME/.sdkman/bin/sdkman-init.sh"

if [[ "$HYPRLAND_INSTANCE_SIGNATURE" != "" ]]; then
    fastfetch --config ~/.config/fastfetch/config-hyprland.jsonc
else
    fastfetch
fi

export ANDROID_HOME=~/android-sdk
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools

# Added by Antigravity CLI installer
export PATH="/home/silas270/.local/bin:$PATH"

alias start-wayland='dbus-run-session startplasma-wayland'
. "$HOME/.cargo/env"

runapp() {
  ./gradlew installDebug || return 1

  local package
  package=$(grep -oP 'applicationId\s*=?\s*"\K[^"]+' app/build.gradle* | head -1)
  if [ -z "$package" ]; then
    echo "Konnte applicationId nicht finden."
    return 1
  fi

  local launcher
  launcher=$(adb shell cmd package resolve-activity --brief "$package" | tail -1 | tr -d '\r')

  if [ -z "$launcher" ]; then
    echo "Konnte Launcher-Activity nicht finden."
    return 1
  fi

  adb shell am start -n "$launcher" && \
  adb logcat "*:S" AndroidRuntime:E System.err:W RustStdoutStderr:D "$(basename "$package")":D
}
