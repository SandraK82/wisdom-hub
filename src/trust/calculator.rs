//! Trust score calculation

use crate::models::{Address, TrustScore, HubResult};
use super::TrustPathFinder;

/// Trust score calculator
pub struct TrustCalculator {
    path_finder: TrustPathFinder,
}

impl TrustCalculator {
    /// Create a new trust calculator
    pub fn new(path_finder: TrustPathFinder) -> Self {
        Self { path_finder }
    }

    /// Calculate trust score for an entity from a viewer's perspective
    pub async fn calculate_score(
        &self,
        entity: &Address,
        viewer: &Address,
    ) -> HubResult<TrustScore> {
        // Calculate from viewer's perspective
        if let Some(path) = self.path_finder.find_best_path(viewer, entity).await? {
            return Ok(TrustScore::new(
                entity.clone(),
                viewer.clone(),
                path.effective_trust,
                1,
            ).with_best_path(path));
        }

        // Default: return a neutral score
        Ok(TrustScore::neutral(entity.clone(), viewer.clone()))
    }

    /// Calculate aggregated trust score from multiple sources
    pub async fn calculate_aggregated_score(
        &self,
        entity: &Address,
        viewer: &Address,
    ) -> HubResult<TrustScore> {
        // For now, just use single path calculation
        self.calculate_score(entity, viewer).await
    }
}

impl Default for TrustCalculator {
    fn default() -> Self {
        Self::new(TrustPathFinder::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_self_trust_score() {
        let calculator = TrustCalculator::default();
        let agent = Address::agent("hub:8080", "agent-1");

        let score = calculator
            .calculate_score(&agent, &agent)
            .await
            .unwrap();

        assert_eq!(score.score, 1.0);
        assert_eq!(score.path_count, 1);
        assert!(score.best_path.is_some());
    }

    #[tokio::test]
    async fn test_no_path_score() {
        let calculator = TrustCalculator::default();
        let agent1 = Address::agent("hub:8080", "agent-1");
        let agent2 = Address::agent("hub:8080", "agent-2");

        let score = calculator
            .calculate_score(&agent2, &agent1)
            .await
            .unwrap();

        // No path found, should be neutral
        assert_eq!(score.score, 0.0);
        assert_eq!(score.path_count, 0);
        assert!(score.best_path.is_none());
    }
}
