use std::path::Path;

/// Files that must remain next to the Windows executable at runtime. All
/// other icons, fonts and sounds under `assets/` are embedded in the binary
/// or are build/package artwork only.
const PACKAGED_ASSETS: &[&str] = &["icons/app-icon-64.png", "troubleshoot.ps1"];

fn main() {
    embed_application_manifest();
    write_packaged_assets_manifest();

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/app-icon.ico")
            .set("ProductName", "Agent Minibar")
            .set("FileDescription", "Agent Minibar")
            .set("CompanyName", "Truman_show")
            .set("InternalName", "Agent Minibar")
            .set("OriginalFilename", "agent-minibar.exe")
            .set("LegalCopyright", "Copyright 2026 Truman_show");
        resource
            .compile()
            .expect("compile Windows application icon");
    }
}

/// Record the runtime asset allowlist (mirrored by `build.ps1`) so the app can
/// prune files that older releases installed but this one no longer ships.
fn write_packaged_assets_manifest() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let assets = Path::new(&manifest_dir).join("assets");
    for file in PACKAGED_ASSETS {
        let path = assets.join(file);
        assert!(path.is_file(), "packaged runtime asset missing: {file}");
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let mut source = String::from("&[\n");
    for file in PACKAGED_ASSETS {
        source.push_str(&format!("    {file:?},\n"));
    }
    source.push(']');
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    std::fs::write(Path::new(&out_dir).join("packaged_assets.rs"), source)
        .expect("write packaged assets manifest");
}

/// Embed the application manifest: Common Controls v6 (GPUI and the popup
/// host import TaskDialogIndirect/SetWindowSubclass) and per-monitor v2 DPI
/// awareness, which both GPUI windows rely on for crisp scaling.
fn embed_application_manifest() {
    // Read the directory at run time: `env!` bakes in the checkout the build
    // script was compiled from, which goes stale when worktrees share `target`.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let manifest = Path::new(&manifest_dir).join("assets/app.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_abi = std::env::var("CARGO_CFG_TARGET_ABI").unwrap_or_default();
    match (target_env.as_str(), target_abi.as_str()) {
        ("msvc", _) => {
            println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
            println!(
                "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
                manifest.display()
            );
        }
        ("gnu", "llvm") => {
            println!("cargo:rustc-link-arg-bins=-Wl,/MANIFEST:EMBED");
            println!(
                "cargo:rustc-link-arg-bins=-Wl,/MANIFESTINPUT:{}",
                manifest.display()
            );
        }
        _ => {}
    }
}
