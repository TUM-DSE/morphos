use crate::DATA_ADDR;
use std::io::{BufReader, Lines};
use std::process::{Child, ChildStdout, Command, Stdio};

pub enum FileSystem<'a> {
    CpioArchive(&'a str),
    Raw(&'a str),
}

pub fn start_click(fs: FileSystem, extra_args: &[String]) -> anyhow::Result<Child> {
    let vfs_fstab = match fs {
        FileSystem::CpioArchive(_) => " vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"]",
        FileSystem::Raw(_) => "",
    };
    let initrd = match fs {
        FileSystem::CpioArchive(path) => path,
        FileSystem::Raw(path) => path,
    };

    let mut args = [
        "qemu-system-x86_64",
        "-accel",
        "kvm",
        "-cpu",
        "max",
        "-netdev",
        "bridge,id=en0,br=clicknet",
        "-device",
        "virtio-net-pci,netdev=en0",
        "-append",
        &format!(r#"netdev.ip={DATA_ADDR}/24:172.44.0.1{vfs_fstab} --"#),
        "-kernel",
        "../.unikraft/build/click_qemu-x86_64",
        "-initrd",
        initrd,
        "-nographic",
    ]
    .map(|s| s.to_string())
    .to_vec();

    args.extend_from_slice(extra_args);
    println!("Starting Click with following arguments: {args:?}");

    let child = Command::new("sudo")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    Ok(child)
}

pub fn wait_until_ready(lines: &mut Lines<BufReader<ChildStdout>>) {
    for line in lines {
        if let Ok(line) = line {
            println!("{line}");
            if line.contains("Received packet") && !line.contains("->") {
                return;
            }
        }
    }
}
