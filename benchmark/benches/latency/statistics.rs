use crate::Datapoint;
use serde::{Deserialize, Serialize};
use statrs::statistics::{Distribution as StatsrsDistribution, Max, Median, Min};
use std::fmt::{Display, Formatter};

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Statistics {
    pub latency: Distribution,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Distribution {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub median: f64,
}

impl Display for Statistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "=== Latency ===\n{}\n", self.latency)
    }
}

impl Display for Distribution {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Min: {}\nMax: {}\nMean: {}\nVariance: {}\nStd Dev: {}\nMedian: {}",
            self.min, self.max, self.mean, self.variance, self.std_dev, self.median
        )
    }
}

pub fn calculate_statistics(datapoints: &[Datapoint]) -> Statistics {
    let latency: Vec<_> = datapoints
        .iter()
        .map(|datapoint| datapoint.latency.as_nanos() as f64)
        .collect();

    Statistics {
        latency: calculate_distribution(&latency),
    }
}

fn calculate_distribution(points: &[f64]) -> Distribution {
    let data = statrs::statistics::Data::new(points.to_vec());

    let min = data.min();
    let max = data.max();
    let mean = data.mean().expect("cannot calculate mean");
    let variance = data.variance().expect("cannot calculate variance");
    let std_dev = data.std_dev().expect("cannot calculate std dev");
    let median = data.median();

    Distribution {
        min,
        max,
        mean,
        variance,
        std_dev,
        median,
    }
}
