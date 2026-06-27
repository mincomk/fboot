#!/usr/bin/env sh
set -eu
pnpm --dir fboot install --frozen-lockfile
pnpm --dir fboot build
cargo build --release --manifest-path fbootd/Cargo.toml --features frontend
out_dir="fbootd/target${CARGO_BUILD_TARGET:+/$CARGO_BUILD_TARGET}/release"
echo "Built fbootd with embedded frontend -> $out_dir/fbootd"
