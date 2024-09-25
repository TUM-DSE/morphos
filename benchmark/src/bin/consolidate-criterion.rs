/// This simple binary is used to consolidate the results of a criterion benchmarking group
/// into a single file, and additionally calculate some statistics on the results.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use statrs::statistics::{Distribution, Max, Median, Min, OrderStatistics};
use std::path::{Path, PathBuf};
use itertools::Itertools;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsolidatedBenchmark {
    pub name: String,
    pub results: Vec<f64>,
    pub statistics: Statistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Statistics {
    pub min: f64,
    pub max: f64,
    pub min_without_outliers: f64,
    pub max_without_outliers: f64,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub median: f64,
    pub q25: f64,
    pub q75: f64,
}

struct Result {
    pub name: String,
    pub results: Vec<f64>,
}

fn main() {
    let benchmark_group = std::env::args()
        .nth(1)
        .expect("no benchmark group provided");
    let benchmark_group_path = PathBuf::from("target/criterion").join(benchmark_group);
    if !benchmark_group_path.exists() {
        panic!("benchmark group does not exist");
    }

    let results = collect_results(&benchmark_group_path);
    let consolidated = results
        .into_iter()
        .map(|result| {
            let statistics = calculate_statistics(&result.results);
            ConsolidatedBenchmark {
                name: result.name,
                results: result.results,
                statistics,
            }
        })
        .collect::<Vec<_>>();

    let serialized = serde_json::to_string(&consolidated).expect("cannot serialize");

    let out_path = benchmark_group_path.join("consolidated.json");
    std::fs::write(out_path, serialized).expect("cannot write to file");
}

fn collect_results(benchmark_group_path: &Path) -> Vec<Result> {
    let mut results: Vec<ResultWithLastModifiedTimestamp> = Vec::new();

    struct ResultWithLastModifiedTimestamp {
        last_modified: std::time::SystemTime,
        result: Result,
    }

    let dirs = std::fs::read_dir(&benchmark_group_path).expect("cannot read directory");
    for dir_entry in dirs {
        let dir_entry = dir_entry.expect("cannot read directory entry");
        if dir_entry.file_name() == "report" {
            continue;
        }

        if dir_entry.file_type().expect("cannot get file type").is_file() {
            continue;
        }

        let base_dir = dir_entry.path().join("base");
        let new_dir = dir_entry.path().join("new");
        let relevant_dir = if new_dir.exists() { new_dir } else { base_dir };

        let sample_file = relevant_dir.join("sample.json");
        let sample = std::fs::read_to_string(&sample_file).expect("cannot read sample file");

        let sample: Value = serde_json::from_str(&sample).expect("cannot deserialize sample");
        let sample_obj = sample.as_object().expect("sample is not an object");
        let times: Vec<_> = sample_obj
            .get("times")
            .expect("no times in sample")
            .as_array()
            .expect("times is not an array")
            .iter()
            .map(|v| v.as_f64().expect("time array item not a f64"))
            .collect();
        let iters : Vec<_> = sample_obj
            .get("iters")
            .expect("no iters in sample")
            .as_array()
            .expect("iters is not an array")
            .iter()
            .map(|v| v.as_f64().expect("iters array item not a f64"))
            .collect();

        let normalized_times = times.iter().zip(iters.iter()).map(|(time, iter)| time / iter).collect();

        results.push(ResultWithLastModifiedTimestamp {
            result: Result {
                name: dir_entry
                    .file_name()
                    .to_str()
                    .expect("cannot convert to str")
                    .to_string(),
                results: normalized_times,
            },
            last_modified: sample_file
                .metadata()
                .expect("cannot get metadata")
                .modified()
                .expect("cannot get modified time"),
        })
    }

    results.into_iter()
        .sorted_by(|a, b| a.last_modified.cmp(&b.last_modified))
        .map(|r| r.result)
        .collect()
}

fn calculate_statistics(points: &[f64]) -> Statistics {
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

    Statistics {
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
