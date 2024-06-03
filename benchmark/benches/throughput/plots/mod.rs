use crate::plots::throughput::{throughput, throughput_small};
use crate::statistics::derivative;
use crate::Datapoint;
use criterion_plot::*;

mod throughput;
mod whisker;

pub use whisker::whisker;

static DEFAULT_FONT: &str = "Helvetica";
static KDE_POINTS: usize = 500;
static SIZE: Size = Size(1280, 720);

const LINEWIDTH: LineWidth = LineWidth(2.);
const POINT_SIZE: PointSize = PointSize(0.75);

const DARK_BLUE: Color = Color::Rgb(31, 120, 180);
const DARK_ORANGE: Color = Color::Rgb(255, 127, 0);
const DARK_RED: Color = Color::Rgb(227, 26, 28);

pub fn create_plots(name: &str, datapoints: &[Datapoint]) {
    let packets_points = datapoints.iter().map(|d| d.rx_packets).collect::<Vec<_>>();
    let bytes_points = datapoints.iter().map(|d| d.rx_bytes).collect::<Vec<_>>();

    let packets_derivative = derivative(&packets_points);
    let bytes_derivative = derivative(&bytes_points);

    // merge with time
    let packets_time = packets_derivative
        .into_iter()
        .enumerate()
        .map(|(idx, d)| (datapoints[idx].time, d))
        .collect::<Vec<_>>();
    let bytes_time = bytes_derivative
        .into_iter()
        .enumerate()
        .map(|(idx, d)| (datapoints[idx].time, d))
        .collect::<Vec<_>>();

    let children = [
        throughput(name, "packets", "packets", &packets_time, None),
        throughput_small(name, "packets", "packets", &packets_time, None),
        throughput(name, "bytes", "bytes", &bytes_time, None),
        throughput_small(name, "bytes", "bytes", &bytes_time, None),
    ];

    for mut child in children {
        child.wait().expect("couldn't wait for child");
    }
}
