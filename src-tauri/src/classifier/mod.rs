use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{Arc, RwLock};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::RepoError;

const PUBLISHER_PUBLIC_KEY: &str = "+FHwp7WgyRxxRPRHkre1cpj0ZcA0sr8y4GdT2eUIEZw=";
const MANIFEST_JSON: &str = include_str!("../../resources/classifier/manifest.json");
const DEFINITIONS_JSON: &str = include_str!("../../resources/classifier/definitions.json");
const CORPUS_JSON: &str = include_str!("../../resources/classifier/corpus.json");
const GOLDEN_JSON: &str = include_str!("../../resources/classifier/golden_vectors.json");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetManifest {
    pub schema_version: u32,
    pub classifier_version: String,
    pub effective_date: String,
    pub release_provenance: String,
    pub corpus_digest: String,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DefinitionsFile {
    formats: Vec<FormatDefinition>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatDefinition {
    pub name: String,
    pub k: usize,
    pub min_confidence: f64,
    pub archetypes: Vec<ArchetypeDefinition>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeDefinition {
    pub id: String,
    pub display_name: String,
    pub strict_mode: bool,
    pub signatures: Vec<SignatureConstraint>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureConstraint {
    pub oracle_id: String,
    pub display_name: String,
    pub min_copies: Option<u16>,
    pub exact_copies: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CorpusFile {
    decks: Vec<CorpusDeck>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusDeck {
    pub id: String,
    pub format: String,
    pub archetype_id: String,
    pub cards: BTreeMap<String, u16>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoldenFile {
    vectors: Vec<GoldenVector>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoldenVector {
    id: String,
    format: String,
    complete: bool,
    cards: BTreeMap<String, u16>,
    expected_result_id: String,
    expected_method: ClassificationMethod,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifierAssets {
    pub manifest: AssetManifest,
    pub formats: Vec<FormatDefinition>,
    pub corpus: Vec<CorpusDeck>,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationMethod {
    Signature,
    Knn,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalCard {
    pub oracle_id: String,
    pub quantity: u16,
    #[serde(default)]
    pub basic_land: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteDeck {
    pub format: String,
    pub complete: bool,
    pub cards: Vec<CanonicalCard>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborExplanation {
    pub corpus_id: String,
    pub archetype_id: String,
    pub similarity: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationExplanation {
    pub summary: String,
    pub matched_signature_cards: Vec<String>,
    pub neighbors: Vec<NeighborExplanation>,
    pub decisive_rule: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationResult {
    pub result_id: String,
    pub result_name: String,
    pub classifier_version: String,
    pub classifier_digest: String,
    pub method: ClassificationMethod,
    pub confidence: f64,
    pub explanation: ClassificationExplanation,
}

pub struct DeckClassifier;

impl DeckClassifier {
    pub fn builtin_asset_json() -> (String, String, String, String) {
        (
            MANIFEST_JSON.to_owned(),
            DEFINITIONS_JSON.to_owned(),
            CORPUS_JSON.to_owned(),
            GOLDEN_JSON.to_owned(),
        )
    }

    pub fn load_builtin() -> Result<ClassifierAssets, RepoError> {
        Self::load(AssetSource {
            manifest_json: MANIFEST_JSON,
            definitions_json: DEFINITIONS_JSON,
            corpus_json: CORPUS_JSON,
            golden_json: GOLDEN_JSON,
        })
    }

    pub fn load(source: AssetSource<'_>) -> Result<ClassifierAssets, RepoError> {
        let manifest: AssetManifest =
            serde_json::from_str(source.manifest_json).map_err(|_| RepoError::AssetsInvalid)?;
        let definitions: DefinitionsFile =
            serde_json::from_str(source.definitions_json).map_err(|_| RepoError::AssetsInvalid)?;
        let corpus: CorpusFile =
            serde_json::from_str(source.corpus_json).map_err(|_| RepoError::AssetsInvalid)?;
        let golden: GoldenFile =
            serde_json::from_str(source.golden_json).map_err(|_| RepoError::AssetsInvalid)?;

        validate_schema(&manifest, &definitions, &corpus)?;
        let corpus_digest = format!("sha256:{}", sha256(source.corpus_json.as_bytes()));
        if manifest.corpus_digest != corpus_digest {
            return Err(RepoError::AssetsInvalid);
        }
        verify_asset_signature(
            &manifest.signature,
            source.definitions_json,
            source.corpus_json,
            source.golden_json,
        )?;
        let digest = format!(
            "sha256:{}",
            sha256(
                [
                    source.manifest_json,
                    source.definitions_json,
                    source.corpus_json,
                    source.golden_json,
                ]
                .join("\n")
                .as_bytes(),
            )
        );
        let assets = ClassifierAssets {
            manifest,
            formats: definitions.formats,
            corpus: corpus.decks,
            digest,
        };
        for vector in golden.vectors {
            let deck = CompleteDeck {
                format: vector.format,
                complete: vector.complete,
                cards: vector
                    .cards
                    .into_iter()
                    .map(|(oracle_id, quantity)| CanonicalCard {
                        oracle_id,
                        quantity,
                        basic_land: false,
                    })
                    .collect(),
            };
            let result = Self::classify(&deck, &assets)?;
            if result.result_id != vector.expected_result_id
                || result.method != vector.expected_method
            {
                return Err(RepoError::AssetsInvalid);
            }
        }
        Ok(assets)
    }

    pub fn classify(
        deck: &CompleteDeck,
        assets: &ClassifierAssets,
    ) -> Result<ClassificationResult, RepoError> {
        if !deck.complete {
            return Err(RepoError::DeckIncomplete);
        }
        let format = assets
            .formats
            .iter()
            .find(|definition| definition.name.eq_ignore_ascii_case(&deck.format))
            .ok_or(RepoError::FormatUnsupported)?;
        let vector = canonical_vector(deck);

        let mut matching = format
            .archetypes
            .iter()
            .enumerate()
            .filter(|(_, archetype)| signature_matches(archetype, &vector))
            .map(|(order, archetype)| (signature_specificity(archetype), order, archetype))
            .collect::<Vec<_>>();
        matching.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        if let Some((_, _, archetype)) = matching.first() {
            let matched_signature_cards = archetype
                .signatures
                .iter()
                .map(|constraint| constraint.display_name.clone())
                .collect::<Vec<_>>();
            return Ok(ClassificationResult {
                result_id: archetype.id.clone(),
                result_name: archetype.display_name.clone(),
                classifier_version: assets.manifest.classifier_version.clone(),
                classifier_digest: assets.digest.clone(),
                method: ClassificationMethod::Signature,
                confidence: 1.0,
                explanation: ClassificationExplanation {
                    summary: format!(
                        "Matched all {} signature constraints for {}.",
                        archetype.signatures.len(),
                        archetype.display_name
                    ),
                    matched_signature_cards,
                    neighbors: Vec::new(),
                    decisive_rule: Some(archetype.id.clone()),
                },
            });
        }

        let strict_ids = format
            .archetypes
            .iter()
            .filter(|archetype| archetype.strict_mode)
            .map(|archetype| archetype.id.as_str())
            .collect::<HashSet<_>>();
        let mut neighbors = assets
            .corpus
            .iter()
            .filter(|entry| entry.format.eq_ignore_ascii_case(&format.name))
            .filter(|entry| !strict_ids.contains(entry.archetype_id.as_str()))
            .map(|entry| NeighborExplanation {
                corpus_id: entry.id.clone(),
                archetype_id: entry.archetype_id.clone(),
                similarity: cosine_similarity(&vector, &entry.cards),
            })
            .collect::<Vec<_>>();
        neighbors.sort_by(|left, right| {
            right
                .similarity
                .total_cmp(&left.similarity)
                .then_with(|| left.corpus_id.cmp(&right.corpus_id))
        });
        neighbors.truncate(format.k);

        let mut weights = BTreeMap::<String, f64>::new();
        let mut total = 0.0;
        for neighbor in &neighbors {
            *weights.entry(neighbor.archetype_id.clone()).or_default() += neighbor.similarity;
            total += neighbor.similarity;
        }
        let archetype_order = format
            .archetypes
            .iter()
            .enumerate()
            .map(|(order, archetype)| (archetype.id.as_str(), order))
            .collect::<BTreeMap<_, _>>();
        let winner = weights.iter().max_by(|left, right| {
            left.1.total_cmp(right.1).then_with(|| {
                let left_order = archetype_order
                    .get(left.0.as_str())
                    .copied()
                    .unwrap_or(usize::MAX);
                let right_order = archetype_order
                    .get(right.0.as_str())
                    .copied()
                    .unwrap_or(usize::MAX);
                right_order.cmp(&left_order)
            })
        });
        let confidence = winner
            .map(|(_, weight)| if total > 0.0 { *weight / total } else { 0.0 })
            .unwrap_or(0.0);
        let winner_id = winner.map(|(id, _)| id.as_str());
        let accepted = confidence >= format.min_confidence;
        let winner_definition = winner_id.and_then(|id| {
            format
                .archetypes
                .iter()
                .find(|archetype| archetype.id == id)
        });
        let (result_id, result_name) = if accepted {
            winner_definition
                .map(|archetype| (archetype.id.clone(), archetype.display_name.clone()))
                .unwrap_or_else(unclassified_label)
        } else {
            unclassified_label()
        };
        Ok(ClassificationResult {
            result_id,
            result_name,
            classifier_version: assets.manifest.classifier_version.clone(),
            classifier_digest: assets.digest.clone(),
            method: ClassificationMethod::Knn,
            confidence,
            explanation: ClassificationExplanation {
                summary: if accepted {
                    format!(
                        "Top {} local neighbors produced {:.3} confidence.",
                        neighbors.len(),
                        confidence
                    )
                } else {
                    format!(
                        "Local neighbor confidence {:.3} is below the {:.3} threshold.",
                        confidence, format.min_confidence
                    )
                },
                matched_signature_cards: Vec::new(),
                neighbors,
                decisive_rule: winner_id.map(ToOwned::to_owned),
            },
        })
    }

    pub fn classify_confirmable(
        deck: &CompleteDeck,
        assets: &ClassifierAssets,
    ) -> Result<ClassificationResult, RepoError> {
        match Self::classify(deck, assets) {
            Err(RepoError::FormatUnsupported) => Ok(ClassificationResult {
                result_id: "unclassified".into(),
                result_name: "Unclassified".into(),
                classifier_version: assets.manifest.classifier_version.clone(),
                classifier_digest: assets.digest.clone(),
                method: ClassificationMethod::Unsupported,
                confidence: 0.0,
                explanation: ClassificationExplanation {
                    summary: format!("No bundled classifier coverage for {}.", deck.format),
                    matched_signature_cards: Vec::new(),
                    neighbors: Vec::new(),
                    decisive_rule: None,
                },
            }),
            result => result,
        }
    }
}

#[derive(Clone, Copy)]
pub struct AssetSource<'a> {
    pub manifest_json: &'a str,
    pub definitions_json: &'a str,
    pub corpus_json: &'a str,
    pub golden_json: &'a str,
}

#[derive(Clone)]
pub struct AssetRegistry {
    active: Arc<RwLock<ClassifierAssets>>,
}

impl AssetRegistry {
    pub fn builtin() -> Result<Self, RepoError> {
        Ok(Self {
            active: Arc::new(RwLock::new(DeckClassifier::load_builtin()?)),
        })
    }

    pub fn current(&self) -> Result<ClassifierAssets, RepoError> {
        self.active
            .read()
            .map_err(|_| RepoError::AssetsInvalid)
            .map(|assets| assets.clone())
    }

    pub fn activate(&self, source: AssetSource<'_>) -> Result<ClassifierAssets, RepoError> {
        let candidate = DeckClassifier::load(source)?;
        let mut active = self.active.write().map_err(|_| RepoError::AssetsInvalid)?;
        *active = candidate.clone();
        Ok(candidate)
    }
}

pub fn canonical_vector(deck: &CompleteDeck) -> BTreeMap<String, u16> {
    let mut vector = BTreeMap::<String, (u16, bool)>::new();
    for card in &deck.cards {
        let entry = vector
            .entry(card.oracle_id.clone())
            .or_insert((0, card.basic_land));
        entry.0 = entry.0.saturating_add(card.quantity);
        entry.1 |= card.basic_land;
    }
    vector
        .into_iter()
        .map(|(oracle_id, (quantity, basic_land))| {
            (
                oracle_id,
                if basic_land {
                    quantity
                } else {
                    quantity.min(4)
                },
            )
        })
        .collect()
}

fn signature_matches(archetype: &ArchetypeDefinition, vector: &BTreeMap<String, u16>) -> bool {
    !archetype.signatures.is_empty()
        && archetype.signatures.iter().all(|constraint| {
            let copies = vector.get(&constraint.oracle_id).copied().unwrap_or(0);
            match (constraint.exact_copies, constraint.min_copies) {
                (Some(exact), _) => copies == exact,
                (None, Some(minimum)) => copies >= minimum,
                (None, None) => copies >= 1,
            }
        })
}

fn signature_specificity(archetype: &ArchetypeDefinition) -> usize {
    archetype.signatures.len() * 2
        + archetype
            .signatures
            .iter()
            .filter(|constraint| constraint.exact_copies.is_some())
            .count()
}

fn cosine_similarity(left: &BTreeMap<String, u16>, right: &BTreeMap<String, u16>) -> f64 {
    let dot = left
        .iter()
        .map(|(card, count)| f64::from(*count) * f64::from(*right.get(card).unwrap_or(&0)))
        .sum::<f64>();
    let left_norm = left
        .values()
        .map(|count| f64::from(*count).powi(2))
        .sum::<f64>()
        .sqrt();
    let right_norm = right
        .values()
        .map(|count| f64::from(*count).powi(2))
        .sum::<f64>()
        .sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm * right_norm)
    }
}

fn validate_schema(
    manifest: &AssetManifest,
    definitions: &DefinitionsFile,
    corpus: &CorpusFile,
) -> Result<(), RepoError> {
    if manifest.schema_version != 1
        || manifest.classifier_version.trim().is_empty()
        || manifest.effective_date.trim().is_empty()
        || definitions.formats.is_empty()
    {
        return Err(RepoError::AssetsInvalid);
    }
    let mut format_ids = BTreeSet::new();
    let mut archetype_ids = BTreeSet::new();
    for format in &definitions.formats {
        if format.name.trim().is_empty()
            || format.k == 0
            || !(0.0..=1.0).contains(&format.min_confidence)
            || !format_ids.insert(format.name.to_ascii_lowercase())
        {
            return Err(RepoError::AssetsInvalid);
        }
        for archetype in &format.archetypes {
            if archetype.id.trim().is_empty()
                || archetype.display_name.trim().is_empty()
                || archetype.signatures.is_empty()
                || !archetype_ids.insert(archetype.id.clone())
                || archetype.signatures.iter().any(|constraint| {
                    constraint.oracle_id.trim().is_empty()
                        || constraint.display_name.trim().is_empty()
                        || (constraint.min_copies.is_some() && constraint.exact_copies.is_some())
                })
            {
                return Err(RepoError::AssetsInvalid);
            }
        }
    }
    let mut corpus_ids = BTreeSet::new();
    if corpus.decks.iter().any(|deck| {
        deck.id.trim().is_empty()
            || deck.cards.is_empty()
            || !corpus_ids.insert(deck.id.clone())
            || !archetype_ids.contains(&deck.archetype_id)
            || !format_ids.contains(&deck.format.to_ascii_lowercase())
    }) {
        return Err(RepoError::AssetsInvalid);
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn verify_asset_signature(
    encoded_signature: &str,
    definitions: &str,
    corpus: &str,
    golden: &str,
) -> Result<(), RepoError> {
    let public_key = STANDARD
        .decode(PUBLISHER_PUBLIC_KEY)
        .map_err(|_| RepoError::AssetsInvalid)?;
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| RepoError::AssetsInvalid)?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key).map_err(|_| RepoError::AssetsInvalid)?;
    let signature = encoded_signature
        .strip_prefix("ed25519:")
        .ok_or(RepoError::AssetsInvalid)
        .and_then(|encoded| {
            STANDARD
                .decode(encoded)
                .map_err(|_| RepoError::AssetsInvalid)
        })
        .and_then(|bytes| Signature::from_slice(&bytes).map_err(|_| RepoError::AssetsInvalid))?;
    verifying_key
        .verify(
            [definitions, corpus, golden].join("\n").as_bytes(),
            &signature,
        )
        .map_err(|_| RepoError::AssetsInvalid)
}

fn unclassified_label() -> (String, String) {
    ("unclassified".into(), "Unclassified".into())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn deck(cards: &[(&str, u16)]) -> CompleteDeck {
        CompleteDeck {
            format: "Modern".into(),
            complete: true,
            cards: cards
                .iter()
                .map(|(oracle_id, quantity)| CanonicalCard {
                    oracle_id: (*oracle_id).into(),
                    quantity: *quantity,
                    basic_land: false,
                })
                .collect(),
        }
    }

    fn builtin() -> ClassifierAssets {
        DeckClassifier::load_builtin().expect("valid bundled assets")
    }

    #[test]
    fn ut_054_complete_signature_match_uses_signature_method() {
        let result = DeckClassifier::classify(
            &deck(&[
                ("oracle-murktide-regent", 2),
                ("oracle-dragons-rage-channeler", 4),
            ]),
            &builtin(),
        )
        .expect("classification");
        assert_eq!(result.result_id, "izzet_murktide");
        assert_eq!(result.method, ClassificationMethod::Signature);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn ut_055_omitted_copy_constraint_defaults_to_one() {
        let assets = builtin();
        let result = DeckClassifier::classify(
            &deck(&[("oracle-goblin-guide", 1), ("oracle-lightning-bolt", 4)]),
            &assets,
        )
        .expect("classification");
        assert_eq!(result.result_id, "burn");
        assert_eq!(result.method, ClassificationMethod::Signature);
    }

    #[test]
    fn ut_056_exact_zero_is_distinct_from_omitted_constraint() {
        let assets = builtin();
        let matching = DeckClassifier::classify(&deck(&[("oracle-living-end", 3)]), &assets)
            .expect("matching");
        assert_eq!(matching.result_id, "living_end");
        let not_matching = DeckClassifier::classify(
            &deck(&[("oracle-living-end", 3), ("oracle-crashing-footfalls", 1)]),
            &assets,
        )
        .expect("fallback");
        assert_ne!(not_matching.result_id, "living_end");
    }

    #[test]
    fn ut_057_and_it_182_multiple_matches_use_specificity_then_asset_order() {
        let result = DeckClassifier::classify(
            &deck(&[
                ("oracle-murktide-regent", 2),
                ("oracle-dragons-rage-channeler", 4),
                ("oracle-goblin-guide", 1),
                ("oracle-lightning-bolt", 4),
            ]),
            &builtin(),
        )
        .expect("classification");
        assert_eq!(result.result_id, "izzet_murktide");
        assert_eq!(
            result.explanation.decisive_rule.as_deref(),
            Some("izzet_murktide")
        );
    }

    #[test]
    fn ut_058_strict_labels_are_excluded_from_knn_voting() {
        let mut assets = builtin();
        assets.corpus.push(CorpusDeck {
            id: "strict-neighbor".into(),
            format: "Modern".into(),
            archetype_id: "living_end".into(),
            cards: BTreeMap::from([("oracle-strict-only".into(), 4)]),
        });
        let result = DeckClassifier::classify(&deck(&[("oracle-strict-only", 4)]), &assets)
            .expect("classification");
        assert_ne!(result.result_id, "living_end");
        assert!(
            result
                .explanation
                .neighbors
                .iter()
                .all(|neighbor| neighbor.archetype_id != "living_end")
        );
    }

    #[test]
    fn ut_059_knn_selects_top_five_in_deterministic_order() {
        let result = DeckClassifier::classify(
            &deck(&[
                ("oracle-monastery-swiftspear", 4),
                ("oracle-lava-spike", 4),
                ("oracle-boros-charm", 4),
            ]),
            &builtin(),
        )
        .expect("classification");
        assert_eq!(result.method, ClassificationMethod::Knn);
        assert_eq!(result.explanation.neighbors.len(), 5);
        assert!(result.explanation.neighbors.windows(2).all(|pair| {
            pair[0].similarity > pair[1].similarity
                || (pair[0].similarity == pair[1].similarity
                    && pair[0].corpus_id < pair[1].corpus_id)
        }));
    }

    #[test]
    fn ut_060_confidence_at_threshold_is_accepted_and_next_value_is_not() {
        let input = deck(&[("oracle-monastery-swiftspear", 4), ("oracle-lava-spike", 4)]);
        let mut assets = builtin();
        assets.formats[0].min_confidence = 0.0;
        let baseline = DeckClassifier::classify(&input, &assets).expect("baseline");
        assert!(baseline.confidence > 0.0);
        assets.formats[0].min_confidence = baseline.confidence;
        assert_ne!(
            DeckClassifier::classify(&input, &assets)
                .expect("threshold")
                .result_id,
            "unclassified"
        );
        assets.formats[0].min_confidence = f64::from_bits(baseline.confidence.to_bits() + 1);
        assert_eq!(
            DeckClassifier::classify(&input, &assets)
                .expect("below")
                .result_id,
            "unclassified"
        );
    }

    #[test]
    fn ut_061_equal_weighted_votes_resolve_by_declared_order() {
        let mut assets = builtin();
        assets.formats[0].k = 2;
        assets.formats[0].min_confidence = 0.0;
        assets.corpus = vec![
            CorpusDeck {
                id: "burn-tie".into(),
                format: "Modern".into(),
                archetype_id: "burn".into(),
                cards: BTreeMap::from([("oracle-tie".into(), 4)]),
            },
            CorpusDeck {
                id: "hammer-tie".into(),
                format: "Modern".into(),
                archetype_id: "hammer_time".into(),
                cards: BTreeMap::from([("oracle-tie".into(), 4)]),
            },
        ];
        let result = DeckClassifier::classify(&deck(&[("oracle-tie", 4)]), &assets).expect("tie");
        assert_eq!(result.result_id, "burn");
    }

    #[test]
    fn ut_062_partial_deck_is_rejected_before_vectorization() {
        let mut partial = deck(&[("oracle-card", 4)]);
        partial.complete = false;
        assert_eq!(
            DeckClassifier::classify(&partial, &builtin()),
            Err(RepoError::DeckIncomplete)
        );
    }

    #[test]
    fn ut_063_unknown_format_does_not_load_another_corpus() {
        let mut unknown = deck(&[("oracle-lightning-bolt", 4)]);
        unknown.format = "Pauper".into();
        assert_eq!(
            DeckClassifier::classify(&unknown, &builtin()),
            Err(RepoError::FormatUnsupported)
        );
    }

    #[test]
    fn ut_064_invalid_signature_schema_duplicate_or_digest_rejects_bundle() {
        let mut bad_manifest =
            MANIFEST_JSON.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2");
        assert_eq!(
            DeckClassifier::load(AssetSource {
                manifest_json: &bad_manifest,
                definitions_json: DEFINITIONS_JSON,
                corpus_json: CORPUS_JSON,
                golden_json: GOLDEN_JSON,
            }),
            Err(RepoError::AssetsInvalid)
        );
        bad_manifest = MANIFEST_JSON.replace(
            "dEhh9X+99qcS+WPjPwHmT3gbBd26rTXKKbTt6LY2H4Ues1tQhKhI+ru2d27DagSlQ/RkPm3BLlvIYQzey+a2Aw==",
            "AEhh9X+99qcS+WPjPwHmT3gbBd26rTXKKbTt6LY2H4Ues1tQhKhI+ru2d27DagSlQ/RkPm3BLlvIYQzey+a2Aw==",
        );
        assert_eq!(
            DeckClassifier::load(AssetSource {
                manifest_json: &bad_manifest,
                definitions_json: DEFINITIONS_JSON,
                corpus_json: CORPUS_JSON,
                golden_json: GOLDEN_JSON,
            }),
            Err(RepoError::AssetsInvalid)
        );
        let duplicate = CORPUS_JSON.replace("\"burn-002\"", "\"burn-001\"");
        assert_eq!(
            DeckClassifier::load(AssetSource {
                manifest_json: MANIFEST_JSON,
                definitions_json: DEFINITIONS_JSON,
                corpus_json: &duplicate,
                golden_json: GOLDEN_JSON,
            }),
            Err(RepoError::AssetsInvalid)
        );
    }

    #[test]
    fn ut_065_same_revision_and_assets_are_byte_equivalent() {
        let assets = builtin();
        let input = deck(&[("oracle-monastery-swiftspear", 4)]);
        let left = DeckClassifier::classify(&input, &assets).expect("left");
        let right = DeckClassifier::classify(&input, &assets).expect("right");
        assert_eq!(
            serde_json::to_vec(&left).expect("left bytes"),
            serde_json::to_vec(&right).expect("right bytes")
        );
    }

    #[test]
    fn ut_067_one_hundred_card_classification_meets_budget() {
        let input = CompleteDeck {
            format: "Modern".into(),
            complete: true,
            cards: (0..100)
                .map(|index| CanonicalCard {
                    oracle_id: format!("oracle-{index:03}"),
                    quantity: 1,
                    basic_land: false,
                })
                .collect(),
        };
        let started = Instant::now();
        DeckClassifier::classify(&input, &builtin()).expect("classification");
        assert!(started.elapsed() < Duration::from_millis(250));
    }

    #[test]
    fn it_181_partial_malformed_and_unknown_have_explicit_non_run_outcomes() {
        let assets = builtin();
        let mut partial = deck(&[]);
        partial.complete = false;
        assert_eq!(
            DeckClassifier::classify(&partial, &assets),
            Err(RepoError::DeckIncomplete)
        );
        let malformed = deck(&[]);
        assert_eq!(
            DeckClassifier::classify(&malformed, &assets)
                .expect("unclassified")
                .result_id,
            "unclassified"
        );
        let mut unknown = deck(&[]);
        unknown.format = "Unknown".into();
        assert_eq!(
            DeckClassifier::classify_confirmable(&unknown, &assets)
                .expect("confirmable")
                .method,
            ClassificationMethod::Unsupported
        );
    }

    #[test]
    fn it_183_exact_zero_exact_count_and_default_min_are_distinct() {
        ut_055_omitted_copy_constraint_defaults_to_one();
        ut_056_exact_zero_is_distinct_from_omitted_constraint();
        let result = DeckClassifier::classify(
            &deck(&[("oracle-living-end", 2), ("oracle-crashing-footfalls", 0)]),
            &builtin(),
        )
        .expect("fallback");
        assert_ne!(result.result_id, "living_end");
    }

    #[test]
    fn it_184_tie_is_stable_and_low_confidence_is_unclassified() {
        ut_061_equal_weighted_votes_resolve_by_declared_order();
        let mut assets = builtin();
        assets.formats[0].min_confidence = 1.0;
        let result =
            DeckClassifier::classify(&deck(&[("oracle-unknown", 4)]), &assets).expect("result");
        assert_eq!(result.result_id, "unclassified");
    }

    #[test]
    fn it_185_invalid_assets_leave_last_valid_active() {
        let registry = AssetRegistry::builtin().expect("registry");
        let original = registry.current().expect("current");
        let tampered = CORPUS_JSON.replace("burn-001", "tampered");
        assert_eq!(
            registry.activate(AssetSource {
                manifest_json: MANIFEST_JSON,
                definitions_json: DEFINITIONS_JSON,
                corpus_json: &tampered,
                golden_json: GOLDEN_JSON,
            }),
            Err(RepoError::AssetsInvalid)
        );
        assert_eq!(
            registry.current().expect("still active").digest,
            original.digest
        );
    }
}
