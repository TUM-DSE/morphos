use std::process::Command;

pub fn restore_echo() {
    Command::new("stty")
        .arg("echo")
        .spawn()
        .expect("couldn't spawn stty")
        .wait()
        .expect("stty echo failed");
}
