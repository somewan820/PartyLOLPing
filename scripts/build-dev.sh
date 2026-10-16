#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"

if [[ ! -d /Applications/Xcode.app/Contents/Developer ]]; then
    print -u2 "Xcode.app is required for GPUI/Metal development."
    exit 1
fi
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
cargo build --manifest-path rust/Cargo.toml --bin partylolping
print "Debug binary: $PWD/rust/target/debug/partylolping"
