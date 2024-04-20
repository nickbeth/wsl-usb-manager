fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    embed_manifest::embed_manifest(embed_manifest::new_manifest(
        "resources/wsl-usb-manager.exe.manifest",
    ))
    .expect("unable to embed manifest file");
    println!("cargo:rerun-if-changed=resources/wsl-usb-manager.exe.manifest");

    embed_resource::compile("resources/resources.rc", embed_resource::NONE);
    println!("cargo:rerun-if-changed=resources/resources.rc");
}
