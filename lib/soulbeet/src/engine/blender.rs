use std::collections::HashSet;

use tracing::info;

use shared::recommendation::{BlendSummary, CandidateSet, CandidateSnapshot, ProfileConfig};

/// Default blend weights when both sources are present.
/// Both have 4 working signals each. Last.fm gets slightly higher weight
/// because its tag data and similarity scores are generally richer.
const LB_WEIGHT: f64 = 0.45;
const LFM_WEIGHT: f64 = 0.55;

/// Merge candidate sets from multiple generator sources.
///
/// When a single source is present, its candidates pass through unchanged.
/// When both Last.fm and ListenBrainz candidates are present, scores are
/// normalized per-source, weighted, and cross-source overlap gets a bonus.
pub fn blend(
    sources: Vec<(&str, CandidateSet)>,
    config: &ProfileConfig,
) -> (CandidateSet, BlendSummary) {
    let source_count = sources.len();

    if sources.is_empty() {
        return (CandidateSet::new(), BlendSummary::default());
    }

    if sources.len() == 1 {
        let (name, set) = sources.into_iter().next().unwrap();
        info!(
            "Single source '{}', passing through {} candidates",
            name,
            set.len()
        );
        let total = set.len();
        let mut sorted: Vec<_> = set.candidates.values().collect();
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top_blended = sorted
            .iter()
            .take(5)
            .map(|c| CandidateSnapshot::from_candidate(c))
            .collect();
        let summary = BlendSummary {
            sources: 1,
            total_after_blend: total,
            cross_source_matches: 0,
            top_blended,
        };
        return (set, summary);
    }

    info!("Blending {} sources", sources.len());

    // Normalize each source to [0, 1] range and apply source weight.
    let mut all_keys: HashSet<String> = HashSet::new();
    let mut normalized_sources: Vec<(&str, CandidateSet)> = Vec::new();

    for (name, mut set) in sources {
        let max = set.max_score();
        if max > 0.0 {
            for candidate in set.candidates.values_mut() {
                candidate.score /= max;

                // Apply source-specific weight
                let weight = source_weight(name);
                candidate.score *= weight;
            }
        }
        for key in set.candidates.keys() {
            all_keys.insert(key.clone());
        }
        normalized_sources.push((name, set));
    }

    // Merge into a single set. Candidates appearing in multiple sources get
    // a cross-source bonus.
    let mut merged = CandidateSet::new();

    for key in &all_keys {
        let mut appearances = Vec::new();
        for (name, set) in &normalized_sources {
            if let Some(candidate) = set.candidates.get(key) {
                appearances.push((*name, candidate));
            }
        }

        if appearances.len() > 1 {
            // Cross-source: take the first as base, accumulate scores and signals
            let (_, first) = appearances[0];
            let mut merged_candidate = first.clone();

            for (_, other) in &appearances[1..] {
                merged_candidate.score += other.score;
                for signal in &other.signals {
                    if !merged_candidate.signals.contains(signal) {
                        merged_candidate.signals.push(signal.clone());
                    }
                }
            }

            // Apply cross-source bonus
            merged_candidate.score *= config.cross_source_bonus;

            // Signal diversity bonus: 1.0 + 0.1 * ln(unique_signal_count)
            let unique_signals = merged_candidate.signals.len() as f64;
            if unique_signals > 1.0 {
                merged_candidate.score *= 1.0 + 0.1 * unique_signals.ln();
            }

            merged_candidate.source = "blended".to_string();
            merged.candidates.insert(key.clone(), merged_candidate);
        } else {
            // Single-source candidate
            let (_, candidate) = appearances[0];
            merged.candidates.insert(key.clone(), candidate.clone());
        }
    }

    let cross_source_matches = merged
        .candidates
        .values()
        .filter(|c| c.source == "blended")
        .count();

    info!(
        "Blended result: {} candidates ({} cross-source)",
        merged.len(),
        cross_source_matches
    );

    let mut sorted: Vec<_> = merged.candidates.values().collect();
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_blended = sorted
        .iter()
        .take(5)
        .map(|c| CandidateSnapshot::from_candidate(c))
        .collect();

    let summary = BlendSummary {
        sources: source_count,
        total_after_blend: merged.len(),
        cross_source_matches,
        top_blended,
    };

    (merged, summary)
}

fn source_weight(name: &str) -> f64 {
    match name {
        "listenbrainz" | "listenbrainz_pipeline" => LB_WEIGHT,
        "lastfm" | "lastfm_pipeline" => LFM_WEIGHT,
        _ => 0.5,
    }
}
