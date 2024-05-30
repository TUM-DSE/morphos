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

    let (packet_received_sender, packet_received_receiver) = std::sync::mpsc::channel();

    let click_api = ClickApi::new()?;
    let mut vm = vm::Vm::new(packet_received_sender)?;
    let mut app = App::new(click_api, packet_received_receiver)?;

    thread::spawn(move || {
        vm.run().expect("Failed to run VM");
    });
    app.run()?;

    Ok(())
}
