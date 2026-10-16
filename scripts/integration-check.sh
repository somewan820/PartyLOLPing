#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
cargo test --manifest-path rust/Cargo.toml network
cargo test --manifest-path rust/Cargo.toml discovery
cargo check --manifest-path rust/Cargo.toml --bin partylolping
print "connection_status=ready transport=rust-tcp discovery=rust room_domain=rust ui=gpui"
