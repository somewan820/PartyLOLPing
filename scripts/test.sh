#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"
if [[ -z "${DEVELOPER_DIR:-}" && -d /Applications/Xcode.app/Contents/Developer ]]; then
    export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi
cargo fmt --manifest-path rust/Cargo.toml -- --check
cargo test --manifest-path rust/Cargo.toml
cargo check --manifest-path rust/Cargo.toml --bin partylolping
