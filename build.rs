fn main() -> Result<(), Box<dyn std::error::Error>> {
    windows_reactor_setup::as_self_contained();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    let mut resource = winresource::WindowsResource::new();
    resource
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
