use std::collections::HashSet;

use chrono::Datelike;
use tracing::info;

use shared::recommendation::{CandidateSet, FreshnessSummary, ProfileConfig, UserMusicProfile};

/// Apply freshness adjustments to candidate scores.
///
/// Two mechanisms:
/// 1. Known-artist penalty: candidates from artists the user already listens to
///    get a score reduction scaled by the user's freshness half-life.
/// 2. New-release boost: candidates released recently get a score bump.
pub fn apply_freshness(
    candidates: &mut CandidateSet,
    profile: &UserMusicProfile,
    known_artists: &HashSet<String>,
    config: &ProfileConfig,
) -> FreshnessSummary {
    let mut penalized = 0usize;
    let mut boosted = 0usize;

    // Scale penalty by freshness half-life:
    // Short half-life (user gets bored fast) = stronger penalty (0.6x)
    // Long half-life (user revisits) = lighter penalty (0.9x)
    let half_life = profile.freshness_half_life_days.clamp(7.0, 365.0);
    let penalty = 0.6 + 0.3 * (half_life / 365.0); // ranges from 0.6 to 0.9

    // New release boost: candidates with recent release years get a score boost
    let current_year = chrono::Utc::now().year() as u16;

    for candidate in candidates.candidates.values_mut() {
        // Known-artist penalty
        let artist_key = candidate.artist.to_lowercase();
        if known_artists.contains(&artist_key) {
            candidate.score *= penalty;
            penalized += 1;
        }

        // New-release boost
        if let Some(year) = candidate.release_year {
            let age = current_year.saturating_sub(year);
            if age == 0 {
                candidate.score *= config.new_release_boost;
                boosted += 1;
            } else if age <= 1 {
                candidate.score *= 1.0 + (config.new_release_boost - 1.0) * 0.5;
                boosted += 1;
            }
        }
    }

    info!(
        "Freshness pass: penalized {} known-artist candidates (penalty={:.2}), boosted {} new releases",
        penalized,
        penalty,
        boosted,
    );

    FreshnessSummary {
        known_artists_penalized: penalized,
        total_candidates: candidates.len(),
        penalty_factor: penalty,
        new_release_boosted: boosted,
    }
}
