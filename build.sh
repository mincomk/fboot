#!/usr/bin/env sh
set -eu
pnpm --dir fboot install --frozen-lockfile
pnpm --dir fboot build
cargo build --release --manifest-path fbootd/Cargo.toml --features frontend
echo "Built fbootd with embedded frontend -> fbootd/target/release/fbootd"
