use serde::{Deserialize, Serialize};
use statrs::statistics::{Distribution as StatsrsDistribution, Max, Median, Min};
use std::fmt::{Display, Formatter};

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Statistics {
    pub memory: Distribution,
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
        write!(f, "=== Memory ===\n{}\n", self.memory)
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

pub fn calculate_statistics(datapoints: &[u64]) -> Statistics {
    Statistics {
        memory: calculate_distribution(&datapoints),
    }
}

fn calculate_distribution(points: &[u64]) -> Distribution {
    let points = points.iter().map(|&x| x as f64).collect::<Vec<f64>>();
    let data = statrs::statistics::Data::new(points);

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
