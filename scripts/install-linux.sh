#!/usr/bin/env bash
set -euo pipefail
umask 077
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
binary="${1:-$root/src-tauri/target/release/denizsigner}"
[[ -x "$binary" ]] || { echo 'Build the Linux executable first.' >&2; exit 1; }
base="${XDG_DATA_HOME:-$HOME/.local/share}"
config="${XDG_CONFIG_HOME:-$HOME/.config}"
install -Dm755 "$binary" "$HOME/.local/opt/denizsigner/denizsigner"
install -dm755 "$HOME/.local/bin"
cat > "$HOME/.local/bin/denizsigner" <<'LAUNCHER'
#!/bin/sh
# Prefer the native Wayland backend in Wayland sessions. X11 remains usable.
if [ -n "${WAYLAND_DISPLAY:-}" ]; then export GDK_BACKEND=wayland; fi
case "${XDG_CURRENT_DESKTOP:-}" in
  *Hyprland*)
    if [ -f "${XDG_DATA_HOME:-$HOME/.local/share}/macshell/window-decoration/hyprbars.so" ]; then
      export DENIZSIGNER_HYPRBARS=1
    fi
    ;;
esac
exec "$HOME/.local/opt/denizsigner/denizsigner" "$@"
LAUNCHER
chmod 755 "$HOME/.local/bin/denizsigner"
install -Dm644 "$root/src-tauri/icons/128x128@2x.png" "$base/icons/hicolor/256x256/apps/com.denizbudakli.denizsigner.png"
install -dm755 "$base/applications"
python - "$root" "$base" "$config" <<'PY'
import sys,json,os
from pathlib import Path
root,base,config=map(Path,sys.argv[1:])
desktop=(root/'packaging/linux/denizsigner.desktop').read_text()
# Absolute executable path works even when ~/.local/bin is not in the session PATH.
desktop=desktop.replace('Exec=denizsigner','Exec='+str(Path.home()/'.local/bin/denizsigner'))
(base/'applications/denizsigner.desktop').write_text(desktop)
(base/'applications/denizsigner.desktop').chmod(0o644)
pins=config/'macdock/pinned.json'
if pins.is_file():
    values=json.loads(pins.read_text())
    if not isinstance(values,list): raise SystemExit('MacShell pinned.json is not an array; left intact')
    app='denizsigner'
    values=[v for v in values if v != 'com.denizbudakli.denizsigner']
    if app not in values: values.append(app)
    temp=pins.with_suffix('.json.denizsigner-tmp')
    temp.write_text(json.dumps(values,indent=2)+'\n');os.replace(temp,pins)
PY
command -v update-desktop-database >/dev/null && update-desktop-database "$base/applications"
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$base/icons/hicolor" || true
printf 'Installed DenizSigner in %s\n' "$HOME/.local/opt/denizsigner"
