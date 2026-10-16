#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"

if [[ ! -d /Applications/Xcode.app/Contents/Developer ]]; then
    print -u2 "Xcode.app is required for GPUI/Metal development."
    exit 1
fi
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

print "Starting PartyLOLPing in GPUI development mode..."
print "Backend/UI: Rust + GPUI | incremental debug build"
exec cargo run --manifest-path rust/Cargo.toml --bin partylolping -- "$@"
