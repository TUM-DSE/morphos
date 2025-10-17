use std::thread;
use click_benchmark::startup_base::{self, Configuration, System};

const CONFIGURATIONS: &[Configuration] = &[
    Configuration {
        name: "uk-thomer-nat",
        click_configuration: "configurations/thomer-nat.click",
        vm_extra_args: &[],
        system: System::Unikraft,
    },
    Configuration {
        name: "linux-thomer-nat",
        click_configuration: "configurations/thomer-nat-og.click",
        vm_extra_args: &["DEV0=172.44.0.2/24"],
        system: System::Linux,
    },
    Configuration {
        name: "uk",
        click_configuration: "/tmp/config.click",
        vm_extra_args: &[],
        system: System::UnikraftNoPaging, // unikraft starts faster without paging
    },
    Configuration {
        name: "linux",
        click_configuration: "/tmp/config.click",
        vm_extra_args: &[],
        system: System::Linux,
    },
];

fn main() {
    let only = match  std::env::var("ONLY") {
        Ok(val) => val.split(',').map(|s| s.to_string()).collect::<Vec<String>>(),
        Err(_) => vec![ "uk-thomer-nat".to_string() ],
    };
    let mut configs: Vec<&Configuration> = CONFIGURATIONS.iter().collect();
    if only.len() > 0 {
        let new_configs: Vec<&Configuration> = CONFIGURATIONS
            .iter()
            .filter(|config| only.contains(&config.name.to_string()))
            .collect();
        configs = new_configs;
    }

    thread::spawn(|| {
        startup_base::send_packet_loop().expect("error in send packet loop");
    });

    let nsec = startup_base::run_benchmark(configs[0]).as_nanos();
    println!("Bench-helper startup time (nsec): {}", nsec);
}
