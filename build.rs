use std::env;

fn main() {
    let version =
        env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION environment variable must be set");

    let (major, minor, patch) = parse_semver(&version);

    win_resource::ResourceCompiler::new("resources/RAC.rc")
        .define("CARGO_PKG_VERSION", &version)
        .define("CARGO_PKG_VERSION_MAJOR", major.to_string())
        .define("CARGO_PKG_VERSION_MINOR", minor.to_string())
        .define("CARGO_PKG_VERSION_PATCH", patch.to_string())
        .manifest_required()
        .compile()
        .expect("Failed to compile Windows resource file");

    println!("cargo:rerun-if-changed=resources/RAC.rc");
    println!("cargo:rerun-if-changed=resources/Manifest.xml");
    println!("cargo:rerun-if-changed=icon/RAC.ico");
}

fn parse_semver(version: &str) -> (u32, u32, u32) {
    let version = version.trim();

    let core_version = version
        .split('-')
        .next()
        .unwrap_or(version)
        .split('+')
        .next()
        .unwrap_or(version);

    let parts: Vec<&str> = core_version.split('.').collect();

    if parts.len() != 3 {
        panic!(
            "Invalid Semver format: expected MAJOR.MINOR.PATCH, got '{}' (parts: {})",
            version,
            parts.len()
        );
    }

    let major = parts[0].parse::<u32>().unwrap_or_else(|_| {
        panic!(
            "Invalid MAJOR version number: '{}' in version '{}'",
            parts[0], version
        )
    });

    let minor = parts[1].parse::<u32>().unwrap_or_else(|_| {
        panic!(
            "Invalid MINOR version number: '{}' in version '{}'",
            parts[1], version
        )
    });

    let patch = parts[2].parse::<u32>().unwrap_or_else(|_| {
        panic!(
            "Invalid PATCH version number: '{}' in version '{}'",
            parts[2], version
        )
    });

    (major, minor, patch)
}
