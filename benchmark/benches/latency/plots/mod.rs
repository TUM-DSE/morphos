use crate::plots::latency::{latency, latency_small};
use crate::Datapoint;
use criterion_plot::*;

mod latency;
mod whisker;

pub use whisker::whisker;

static DEFAULT_FONT: &str = "Helvetica";
static SIZE: Size = Size(1280, 720);

const DARK_BLUE: Color = Color::Rgb(31, 120, 180);

pub fn create_plots(name: &str, datapoints: &[Datapoint]) {
    // merge with time
    let latency_time = datapoints.iter().map(|d| (d.time, d.latency.as_nanos() as f64)).collect::<Vec<_>>();

    let children = [
        latency(name, "packets", "ns", &latency_time, None),
        latency_small(name, "ns", "packets", &latency_time, None),
    ];

    for mut child in children {
        child.wait().expect("couldn't wait for child");
    }
}
