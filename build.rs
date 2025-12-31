use std::env;

fn main() {
    let version =
        env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION environment variable must be set");

    let (major, minor, patch) = parse_semver(&version);

    let compile_args = [
        format!("CARGO_PKG_VERSION={version}"),
        format!("CARGO_PKG_VERSION_MAJOR={major}"),
        format!("CARGO_PKG_VERSION_MINOR={minor}"),
        format!("CARGO_PKG_VERSION_PATCH={patch}"),
    ];

    embed_resource::compile("icon/RAC.rc", &compile_args)
        .manifest_required()
        .expect("Failed to compile Windows resource file");

    println!("cargo:rerun-if-changed=icon/RAC.rc");
    println!("cargo:rerun-if-changed=icon/Manifest.xml");
    println!("cargo:rerun-if-changed=icon/RAC.ico");
}

fn parse_semver(version: &str) -> (u32, u32, u32) {
    let parts: Vec<u32> = version
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();

    match parts.as_slice() {
        [major, minor, patch, ..] => (*major, *minor, *patch),
        [major, minor] => (*major, *minor, 0),
        [major] => (*major, 0, 0),
        [] => (0, 0, 0),
    }
}
