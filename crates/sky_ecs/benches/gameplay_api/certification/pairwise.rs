use serde::Serialize;

pub(super) const INITIAL_ROUNDS: usize = 4;
pub(super) const EXTRA_ROUNDS: usize = 8;
pub(super) const CLEAR_WIN_RATIO: f64 = 0.98;
pub(super) const CLEAR_LOSS_RATIO: f64 = 1.02;

#[derive(Debug, Serialize)]
struct PairwiseRound {
    sequence: [&'static str; 4],
    ab_first_ns: f64,
    ab_second_ns: f64,
    ba_second_ns: f64,
    ba_first_ns: f64,
    order_neutral_ratio: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct PairwiseResult {
    pub(super) first: String,
    pub(super) second: String,
    rounds: Vec<PairwiseRound>,
    decision: &'static str,
    winner: String,
    median_order_neutral_ratio: f64,
}

impl PairwiseResult {
    pub(super) fn winner_is(&self, candidate: &str) -> bool {
        self.winner == candidate
    }

    pub(super) fn is_clear(&self) -> bool {
        self.decision == "clear_2_percent_band"
    }
}

pub(super) fn compare_pair<C>(
    first_name: String,
    first: C,
    second_name: String,
    second: C,
    measure: fn(C) -> f64,
) -> PairwiseResult
where
    C: Copy,
{
    let mut rounds = Vec::with_capacity(INITIAL_ROUNDS + EXTRA_ROUNDS);
    append_rounds(&mut rounds, INITIAL_ROUNDS, first, second, measure);
    let mut decision = clear_winner(&rounds);
    if decision.is_none() {
        append_rounds(&mut rounds, EXTRA_ROUNDS, first, second, measure);
        decision = clear_winner(&rounds);
    }

    let mut ratios: Vec<_> = rounds
        .iter()
        .map(|round| round.order_neutral_ratio)
        .collect();
    let median_ratio = median(&mut ratios);
    let (first_wins, decision_name) = match decision {
        Some(first_wins) => (first_wins, "clear_2_percent_band"),
        None => (median_ratio < 1.0, "order_neutral_median_fallback"),
    };

    PairwiseResult {
        first: first_name.clone(),
        second: second_name.clone(),
        rounds,
        decision: decision_name,
        winner: if first_wins { first_name } else { second_name },
        median_order_neutral_ratio: median_ratio,
    }
}

fn append_rounds<C>(
    rounds: &mut Vec<PairwiseRound>,
    count: usize,
    first: C,
    second: C,
    measure: fn(C) -> f64,
) where
    C: Copy,
{
    for _ in 0..count {
        let ab_first = measure(first);
        let ab_second = measure(second);
        let ba_second = measure(second);
        let ba_first = measure(first);
        rounds.push(PairwiseRound {
            sequence: ["first", "second", "second", "first"],
            ab_first_ns: ab_first,
            ab_second_ns: ab_second,
            ba_second_ns: ba_second,
            ba_first_ns: ba_first,
            order_neutral_ratio: ((ab_first / ab_second) * (ba_first / ba_second)).sqrt(),
        });
    }
}

fn clear_winner(rounds: &[PairwiseRound]) -> Option<bool> {
    let first_wins = rounds
        .iter()
        .filter(|round| round.order_neutral_ratio < 1.0)
        .count();
    let second_wins = rounds.len() - first_wins;
    let required_directional_wins = if rounds.len() == INITIAL_ROUNDS {
        INITIAL_ROUNDS
    } else {
        rounds.len().saturating_sub(2)
    };
    let mut ratios: Vec<_> = rounds
        .iter()
        .map(|round| round.order_neutral_ratio)
        .collect();
    let median_ratio = median(&mut ratios);

    if first_wins >= required_directional_wins && median_ratio < CLEAR_WIN_RATIO {
        Some(true)
    } else if second_wins >= required_directional_wins && median_ratio > CLEAR_LOSS_RATIO {
        Some(false)
    } else {
        None
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    }
}
