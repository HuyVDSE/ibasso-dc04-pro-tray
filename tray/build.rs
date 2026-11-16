fn main() {
    for path in ["assets/tray.rc", "assets/tray.ico", "assets/tray.manifest"] {
        println!("cargo:rerun-if-changed={path}");
    }
    embed_resource::compile_for("assets/tray.rc", ["dc04-tray"], embed_resource::NONE)
        .manifest_required()
        .expect("embed the tray icon and application manifest");
}
