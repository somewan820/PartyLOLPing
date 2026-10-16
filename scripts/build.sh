#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"

if [[ -z "${DEVELOPER_DIR:-}" && -d /Applications/Xcode.app/Contents/Developer ]]; then
    export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi

cargo build --manifest-path rust/Cargo.toml --release --bin partylolping

APP="$PWD/dist/PartyLOLPing.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp rust/target/release/partylolping "$APP/Contents/MacOS/PartyLOLPing"

plutil -convert xml1 -o "$APP/Contents/Info.plist" Resources/Info.plist
cp -R Resources/Artwork Resources/Icons Resources/Sounds "$APP/Contents/Resources/"
cp Resources/asset-sources.json "$APP/Contents/Resources/"

if [[ ! -f "$APP/Contents/Resources/AppIcon.icns" || scripts/make_icon.sh -nt "$APP/Contents/Resources/AppIcon.icns" ]]; then
    scripts/make_icon.sh "$PWD/.build/AppIcon.iconset"
    if ! iconutil -c icns .build/AppIcon.iconset -o "$APP/Contents/Resources/AppIcon.icns"; then
        sips -s format icns .build/AppIcon.iconset/icon_512x512.png \
            --out "$APP/Contents/Resources/AppIcon.icns" >/dev/null
    fi
fi

codesign --force --sign - --identifier local.allentu.PartyLOLPing \
    --entitlements Resources/PartyLOLPing.entitlements "$APP"
codesign --verify --deep --strict "$APP"

for key in CFBundleIdentifier NSLocalNetworkUsageDescription NSScreenCaptureUsageDescription NSBonjourServices; do
    if ! plutil -extract "$key" xml1 -o - "$APP/Contents/Info.plist" >/dev/null; then
        print -u2 "Missing required bundle key: $key"
        exit 1
    fi
done
bonjour_xml=$(plutil -extract NSBonjourServices xml1 -o - "$APP/Contents/Info.plist")
for service in _partylolping._tcp _partylolping-media._tcp; do
    if [[ "$bonjour_xml" != *"$service"* ]]; then
        print -u2 "Missing Bonjour service: $service"
        exit 1
    fi
done
entitlements_xml=$(codesign -d --entitlements :- "$APP" 2>/dev/null)
for entitlement in com.apple.security.network.client com.apple.security.network.server; do
    if ! grep -A1 -F "<key>$entitlement</key>" <<<"$entitlements_xml" | grep -q '<true/>'; then
        print -u2 "Missing network entitlement: $entitlement"
        exit 1
    fi
done

"$APP/Contents/MacOS/PartyLOLPing" --check-assets
print "Built GPUI app: $APP"
