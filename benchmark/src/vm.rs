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

impl ClickVm {
    /**
    Returns the PID of the QEMU process running the Click VM.
    The direct child is sudo, so we need to find the grandchild.
     */
    pub fn qemu_pid(&self) -> u32 {
        let mut parent_pid = self.child.id();

        loop {
            match ClickVm::get_child_pid(parent_pid) {
                Some(child_pid) => {
                    parent_pid = child_pid;
                }
                None => return parent_pid,
            }
        }
    }

    fn get_child_pid(parent_pid: u32) -> Option<u32> {
        let output = Command::new("pgrep")
            .arg("-P")
            .arg(parent_pid.to_string())
            .output()
            .expect("failed to run pgrep");

        let pid =
            String::from_utf8(output.stdout).expect("failed to convert pgrep output to string");
        pid.trim().parse().ok()
    }
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
        "taskset",
        "-c",
        "3,4",
        "qemu-system-x86_64",
        "-accel",
        "kvm",
        "-cpu",
        "max",
        "-m",
        "12G",
        "-object",
        "memory-backend-file,id=mem,size=12G,mem-path=/dev/hugepages,share=on",
        "-mem-prealloc",
        "-numa",
        "node,memdev=mem",
        "-netdev",
        "bridge,id=en0,br=clicknet",
        "-device",
        "virtio-net-pci,netdev=en0",
        "-append",
        &format!(r#"{vfs_fstab} --"#),
        "-kernel",
        "../VMs/unikraft_nompk",
        "-initrd",
        initrd,
        "-nographic"
    ]
    .map(|s| s.to_string())
    .to_vec();

    args.extend_from_slice(extra_args);

    let mut child = Command::new("sudo")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = BufReader::new(child.stdout.take().expect("cannot get stdout of click vm"));

    Ok(ClickVm {
        child,
        stdout: Some(stdout),
    })
}

pub fn start_linux_click(config: &str, extra_args: &[String]) -> anyhow::Result<ClickVm> {
    let mut args = [
        "taskset",
        "-c",
        "3,4",
        "../nix/builds/click/bin/click",
        config,
    ]
    .map(|s| s.to_string())
    .to_vec();

    args.extend_from_slice(extra_args);

    let mut child = Command::new("sudo")
        .args(args)
        .stdout(Stdio::piped())
        // .stderr(Stdio::piped())
        .spawn()?;

    let stdout = BufReader::new(child.stdout.take().expect("cannot get stdout of click"));

    Ok(ClickVm {
        child,
        stdout: Some(stdout),
    })
}

pub fn wait_until_ready(lines: &mut Lines<BufReader<ChildStdout>>) {
    for line in lines {
        if let Ok(line) = line {
            if cfg!(feature = "print-output") {
                println!("{}", line);
            }

            if line.contains("Received packet") && !line.contains("->") {
                return;
            }
        }
    }
}

pub fn wait_until_driver_start(lines: &mut Lines<BufReader<ChildStdout>>) {
    for line in lines {
        if let Ok(line) = line {
            if cfg!(feature = "print-output") {
                println!("{}", line);
            }

            if line.contains("Starting driver...") {
                return;
            }
        }
    }
}
