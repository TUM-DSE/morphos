use crate::Configuration;
use click_benchmark::cpio::prepare_cpio_archive;
use click_benchmark::vm::{start_click, wait_until_driver_start, FileSystem};
use std::fs;
use std::io::BufRead;

pub fn measure_memory_usage(config: &Configuration) -> Vec<u64> {
    println!("Preparing CPIO archive");
    let cpio = prepare_cpio_archive(
        &create_click_configuration(config),
        config.files,
    )
    .expect("couldn't prepare cpio archive");

    // Repeat the measurement multiple times
    const MEASUREMENTS: usize = 10;

    let mut memory_usages = Vec::with_capacity(MEASUREMENTS);
    for i in 1..=MEASUREMENTS {
        println!("{i}/{MEASUREMENTS}: Starting Click");
        let mut click_vm = start_click(FileSystem::CpioArchive(&cpio.path.to_string_lossy()), &[])
            .expect("couldn't start click");

        wait_until_driver_start(&mut click_vm.stdout.take().unwrap().lines());

        let pid = click_vm.qemu_pid();
        let memory_usage = get_memory_usage(pid);

        println!("{i}/{MEASUREMENTS}: {memory_usage} bytes");

        memory_usages.push(memory_usage);
    }

    memory_usages
}

fn get_memory_usage(pid: u32) -> u64 {
    let statm_path = format!("/proc/{pid}/statm");
    let statm_contents = fs::read_to_string(statm_path).expect("failed to read from statm");

    // Split the contents by whitespace
    let parts: Vec<&str> = statm_contents.split_whitespace().collect();

    // The second field is the resident memory size in pages
    let resident_pages_str = parts.get(1).expect("Invalid statm format");
    // Parse the number of resident memory pages
    let resident_pages: u64 = resident_pages_str.parse().expect("Invalid resident pages");

    // Get the system page size (in bytes)
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };

    // Calculate memory usage in bytes
    resident_pages * page_size
}

fn create_click_configuration(config: &Configuration) -> String {
    let click_element_config = config.click_config.unwrap_or("");

    format!(
        r#"
FromDevice(0) -> Discard;

SimpleIdle
{click_element_config}
-> Discard;
        "#
    )
}
