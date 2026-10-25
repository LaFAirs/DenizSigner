#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
version=$(python -c 'import json; print(json.load(open("package.json"))["version"])')
name="DenizSigner-${version}-linux-x86_64"
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/$name/src-tauri/target/release" "$stage/$name/src-tauri/icons" "$stage/$name/packaging/linux" "$stage/$name/scripts" "$root/releases"
install -m755 src-tauri/target/release/denizsigner "$stage/$name/src-tauri/target/release/denizsigner"
install -m644 src-tauri/icons/128x128@2x.png "$stage/$name/src-tauri/icons/"
install -m644 packaging/linux/denizsigner.desktop "$stage/$name/packaging/linux/"
install -m755 scripts/install-linux.sh "$stage/$name/scripts/"
install -m644 LICENSE docs/LINUX.md "$stage/$name/"
tar -C "$stage" -czf "releases/$name.tar.gz" "$name"
(cd releases && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
