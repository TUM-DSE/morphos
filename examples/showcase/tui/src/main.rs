use std::thread;
use ratatui::{
    prelude::*,
    widgets::{*, block::*},
};

use crate::app::App;
use crate::click_api::ClickApi;

mod app;
mod click_api;
mod vm;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let (vm_packet_received_sender, vm_packet_received_receiver) = std::sync::mpsc::channel();
    let (post_filtering_packet_received_sender, post_filtering_packet_received_receiver) = std::sync::mpsc::channel();

    let click_api = ClickApi::new()?;
    let mut vm = vm::Vm::new(vm_packet_received_sender, post_filtering_packet_received_sender)?;

    let mut app = App::new(click_api, vm_packet_received_receiver, post_filtering_packet_received_receiver)?;

    thread::spawn(move || {
        vm.run().expect("Failed to run VM");
    });
    app.run()?;

    Ok(())
}
