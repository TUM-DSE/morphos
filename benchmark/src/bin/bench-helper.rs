use std::thread;
use click_benchmark::startup_base::{self, Configuration};

fn main() {
    let config = Configuration {
        name: "thomer-nat",
        click_configuration: "configurations/thomer-nat.click",
        vm_extra_args: &[],
    };

    thread::spawn(|| {
        startup_base::send_packet_loop().expect("error in send packet loop");
    });

    let nsec = startup_base::run_benchmark(&config);
    dbg!(nsec);
}
