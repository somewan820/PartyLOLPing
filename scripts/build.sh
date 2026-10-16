#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
cargo build --manifest-path rust/party-engine/Cargo.toml --release --bin partylolping
APP="$PWD/dist/PartyLOLPing.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp rust/party-engine/target/release/partylolping "$APP/Contents/MacOS/PartyLOLPing"
plutil -convert xml1 -o "$APP/Contents/Info.plist" Resources/Info.plist
cp Resources/asset-sources.json "$APP/Contents/Resources/"
codesign --force --sign - --identifier local.allentu.PartyLOLPing --entitlements Resources/PartyLOLPing.entitlements "$APP"
codesign --verify --deep --strict "$APP"
print "Built GPUI app: $APP"
