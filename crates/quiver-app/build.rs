//! Embeds `windows/quiver.rc` (the app icon and version strings) into the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=windows/quiver.rc");
    println!("cargo:rerun-if-changed=../../installer/icons/icon.ico");
    #[cfg(windows)]
    {
        let embedded = embed_resource::compile("windows/quiver.rc", embed_resource::NONE);
        if let Err(error) = embedded.manifest_required() {
            println!("cargo:warning=embedding the Windows icon failed: {error}");
            std::process::exit(1);
        }
    }
}
