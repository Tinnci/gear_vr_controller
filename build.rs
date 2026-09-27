fn main() -> Result<(), Box<dyn std::error::Error>> {
    windows_reactor_setup::as_self_contained();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=assets/app-icon.ico");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let profile = out
        .ancestors()
        .nth(3)
        .ok_or("Cannot locate target profile")?;
    std::fs::copy("assets/app-icon.ico", profile.join("app-icon.ico"))?;
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("assets/app-icon.ico")
        .set("ProductName", "Gear VR Controller")
        .set("FileDescription", "Gear VR Bluetooth LE input bridge")
        .set("OriginalFilename", "gear_vr_controller_rust.exe")
        .set(
            "LegalCopyright",
            "Copyright Gear VR Controller contributors. MIT License.",
        );
    resource.compile()?;
    Ok(())
}
