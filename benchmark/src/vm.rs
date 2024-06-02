use crate::terminal::restore_echo;
use std::io::{BufReader, Lines};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::process::{Child, ChildStdout, Command, Stdio};

pub const DATA_IFACE: &str = "clicknet";
pub const DATA_ADDR: Ipv4Addr = Ipv4Addr::new(172, 44, 0, 2);
pub const CONTROL_ADDR: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(173, 44, 0, 2), 4444);

pub struct ClickVm {
    pub child: Child,
    pub stdout: Option<BufReader<ChildStdout>>,
}

impl Drop for ClickVm {
    fn drop(&mut self) {
        self.child.kill().expect("failed to kill click child");
        restore_echo();
    }
}

pub enum FileSystem<'a> {
    CpioArchive(&'a str),
    Raw(&'a str),
}

pub fn start_click(fs: FileSystem, extra_args: &[String]) -> anyhow::Result<ClickVm> {
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

    let mut child = Command::new("sudo")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = BufReader::new(child.stdout.take().expect("cannot get stdout of click vm"));

    Ok(ClickVm { child, stdout: Some(stdout) })
}

pub fn wait_until_ready(lines: &mut Lines<BufReader<ChildStdout>>) {
    for line in lines {
        if let Ok(line) = line {
            if line.contains("Received packet") && !line.contains("->") {
                return;
            }
        }
    }
}

pub fn wait_until_driver_start(lines: &mut Lines<BufReader<ChildStdout>>) {
    for line in lines {
        if let Ok(line) = line {
            if line.contains("Starting driver...") {
                return;
            }
        }
    }
}
