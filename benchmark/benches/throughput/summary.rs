use crate::statistics::{calculate_statistics, derivative, Distribution};
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

    pub packets_per_time_unit: Vec<f64>,
    pub packets_per_time_unit_statistics: Distribution,

    pub bytes_per_time_unit: Vec<f64>,
    pub bytes_per_time_unit_statistics: Distribution,
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
            "{:24}: {:.2} packets/s, {:.2} bytes/s",
            self.name,
            self.packets_per_time_unit_statistics.median,
            self.bytes_per_time_unit_statistics.median
        )
    }
}

pub fn calculate_summary<'a>(results: &[(&'a str, Vec<Datapoint>)]) -> Summary<'a> {
    let mut summary = Vec::with_capacity(results.len());
    for (name, datapoints) in results {
        let packets: Vec<_> = datapoints.iter().map(|d| d.rx_packets).collect();
        let bytes: Vec<_> = datapoints.iter().map(|d| d.rx_bytes).collect();

        let packets_per_time_unit = derivative(&packets);
        let bytes_per_time_unit = derivative(&bytes);

        let statistics = calculate_statistics(&datapoints);

        let entry = SummaryEntry {
            name,
            packets_per_time_unit,
            packets_per_time_unit_statistics: statistics.rx_packets,
            bytes_per_time_unit,
            bytes_per_time_unit_statistics: statistics.rx_bytes,
        };
        summary.push(entry);
    }

    Summary { results: summary }
}
