#!/usr/bin/env bash
# Builds Proza and installs it for this user: ~/.local/bin/proza, the launcher entry and the icon.
# Usage: bash tools/install-linux.sh        (uninstall: bash tools/install-linux.sh --remove)
set -euo pipefail
cd "$(dirname "$0")/.."
bin="$HOME/.local/bin/proza"
apps="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor/scalable/apps"

if [ "${1:-}" = "--remove" ]; then
    rm -f "$bin" "$apps/dev.alecsh.Proza.desktop" "$icons/dev.alecsh.Proza.svg"
    update-desktop-database -q "$apps" 2>/dev/null || true
    echo "Proza removed (settings in ~/.config/proza and themes stay)."
    exit 0
fi

cargo build --release
install -Dm755 target/release/proza "$bin"
install -Dm644 data/app/dev.alecsh.Proza.desktop "$apps/dev.alecsh.Proza.desktop"
install -Dm644 data/app/dev.alecsh.Proza.svg "$icons/dev.alecsh.Proza.svg"
update-desktop-database -q "$apps" 2>/dev/null || true
gtk-update-icon-cache -q -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
echo "Proza installed: $bin"
