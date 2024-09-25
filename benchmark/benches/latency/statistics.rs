use crate::Datapoint;
use serde::{Deserialize, Serialize};
use statrs::statistics::{Distribution as StatsrsDistribution, Max, Median, Min, OrderStatistics};
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
    pub q25: f64,
    pub q75: f64,
    pub min_without_outliers: f64,
    pub max_without_outliers: f64,
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
    let mut data = statrs::statistics::Data::new(points.to_vec());

    let min = data.min();
    let max = data.max();
    let mean = data.mean().expect("cannot calculate mean");
    let variance = data.variance().expect("cannot calculate variance");
    let std_dev = data.std_dev().expect("cannot calculate std dev");
    let median = data.median();
    let q25 = data.percentile(25);
    let q75 = data.percentile(75);
    let iqr = data.interquartile_range();

    let considered_min_without_outliers = q25 - 1.5 * iqr;
    let considered_max_without_outliers = q75 + 1.5 * iqr;

    let min_without_outliers = *data
        .iter()
        .filter(|&x| *x >= considered_min_without_outliers)
        .min_by(|a, b| a.total_cmp(b))
        .expect("cannot calculate min without outliers");
    let max_without_outliers = *data
        .iter()
        .filter(|&x| *x <= considered_max_without_outliers)
        .max_by(|a, b| a.total_cmp(b))
        .expect("cannot calculate max without outliers");

    Distribution {
        min,
        max,
        mean,
        variance,
        std_dev,
        median,
        min_without_outliers,
        max_without_outliers,
        q25,
        q75,
    }
}
