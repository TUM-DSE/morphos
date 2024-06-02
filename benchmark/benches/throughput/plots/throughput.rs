// Original source dual-licensed under MIT / Apache 2: https://github.com/bheisler/criterion.rs/blob/master/src/plot/gnuplot_backend/iteration_times.rs
// Modified to change iteration times to average throughput

use crate::persistence::plot_path;
use criterion_plot::prelude::*;
use std::process::Child;

use super::*;

fn throughput_figure(unit: &str, data: &[(u64, f64)], size: Option<Size>) -> Figure {
    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(size.unwrap_or(SIZE))
        .configure(Axis::BottomX, |a| {
            a.configure(Grid::Major, |g| g.show())
                .set(Label("Time (ms)"))
        })
        .configure(Axis::LeftY, |a| {
            a.configure(Grid::Major, |g| g.show())
                .set(Label(format!("Throughput ({})", unit)))
        })
        .plot(
            Points {
                x: data.iter().map(|&(x, _)| x),
                y: data.iter().map(|&(_, y)| y),
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(PointSize(0.5))
                    .set(PointType::FilledCircle)
            },
        );
    figure
}

pub fn throughput(
    name: &str,
    unit: &str,
    data_name: &str,
    data: &[(u64, f64)],
    size: Option<Size>,
) -> Child {
    let mut figure = throughput_figure(unit, data, size);
    figure.set(Title(name.to_string()));
    figure.configure(Key, |k| {
        k.set(Justification::Left)
            .set(Order::SampleText)
            .set(Position::Inside(Vertical::Top, Horizontal::Left))
    });

    let path = plot_path(name, &format!("{data_name}-throughput.svg"));
    figure.set(Output(path)).draw().unwrap()
}

pub fn throughput_small(
    name: &str,
    unit: &str,
    data_name: &str,
    data: &[(u64, f64)],
    size: Option<Size>,
) -> Child {
    let mut figure = throughput_figure(unit, data, size);
    figure.configure(Key, |k| k.hide());

    let path = plot_path(name, &format!("{data_name}-throughput-small.svg"));
    figure.set(Output(path)).draw().unwrap()
}
