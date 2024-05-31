use std::path::Path;

const MKCPIO_PATH: &str = "../.unikraft/unikraft/support/scripts/mkcpio";

pub fn make_cpio_archive(archive_result: &Path, to_archive: &Path) -> anyhow::Result<()> {
    if let Some(archive_result_parent_dir) = archive_result.parent() {
        std::fs::create_dir_all(archive_result_parent_dir)?;
    };

    std::process::Command::new("sh")
        .arg(MKCPIO_PATH)
        .arg(archive_result)
        .arg(to_archive)
        .output()?;
    Ok(())
}
