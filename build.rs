fn main() {
    println!("cargo:rerun-if-changed=res/pdflight.rc");
    println!("cargo:rerun-if-changed=res/pdflight.manifest");
    println!("cargo:rerun-if-changed=res/pdflight.ico");
    embed_resource::compile("res/pdflight.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
