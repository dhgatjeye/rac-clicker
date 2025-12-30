fn main() {
    let version_str = env!("CARGO_PKG_VERSION");

    let parts: Vec<&str> = version_str.split('.').collect();

    let (major, minor, patch) = match parts.as_slice() {
        [maj, min, pat] => (
            maj.parse::<u32>().unwrap_or(0),
            min.parse::<u32>().unwrap_or(0),
            pat.parse::<u32>().unwrap_or(0),
        ),
        [maj, min] => (
            maj.parse::<u32>().unwrap_or(0),
            min.parse::<u32>().unwrap_or(0),
            0,
        ),
        [maj] => (maj.parse::<u32>().unwrap_or(0), 0, 0),
        _ => (0, 0, 0),
    };

    if let Err(e) = embed_resource::compile(
        "icon/RAC.rc",
        &[
            format!("CARGO_PKG_VERSION={}", version_str),
            format!("CARGO_PKG_VERSION_MAJOR={}", major),
            format!("CARGO_PKG_VERSION_MINOR={}", minor),
            format!("CARGO_PKG_VERSION_PATCH={}", patch),
        ],
    )
    .manifest_required()
    {
        panic!("Failed to compile Windows resources: {}", e);
    }
}
