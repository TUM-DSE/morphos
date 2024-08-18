use crate::statistics::{calculate_statistics, Distribution};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;

#[derive(Serialize, Deserialize)]
pub struct Summary<'a> {
    #[serde(borrow)]
    pub results: Vec<SummaryEntry<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct SummaryEntry<'a> {
    #[serde(borrow)]
    pub name: &'a str,

    pub memory_usages: Vec<f64>,
    pub memory_usages_distribution: Distribution,
}

impl Display for Summary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for result in &self.results {
            write!(f, "{}\n", result)?;
        }
        Ok(())
    }
}

impl Display for SummaryEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:24}: {:.2} bytes",
            self.name, self.memory_usages_distribution.median
        )
    }
}

pub fn calculate_summary<'a>(results: &[(&'a str, Vec<u64>)]) -> Summary<'a> {
    let mut summary = Vec::with_capacity(results.len());
    for (name, datapoints) in results {
        let statistics = calculate_statistics(&datapoints);

        let entry = SummaryEntry {
            name,
            memory_usages: datapoints.iter().map(|&x| x as f64).collect(),
            memory_usages_distribution: statistics.memory,
        };
        summary.push(entry);
    }

    Summary { results: summary }
}
