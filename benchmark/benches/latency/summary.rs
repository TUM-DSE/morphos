use crate::statistics::{calculate_statistics, Distribution};
use crate::Datapoint;
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

    pub latency: Vec<u128>,
    pub latency_statistics: Distribution,
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
            "{:24}: {:.2} ns",
            self.name, self.latency_statistics.median
        )
    }
}

pub fn calculate_summary<'a>(results: &[(&'a str, Vec<Datapoint>)]) -> Summary<'a> {
    let mut summary = Vec::with_capacity(results.len());
    for (name, datapoints) in results {
        let latency: Vec<_> = datapoints.iter().map(|d| d.latency.as_nanos()).collect();
        let statistics = calculate_statistics(&datapoints);

        let entry = SummaryEntry {
            name,
            latency,
            latency_statistics: statistics.latency,
        };
        summary.push(entry);
    }

    Summary { results: summary }
}
