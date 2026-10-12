#!/bin/sh
# Packages a locally built DenizSideloader.app into an unsigned IPA for
# DenizSigner (Windows) signing. macOS only (needs Xcode build first).
# Usage: ./package-ipa.sh [derivedDataPath]
set -eu
DERIVED="${1:-build/derived}"
APP="$DERIVED/Build/Products/Release-iphoneos/DenizSideloader.app"
if [ ! -d "$APP" ]; then
  echo "missing $APP — run the Xcode Release build first" >&2
  exit 1
fi
rm -rf Payload DenizSideloader-unsigned.ipa
mkdir Payload
cp -R "$APP" Payload/
zip -r -y DenizSideloader-unsigned.ipa Payload >/dev/null
rm -rf Payload
echo "wrote DenizSideloader-unsigned.ipa"
