use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

pub struct Vm {
    vm_packet_received_sender: Sender<()>,
    post_filtering_packet_received_sender: Sender<()>
}

impl Vm {
    pub fn new(vm_packet_received_sender: Sender<()>, post_filtering_packet_received_sender: Sender<()>) -> eyre::Result<Self> {
        Ok(Self {
            vm_packet_received_sender,
            post_filtering_packet_received_sender
        })
    }

    pub fn run(&mut self) -> eyre::Result<()> {
        let drain = tui_logger::Drain::new();

        let mut child = Command::new("sh")
            .arg("run.sh")
            .stdout(Stdio::piped())
            .current_dir("..")
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            let line = line?;
            let cleaned_line = strip_ansi_escapes::strip_str(&line);

            drain.log(&log::Record::builder()
                .args(format_args!("{}\n", cleaned_line))
                .level(log::Level::Info)
                .target("vm")
                .build());

            if line.contains("Received packet (pre-filtering)") && !line.contains("->") {
                self.vm_packet_received_sender.send(())?;
            }


            if line.contains("Received packet (post-filtering)") && !line.contains("->") {
                self.post_filtering_packet_received_sender.send(())?;
            }
        }

        Ok(())
    }
}

impl Drop for Vm {
    fn drop(&mut self) {}
}