#!/bin/zsh
set -euo pipefail
cd "${0:A:h:h}"
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
cargo test --manifest-path rust/party-engine/Cargo.toml network
cargo check --manifest-path rust/party-engine/Cargo.toml --bin partylolping
print "connection_status=ready transport=rust-tcp discovery=rust room_domain=rust ui=gpui"
