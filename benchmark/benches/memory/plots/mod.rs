use crate::plots::memory::{memory, memory_small};
use criterion_plot::*;

mod memory;
mod whisker;

pub use whisker::whisker;

static DEFAULT_FONT: &str = "Helvetica";
static SIZE: Size = Size(1280, 720);

const DARK_BLUE: Color = Color::Rgb(31, 120, 180);

pub fn create_plots(name: &str, datapoints: &[u64]) {
    // merge with index
    let datapoints_iteration = datapoints
        .into_iter()
        .enumerate()
        .map(|(idx, x)| (idx as u64, *x as f64))
        .collect::<Vec<_>>();

    let children = [
        memory(name, "bytes", "bytes", &datapoints_iteration, None),
        memory_small(name, "bytes", "bytes", &datapoints_iteration, None),
    ];

    for mut child in children {
        child.wait().expect("couldn't wait for child");
    }
}
