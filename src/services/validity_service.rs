//! Validity service for checking reasoning chain integrity

use std::collections::HashSet;
use uuid::Uuid;

use crate::models::{Address, Relation, RelationType};

/// The validity status of a reasoning chain
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainValidity {
    /// All premises exist and are not contested
    Valid,
    /// Depends on unverified premises
    Conditional,
    /// At least one element is contested
    Contested,
    /// A premise is missing or was revoked
    Broken,
}

impl std::fmt::Display for ChainValidity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainValidity::Valid => write!(f, "valid"),
            ChainValidity::Conditional => write!(f, "conditional"),
            ChainValidity::Contested => write!(f, "contested"),
            ChainValidity::Broken => write!(f, "broken"),
        }
    }
}

/// Type of validity issue found
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueType {
    /// A DERIVED_FROM reference points to a non-existent fragment
    MissingReference,
    /// A premise has CONTRADICTS relations
    ContestedPremise,
    /// A fragment has low confidence
    LowConfidence,
    /// The creator has low trust score
    UnverifiedSource,
    /// Circular dependency detected
    CircularDependency,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::MissingReference => write!(f, "missing_reference"),
            IssueType::ContestedPremise => write!(f, "contested_premise"),
            IssueType::LowConfidence => write!(f, "low_confidence"),
            IssueType::UnverifiedSource => write!(f, "unverified_source"),
            IssueType::CircularDependency => write!(f, "circular_dependency"),
        }
    }
}

/// A specific validity issue
#[derive(Debug, Clone)]
pub struct ValidityIssue {
    /// The fragment with the issue
    pub fragment_id: String,
    /// Type of issue
    pub issue_type: IssueType,
    /// Human-readable description
    pub description: String,
    /// Severity (0.0 to 1.0)
    pub severity: f32,
}

impl ValidityIssue {
    /// Create a new validity issue
    pub fn new(
        fragment_id: impl Into<String>,
        issue_type: IssueType,
        description: impl Into<String>,
        severity: f32,
    ) -> Self {
        Self {
            fragment_id: fragment_id.into(),
            issue_type,
            description: description.into(),
            severity: severity.clamp(0.0, 1.0),
        }
    }
}

/// Complete validity report for a reasoning chain
#[derive(Debug, Clone)]
pub struct ValidityReport {
    /// Overall validity status
    pub validity: ChainValidity,
    /// All issues found
    pub issues: Vec<ValidityIssue>,
    /// Confidence-weighted validity score (0.0 to 1.0)
    pub confidence_weighted_score: f32,
    /// Number of fragments analyzed
    pub fragments_analyzed: usize,
    /// Number of relations analyzed
    pub relations_analyzed: usize,
}

impl ValidityReport {
    /// Create a new validity report
    pub fn new() -> Self {
        Self {
            validity: ChainValidity::Valid,
            issues: Vec::new(),
            confidence_weighted_score: 1.0,
            fragments_analyzed: 0,
            relations_analyzed: 0,
        }
    }

    /// Add an issue and update validity status
    pub fn add_issue(&mut self, issue: ValidityIssue) {
        // Update validity based on issue type
        let new_validity = match issue.issue_type {
            IssueType::MissingReference => ChainValidity::Broken,
            IssueType::CircularDependency => ChainValidity::Broken,
            IssueType::ContestedPremise => {
                if self.validity != ChainValidity::Broken {
                    ChainValidity::Contested
                } else {
                    self.validity.clone()
                }
            }
            IssueType::LowConfidence | IssueType::UnverifiedSource => {
                if self.validity == ChainValidity::Valid {
                    ChainValidity::Conditional
                } else {
                    self.validity.clone()
                }
            }
        };

        // Only downgrade validity, never upgrade
        if self.validity == ChainValidity::Valid
            || (self.validity == ChainValidity::Conditional
                && new_validity != ChainValidity::Valid)
            || (self.validity == ChainValidity::Contested && new_validity == ChainValidity::Broken)
        {
            self.validity = new_validity;
        }

        // Adjust confidence score
        self.confidence_weighted_score *= 1.0 - (issue.severity * 0.2);
        self.confidence_weighted_score = self.confidence_weighted_score.max(0.0);

        self.issues.push(issue);
    }
}

impl Default for ValidityReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Evidence balance for a thesis
#[derive(Debug, Clone)]
pub struct EvidenceBalance {
    /// The thesis fragment ID
    pub thesis_id: String,
    /// Supporting fragment IDs with confidence
    pub supporting: Vec<(String, f32)>,
    /// Contradicting fragment IDs with confidence
    pub contradicting: Vec<(String, f32)>,
    /// Weighted support score
    pub support_score: f32,
    /// Weighted contradict score
    pub contradict_score: f32,
    /// Net score (support - contradict)
    pub net_score: f32,
}

impl EvidenceBalance {
    /// Create a new evidence balance
    pub fn new(thesis_id: impl Into<String>) -> Self {
        Self {
            thesis_id: thesis_id.into(),
            supporting: Vec::new(),
            contradicting: Vec::new(),
            support_score: 0.0,
            contradict_score: 0.0,
            net_score: 0.0,
        }
    }

    /// Add a supporting fragment
    pub fn add_support(&mut self, fragment_id: impl Into<String>, confidence: f32) {
        self.supporting.push((fragment_id.into(), confidence));
        self.support_score += confidence;
        self.net_score = self.support_score - self.contradict_score;
    }

    /// Add a contradicting fragment
    pub fn add_contradiction(&mut self, fragment_id: impl Into<String>, confidence: f32) {
        self.contradicting.push((fragment_id.into(), confidence));
        self.contradict_score += confidence;
        self.net_score = self.support_score - self.contradict_score;
    }
}

/// Service for checking validity of reasoning chains
pub struct ValidityService {
    /// Minimum confidence threshold for "low confidence" warning
    pub min_confidence_threshold: f32,
    /// Minimum trust score for "unverified source" warning
    pub min_trust_threshold: f32,
}

impl ValidityService {
    /// Create a new validity service with default thresholds
    pub fn new() -> Self {
        Self {
            min_confidence_threshold: 0.3,
            min_trust_threshold: 0.3,
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(min_confidence: f32, min_trust: f32) -> Self {
        Self {
            min_confidence_threshold: min_confidence,
            min_trust_threshold: min_trust,
        }
    }

    /// Analyze DERIVED_FROM relations for a fragment
    /// Returns the IDs of all fragments this one derives from
    pub fn get_derivation_sources(
        &self,
        fragment_id: &str,
        relations: &[Relation],
    ) -> Vec<String> {
        relations
            .iter()
            .filter(|r| {
                r.from.entity == fragment_id && r.relation_type == RelationType::DerivedFrom
            })
            .map(|r| r.to.entity.clone())
            .collect()
    }

    /// Find SUPPORTS relations for a fragment
    pub fn find_supporting_relations<'a>(
        &self,
        fragment_id: &str,
        relations: &'a [Relation],
    ) -> Vec<&'a Relation> {
        relations
            .iter()
            .filter(|r| r.to.entity == fragment_id && r.relation_type == RelationType::Supports)
            .collect()
    }

    /// Find CONTRADICTS relations for a fragment
    pub fn find_contradicting_relations<'a>(
        &self,
        fragment_id: &str,
        relations: &'a [Relation],
    ) -> Vec<&'a Relation> {
        relations
            .iter()
            .filter(|r| {
                r.to.entity == fragment_id && r.relation_type == RelationType::Contradicts
            })
            .collect()
    }

    /// Calculate evidence balance for a thesis
    pub fn calculate_evidence_balance(
        &self,
        thesis_id: &str,
        relations: &[Relation],
    ) -> EvidenceBalance {
        let mut balance = EvidenceBalance::new(thesis_id);

        for relation in relations {
            if relation.to.entity == thesis_id {
                match relation.relation_type {
                    RelationType::Supports => {
                        balance.add_support(&relation.from.entity, relation.confidence);
                    }
                    RelationType::Contradicts => {
                        balance.add_contradiction(&relation.from.entity, relation.confidence);
                    }
                    _ => {}
                }
            }
        }

        balance
    }

    /// Check for circular dependencies in derivation chain
    pub fn check_circular_dependencies(
        &self,
        start_id: &str,
        relations: &[Relation],
    ) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        
        self.dfs_cycle_check(start_id, relations, &mut visited, &mut path)
    }

    fn dfs_cycle_check(
        &self,
        current_id: &str,
        relations: &[Relation],
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if path.contains(&current_id.to_string()) {
            // Found a cycle
            let cycle_start = path.iter().position(|s| s == current_id).unwrap();
            return Some(path[cycle_start..].to_vec());
        }

        if visited.contains(current_id) {
            return None;
        }

        visited.insert(current_id.to_string());
        path.push(current_id.to_string());

        let sources = self.get_derivation_sources(current_id, relations);
        for source_id in sources {
            if let Some(cycle) = self.dfs_cycle_check(&source_id, relations, visited, path) {
                return Some(cycle);
            }
        }

        path.pop();
        None
    }
}

impl Default for ValidityService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Address;

    fn create_test_relation(from: &str, to: &str, rel_type: RelationType) -> Relation {
        Relation::new(
            Address::fragment("hub:8080", from),
            Address::fragment("hub:8080", to),
            Address::agent("hub:8080", "test-agent"),
            rel_type,
        )
    }

    #[test]
    fn test_evidence_balance() {
        let service = ValidityService::new();

        let relations = vec![
            create_test_relation("support1", "thesis", RelationType::Supports)
                .with_confidence(0.8),
            create_test_relation("support2", "thesis", RelationType::Supports)
                .with_confidence(0.9),
            create_test_relation("contra1", "thesis", RelationType::Contradicts)
                .with_confidence(0.5),
        ];

        let balance = service.calculate_evidence_balance("thesis", &relations);

        assert_eq!(balance.supporting.len(), 2);
        assert_eq!(balance.contradicting.len(), 1);
        assert!((balance.support_score - 1.7).abs() < 0.01);
        assert!((balance.contradict_score - 0.5).abs() < 0.01);
        assert!((balance.net_score - 1.2).abs() < 0.01);
    }

    #[test]
    fn test_derivation_sources() {
        let service = ValidityService::new();

        let relations = vec![
            create_test_relation("conclusion", "premise1", RelationType::DerivedFrom),
            create_test_relation("conclusion", "premise2", RelationType::DerivedFrom),
            create_test_relation("other", "premise3", RelationType::DerivedFrom),
        ];

        let sources = service.get_derivation_sources("conclusion", &relations);
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&"premise1".to_string()));
        assert!(sources.contains(&"premise2".to_string()));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let service = ValidityService::new();

        // A -> B -> C -> A (cycle)
        let relations = vec![
            create_test_relation("A", "B", RelationType::DerivedFrom),
            create_test_relation("B", "C", RelationType::DerivedFrom),
            create_test_relation("C", "A", RelationType::DerivedFrom),
        ];

        let cycle = service.check_circular_dependencies("A", &relations);
        assert!(cycle.is_some());
    }

    #[test]
    fn test_no_circular_dependency() {
        let service = ValidityService::new();

        // A -> B -> C (no cycle)
        let relations = vec![
            create_test_relation("A", "B", RelationType::DerivedFrom),
            create_test_relation("B", "C", RelationType::DerivedFrom),
        ];

        let cycle = service.check_circular_dependencies("A", &relations);
        assert!(cycle.is_none());
    }

    #[test]
    fn test_validity_report() {
        let mut report = ValidityReport::new();
        assert_eq!(report.validity, ChainValidity::Valid);

        report.add_issue(ValidityIssue::new(
            "frag1",
            IssueType::LowConfidence,
            "Low confidence",
            0.3,
        ));
        assert_eq!(report.validity, ChainValidity::Conditional);

        report.add_issue(ValidityIssue::new(
            "frag2",
            IssueType::ContestedPremise,
            "Contested",
            0.5,
        ));
        assert_eq!(report.validity, ChainValidity::Contested);

        report.add_issue(ValidityIssue::new(
            "frag3",
            IssueType::MissingReference,
            "Missing",
            1.0,
        ));
        assert_eq!(report.validity, ChainValidity::Broken);
    }
}
