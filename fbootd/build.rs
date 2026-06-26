use std::path::Path;

fn main() {
    if std::env::var_os("CARGO_FEATURE_FRONTEND").is_some()
        && !Path::new("../fboot/dist/index.html").exists()
    {
        panic!(
            "frontend feature enabled but ../fboot/dist not found — run ./build.sh (or `pnpm --dir fboot build`) first"
        );
    }

    println!("cargo:rerun-if-changed=../fboot/dist");
}
