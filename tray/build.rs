use std::{env, fs, path::PathBuf};

fn main() {
    for path in [
        "Cargo.toml",
        "assets/tray.rc",
        "assets/tray.ico",
        "assets/tray.manifest",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    let version = env!("CARGO_PKG_VERSION");
    let major = env!("CARGO_PKG_VERSION_MAJOR");
    let minor = env!("CARGO_PKG_VERSION_MINOR");
    let patch = env!("CARGO_PKG_VERSION_PATCH");
    fs::write(
        out_dir.join("app_version.h"),
        format!(
            "#define APP_VERSION_NUMERIC {major},{minor},{patch},0\n\
             #define APP_VERSION_TEXT \"{version}\"\n"
        ),
    )
    .expect("write the app version resource header");

    embed_resource::compile_for("assets/tray.rc", ["dc04-tray"], embed_resource::NONE)
        .manifest_required()
        .expect("embed the tray icon, application manifest, and version");
}
