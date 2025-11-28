use std::path::PathBuf;

fn main() {
    let mut res = winres::WindowsResource::new();
    
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR environment variable not set");

    let icon_path = PathBuf::from(manifest_dir)
        .join("icon")
        .join("RAC.ico");

    res.set_icon(icon_path.to_str().expect("Invalid icon path"));
    res.compile().unwrap();
}