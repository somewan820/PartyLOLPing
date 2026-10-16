#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
cargo fmt --manifest-path rust/party-engine/Cargo.toml -- --check
cargo test --manifest-path rust/party-engine/Cargo.toml
cargo check --manifest-path rust/party-engine/Cargo.toml --bin partylolping
