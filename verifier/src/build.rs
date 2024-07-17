fn main() -> miette::Result<()> {
    let path = std::path::PathBuf::from("src"); // include path
    let mut b = autocxx_build::Builder::new("src/prevail/mod.rs", &[&path]).build()?;

    b.flag_if_supported("-std=c++20")
        .compile("ebpf-verifier");
    println!("cargo:rerun-if-changed=src/prevail/mod.rs");

    Ok(())
}