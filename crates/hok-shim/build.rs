fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=shim.manifest");
    println!("cargo:rerun-if-changed=assets/hok.ico");

    if std::env::var_os("CARGO_CFG_WINDOWS").is_none() {
        return;
    }

    let mut res = winres::WindowsResource::new();
    res.set("FileDescription", "hok shim");
    res.set("ProductName", "hok");
    res.set("OriginalFilename", "hok-shim.exe");

    // winres regenerates the manifest itself from these settings, and it has no
    // notion of consoleAllocationPolicy, so the manifest ships as a file.
    res.set_manifest_file("shim.manifest");
    res.set_icon("assets/hok.ico");
    res.set_language(0x0409);
    res.compile().expect("failed to compile Windows resources");
}
