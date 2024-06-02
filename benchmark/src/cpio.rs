use std::path::{Path, PathBuf};
use tempfile::TempDir;

const MKCPIO_PATH: &str = "../.unikraft/unikraft/support/scripts/mkcpio";

pub struct CpioArchive {
    pub path: PathBuf,

    // Temporary directory that contains the cpio archive.
    // Held so that it's not deleted until the archive is no longer needed.
    _parent: TempDir,
}

pub fn prepare_cpio_archive(
    click_configuration: &str,
    bpfilter_path: Option<impl AsRef<Path>>,
) -> anyhow::Result<CpioArchive> {
    let tmpdir = tempfile::tempdir()?;

    // write click configuration
    let click_configuration_path = tmpdir.path().join("config.click");
    std::fs::write(&click_configuration_path, click_configuration)?;

    // copy filter binary
    if let Some(bpfilter_path) = bpfilter_path {
        let bpfilter_path = bpfilter_path.as_ref();
        let bpfilter_file_name = bpfilter_path
            .file_name()
            .expect("couldn't find bpfilter file name");
        let filter_binary_path = tmpdir.path().join(bpfilter_file_name);
        std::fs::copy(bpfilter_path, &filter_binary_path)?;
    }

    // create cpio archive
    let cpio_archive_path = tmpdir.path().join("config.cpio");
    make_cpio_archive(&cpio_archive_path, tmpdir.path())?;

    Ok(CpioArchive {
        path: cpio_archive_path,
        _parent: tmpdir,
    })
}

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
