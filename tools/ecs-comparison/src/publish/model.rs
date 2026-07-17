use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct RunEstimate {
    pub(super) run: usize,
    pub(super) order: String,
    pub(super) point_ns: f64,
    pub(super) lower_ns: f64,
    pub(super) upper_ns: f64,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Summary {
    pub(super) benchmark: String,
    #[serde(default)]
    pub(super) class: String,
    pub(super) median_ns: f64,
    pub(super) work_items: Option<usize>,
    pub(super) ns_per_item: Option<f64>,
    pub(super) items_per_second: Option<f64>,
    pub(super) run_spread_percent: f64,
    pub(super) noisy: bool,
    pub(super) runs: Vec<RunEstimate>,
}

#[derive(Serialize)]
pub(super) struct PositionBias {
    pub(super) position: usize,
    pub(super) sample_count: usize,
    pub(super) median_ratio: f64,
}

#[derive(Serialize)]
pub(super) struct OrderBias {
    pub(super) positions: Vec<PositionBias>,
    pub(super) max_deviation_percent: f64,
    pub(super) spread_percent: f64,
    pub(super) complete: bool,
    pub(super) noisy: bool,
}

#[derive(Serialize)]
pub(super) struct PublicationReport<'a> {
    pub(super) criterion_estimator: &'static str,
    pub(super) run_count: usize,
    pub(super) order_bias: &'a OrderBias,
    pub(super) benchmarks: &'a [Summary],
}

#[derive(Deserialize)]
pub(super) struct StoredPublicationReport {
    pub(super) run_count: usize,
    pub(super) benchmarks: Vec<Summary>,
}
