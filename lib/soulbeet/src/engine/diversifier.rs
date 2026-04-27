use std::collections::HashMap;

use rand::seq::SliceRandom;
use tracing::info;

use shared::recommendation::{
    Candidate, CandidateSet, CandidateSnapshot, DiversifierSummary, ProfileConfig, UserMusicProfile,
};

/// Select a diverse final track list from the scored candidate pool.
///
/// Steps:
/// 1. Sort by score descending.
/// 2. Greedy selection with artist cap and soft genre quotas.
/// 3. Backfill exploration candidates if underrepresented.
/// 4. Tier shuffle (groups of 10, shuffled within each tier).
pub fn diversify(
    candidates: CandidateSet,
    profile: &UserMusicProfile,
    config: &ProfileConfig,
    target_count: usize,
) -> (Vec<Candidate>, DiversifierSummary) {
    if candidates.is_empty() {
        return (vec![], DiversifierSummary::default());
    }

    let mut pool: Vec<Candidate> = candidates.into_vec();

    // Step 1: sort by score descending
    pool.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Popularity penalty: penalize candidates from popular artists,
    // scaled by the configured penalty strength
    let mut popularity_penalized = 0usize;
    for candidate in &mut pool {
        if let Some(listeners) = candidate.artist_listener_count {
            // Use 1M listeners as a rough "very popular" threshold
            let popularity = (listeners as f64 / 1_000_000.0).clamp(0.0, 1.0);
            if popularity > 0.5 {
                let penalty = (popularity - 0.5) * 2.0 * config.popularity_penalty_strength;
                candidate.score *= (1.0 - penalty).max(0.1);
                popularity_penalized += 1;
            }
        }
    }

    // Re-sort after popularity penalty
    pool.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Modulate exploration budget by repeat ratio.
    // High uniqueness (close to 1.0) = user likes variety = boost exploration by up to 10%
    // Low uniqueness (close to 0) = user likes familiar = reduce by up to 10%
    let uniqueness_nudge = (profile.repeat_ratio - 0.5) * 0.2;
    let effective_exploration_budget =
        (config.exploration_budget + uniqueness_nudge).clamp(0.05, 0.50);

    // Compute genre target weights, adjusted by exploration budget
    let genre_targets = build_genre_targets(profile, config, effective_exploration_budget);

    // Step 2: greedy selection
    let mut selected: Vec<Candidate> = Vec::with_capacity(target_count);
    let mut artist_counts: HashMap<String, u32> = HashMap::new();
    let mut genre_counts: HashMap<String, u32> = HashMap::new();

    // Pre-compute the top 10% score threshold for genre quota override
    let top_10_pct_idx = pool.len() / 10;
    let top_10_pct_score = if top_10_pct_idx < pool.len() {
        pool[top_10_pct_idx].score
    } else {
        0.0
    };

    // Split pool into exploration and non-exploration for backfill tracking
    let mut exploration_pool: Vec<Candidate> = Vec::new();
    let mut regular_pool: Vec<Candidate> = Vec::new();

    for c in pool {
        let is_exploration = c.signals.iter().any(|s| {
            s.contains("tag_explore")
                || s.contains("genre_explore")
                || s.contains("hop2")
                || s.contains("momentum")
        });
        if is_exploration {
            exploration_pool.push(c);
        } else {
            regular_pool.push(c);
        }
    }

    // Interleave: try regular first, then exploration
    let mut all_sorted: Vec<(Candidate, bool)> = Vec::new();
    for c in regular_pool {
        all_sorted.push((c, false));
    }
    for c in exploration_pool.iter().cloned() {
        all_sorted.push((c, true));
    }
    all_sorted.sort_by(|a, b| {
        b.0.score
            .partial_cmp(&a.0.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut exploration_count = 0usize;
    let mut artist_cap_skipped = 0usize;
    let mut genre_quota_skipped = 0usize;

    for (candidate, is_exploration) in &all_sorted {
        if selected.len() >= target_count {
            break;
        }

        let artist_key = candidate.artist.to_lowercase();

        // Artist cap
        let count = artist_counts.get(&artist_key).copied().unwrap_or(0);
        if count >= config.max_per_artist {
            artist_cap_skipped += 1;
            continue;
        }

        // Soft genre quota: check if any of the candidate's genres exceed target by 1.5x
        if candidate.score < top_10_pct_score
            && exceeds_genre_quota(
                &genre_targets,
                &genre_counts,
                candidate,
                selected.len(),
                target_count,
            )
        {
            genre_quota_skipped += 1;
            continue;
        }

        // Accept
        *artist_counts.entry(artist_key).or_default() += 1;
        update_genre_counts(&mut genre_counts, candidate, profile);
        if *is_exploration {
            exploration_count += 1;
        }
        selected.push(candidate.clone());
    }

    // Step 3: Exploration backfill
    let mut backfill_count = 0usize;
    let target_exploration = (target_count as f64 * effective_exploration_budget).ceil() as usize;
    if exploration_count < target_exploration && selected.len() < target_count {
        let deficit = target_exploration.saturating_sub(exploration_count);
        for c in &exploration_pool {
            if backfill_count >= deficit || selected.len() >= target_count {
                break;
            }
            let key = CandidateSet::key(&c.artist, &c.track);
            if selected
                .iter()
                .any(|s| CandidateSet::key(&s.artist, &s.track) == key)
            {
                continue;
            }
            let artist_key = c.artist.to_lowercase();
            let count = artist_counts.get(&artist_key).copied().unwrap_or(0);
            if count >= config.max_per_artist {
                continue;
            }
            *artist_counts.entry(artist_key).or_default() += 1;
            selected.push(c.clone());
            backfill_count += 1;
        }
        if backfill_count > 0 {
            info!("Backfilled {} exploration candidates", backfill_count);
        }
    }

    // Step 4: Tier shuffle -- group into tiers of 10, shuffle within each
    let mut rng = rand::rng();
    for chunk in selected.chunks_mut(10) {
        chunk.shuffle(&mut rng);
    }

    info!(
        "Diversified to {} tracks ({} exploration, {} artists)",
        selected.len(),
        exploration_count,
        artist_counts.len(),
    );

    let unique_genres = genre_counts.len();
    let unique_artists = artist_counts.len();

    let mut top_by_score = selected.clone();
    top_by_score.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_selected = top_by_score
        .iter()
        .take(5)
        .map(CandidateSnapshot::from_candidate)
        .collect();

    let summary = DiversifierSummary {
        popularity_penalized,
        artist_cap_skipped,
        genre_quota_skipped,
        exploration_backfilled: backfill_count,
        effective_exploration_budget,
        unique_artists,
        unique_genres,
        top_selected,
    };

    (selected, summary)
}

/// Build target genre proportions from the profile, adjusted by exploration budget.
fn build_genre_targets(
    profile: &UserMusicProfile,
    _config: &ProfileConfig,
    effective_exploration_budget: f64,
) -> HashMap<String, f64> {
    let mut targets = HashMap::new();
    if profile.genre_distribution.is_empty() {
        return targets;
    }

    // Comfort genres get (1 - exploration_budget) of the space,
    // proportional to their profile weight.
    let comfort_budget = 1.0 - effective_exploration_budget;

    for tag in &profile.genre_distribution {
        let key = tag.name.to_lowercase();
        let is_comfort = profile.tag_comfort_zone.contains(&key);
        let target = if is_comfort {
            tag.weight * comfort_budget
        } else {
            // Exploration tags share the exploration budget evenly
            effective_exploration_budget / profile.tag_exploration_zone.len().max(1) as f64
        };
        targets.insert(key, target);
    }

    targets
}

/// Check if accepting this candidate would push any of its genres past 1.5x the target.
fn exceeds_genre_quota(
    targets: &HashMap<String, f64>,
    counts: &HashMap<String, u32>,
    candidate: &Candidate,
    selected_so_far: usize,
    _target_count: usize,
) -> bool {
    if targets.is_empty() || selected_so_far == 0 {
        return false;
    }
    if let Some(ref genre) = candidate.primary_genre {
        let genre_key = genre.to_lowercase();
        let target_pct = targets.get(&genre_key).copied().unwrap_or(0.05);
        let current_count = counts.get(&genre_key).copied().unwrap_or(0);
        let current_pct = current_count as f64 / selected_so_far as f64;
        // Exceeds if current representation is 1.5x the target
        if current_pct > target_pct * 1.5 {
            return true;
        }
    }
    false
}

/// Update genre counts after accepting a candidate.
fn update_genre_counts(
    counts: &mut HashMap<String, u32>,
    candidate: &Candidate,
    _profile: &UserMusicProfile,
) {
    if let Some(ref genre) = candidate.primary_genre {
        *counts.entry(genre.to_lowercase()).or_default() += 1;
    }
}
