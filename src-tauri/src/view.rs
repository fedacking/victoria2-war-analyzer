use serde::Serialize;

use crate::war::{Battle, BattleSide, WarCollection, WarData, WarKind};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSavefileView {
    pub path: String,
    pub top_level_statement_count: usize,
    pub active_wars: Vec<WarView>,
    pub previous_wars: Vec<WarView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WarView {
    pub name: String,
    pub kind: WarKindView,
    pub attackers: Vec<String>,
    pub defenders: Vec<String>,
    pub battle_count: usize,
    pub total_losses: f64,
    pub battles: Vec<BattleView>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WarKindView {
    Previous,
    Active,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BattleView {
    pub name: String,
    pub location_id: i64,
    pub location_label: String,
    pub total_losses: f64,
    pub attacker: BattleSideView,
    pub defender: BattleSideView,
    pub unit_breakdown: Vec<UnitBreakdownRowView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BattleSideView {
    pub country: Option<String>,
    pub leader: Option<String>,
    pub losses: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnitBreakdownRowView {
    pub unit_kind: String,
    pub attacker_count: i64,
    pub defender_count: i64,
}

pub fn build_parsed_savefile_view(
    path: String,
    top_level_statement_count: usize,
    wars: &WarCollection,
) -> ParsedSavefileView {
    ParsedSavefileView {
        path,
        top_level_statement_count,
        active_wars: build_war_views(&wars.active_wars),
        previous_wars: build_war_views(&wars.previous_wars),
    }
}

fn build_war_views(wars: &[WarData]) -> Vec<WarView> {
    let mut views: Vec<_> = wars.iter().map(WarView::from).collect();
    views.sort_by(|left, right| {
        right
            .total_losses
            .total_cmp(&left.total_losses)
            .then_with(|| right.battle_count.cmp(&left.battle_count))
            .then_with(|| left.name.cmp(&right.name))
    });
    views
}

impl From<&WarData> for WarView {
    fn from(war: &WarData) -> Self {
        let mut battles: Vec<_> = war.history.battles.iter().map(BattleView::from).collect();
        battles.sort_by(|left, right| {
            right
                .total_losses
                .total_cmp(&left.total_losses)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.location_id.cmp(&right.location_id))
        });

        Self {
            name: war.name.clone(),
            kind: WarKindView::from(&war.kind),
            attackers: participant_list(&war.attackers, &war.original_attacker),
            defenders: participant_list(&war.defenders, &war.original_defender),
            battle_count: battles.len(),
            total_losses: war.total_losses(),
            battles,
        }
    }
}

impl From<&WarKind> for WarKindView {
    fn from(kind: &WarKind) -> Self {
        match kind {
            WarKind::Previous => Self::Previous,
            WarKind::Active => Self::Active,
        }
    }
}

impl From<&Battle> for BattleView {
    fn from(battle: &Battle) -> Self {
        Self {
            name: battle.name.clone(),
            location_id: battle.location,
            location_label: format!("Province #{}", battle.location),
            total_losses: battle.total_losses(),
            attacker: BattleSideView::from(&battle.attacker),
            defender: BattleSideView::from(&battle.defender),
            unit_breakdown: build_unit_breakdown(battle),
        }
    }
}

impl From<&BattleSide> for BattleSideView {
    fn from(side: &BattleSide) -> Self {
        Self {
            country: side.country.clone(),
            leader: side.leader.clone(),
            losses: side.losses_amount(),
        }
    }
}

fn build_unit_breakdown(battle: &Battle) -> Vec<UnitBreakdownRowView> {
    let mut rows: Vec<_> = battle
        .attacker
        .unit_counts
        .keys()
        .chain(battle.defender.unit_counts.keys())
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|unit_kind| UnitBreakdownRowView {
            unit_kind: unit_kind.to_string(),
            attacker_count: battle
                .attacker
                .unit_counts
                .get(unit_kind)
                .copied()
                .unwrap_or_default(),
            defender_count: battle
                .defender
                .unit_counts
                .get(unit_kind)
                .copied()
                .unwrap_or_default(),
        })
        .collect();

    rows.sort_by(|left, right| {
        let left_total = left.attacker_count + left.defender_count;
        let right_total = right.attacker_count + right.defender_count;

        right_total
            .cmp(&left_total)
            .then_with(|| left.unit_kind.cmp(&right.unit_kind))
    });

    rows
}

fn participant_list(participants: &[String], original: &str) -> Vec<String> {
    if participants.is_empty() {
        vec![original.to_string()]
    } else {
        participants.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::{WarKindView, build_parsed_savefile_view};
    use crate::{parser::parse_document, war::extract_wars};

    #[test]
    fn builds_sectioned_sorted_view_and_breaks_units_down_by_side() {
        let document = parse_document(
            r#"
            active_war = {
              name = "Low Loss War"
              history = {
                battle = {
                  name = "Quiet Front"
                  location = 10
                  result = no
                  attacker = {
                    country = ENG
                    infantry = 1000
                    losses = 400
                  }
                  defender = {
                    country = FRA
                    infantry = 800
                    losses = 300
                  }
                }
              }
              attacker = ENG
              defender = FRA
              original_attacker = ENG
              original_defender = FRA
            }
            active_war = {
              name = "High Loss War"
              history = {
                battle = {
                  name = "Smaller Clash"
                  location = 20
                  result = yes
                  attacker = {
                    country = PRU
                    artillery = 300
                    engineer = 50
                    losses = 500
                  }
                  defender = {
                    country = AUS
                    dragoon = 100
                    artillery = 200
                    losses = 300
                  }
                }
                battle = {
                  name = "Grand Battle"
                  location = 30
                  result = yes
                  attacker = {
                    country = PRU
                    infantry = 400
                    artillery = 100
                    losses = 2500
                  }
                  defender = {
                    country = AUS
                    infantry = 500
                    hussar = 200
                    losses = 1500
                  }
                }
              }
              attacker = PRU
              defender = AUS
              original_attacker = PRU
              original_defender = AUS
            }
            previous_war = {
              name = "Older War"
              history = {
                battle = {
                  name = "Legacy Fight"
                  location = 40
                  result = no
                  attacker = {
                    country = RUS
                    infantry = 200
                    losses = 100
                  }
                  defender = {
                    country = TUR
                    infantry = 100
                    losses = 150
                  }
                }
              }
              attacker = RUS
              defender = TUR
              original_attacker = RUS
              original_defender = TUR
            }
            "#,
        )
        .unwrap();

        let wars = extract_wars(&document);
        let view = build_parsed_savefile_view("campaign.v2".to_string(), 3, &wars);

        assert_eq!(view.path, "campaign.v2");
        assert_eq!(view.top_level_statement_count, 3);
        assert_eq!(view.active_wars.len(), 2);
        assert_eq!(view.previous_wars.len(), 1);
        assert_eq!(view.active_wars[0].name, "High Loss War");
        assert_eq!(view.active_wars[0].kind, WarKindView::Active);
        assert_eq!(view.active_wars[0].battle_count, 2);
        assert_eq!(view.active_wars[0].total_losses, 4800.0);
        assert_eq!(view.active_wars[0].battles[0].name, "Grand Battle");
        assert_eq!(
            view.active_wars[0].battles[0].location_label,
            "Province #30"
        );
        assert_eq!(view.active_wars[0].battles[0].total_losses, 4000.0);
        assert_eq!(
            view.active_wars[0].battles[0].attacker.country.as_deref(),
            Some("PRU")
        );
        assert_eq!(view.active_wars[0].battles[0].unit_breakdown.len(), 3);
        assert_eq!(
            view.active_wars[0].battles[0].unit_breakdown[0].unit_kind,
            "infantry"
        );
        assert_eq!(
            view.active_wars[0].battles[0].unit_breakdown[0].attacker_count,
            400
        );
        assert_eq!(
            view.active_wars[0].battles[0].unit_breakdown[0].defender_count,
            500
        );
        assert_eq!(
            view.active_wars[0].battles[0].unit_breakdown[1].unit_kind,
            "hussar"
        );
        assert_eq!(
            view.active_wars[0].battles[0].unit_breakdown[1].attacker_count,
            0
        );
        assert_eq!(
            view.active_wars[0].battles[0].unit_breakdown[1].defender_count,
            200
        );
        assert_eq!(
            view.active_wars[0].battles[0].unit_breakdown[2].unit_kind,
            "artillery"
        );
        assert_eq!(view.active_wars[1].name, "Low Loss War");
        assert_eq!(view.previous_wars[0].kind, WarKindView::Previous);
    }

    #[test]
    fn carries_overflow_corrected_losses_and_handles_missing_values() {
        let document = parse_document(
            r#"
            active_war = {
              name = "Overflow War"
              history = {
                battle = {
                  name = "Huge Clash"
                  location = 1
                  result = yes
                  attacker = {
                    country = ENG
                    losses = -1294967.296
                  }
                  defender = {
                    country = FRA
                  }
                }
                battle = {
                  name = "Aftermath"
                  location = 2
                  result = no
                  attacker = {
                    country = ENG
                  }
                  defender = {
                    country = FRA
                  }
                }
              }
              attacker = ENG
              defender = FRA
              original_attacker = ENG
              original_defender = FRA
            }
            "#,
        )
        .unwrap();

        let wars = extract_wars(&document);
        let view = build_parsed_savefile_view("overflow.v2".to_string(), 1, &wars);
        let war = &view.active_wars[0];

        assert_eq!(war.total_losses, 3000000.0);
        assert_eq!(war.battles[0].name, "Huge Clash");
        assert_eq!(war.battles[0].attacker.losses, Some(3000000.0));
        assert_eq!(war.battles[0].defender.losses, None);
        assert!(war.battles[0].unit_breakdown.is_empty());
        assert_eq!(war.battles[1].total_losses, 0.0);
        assert_eq!(war.battles[1].attacker.losses, None);
        assert!(war.battles[1].unit_breakdown.is_empty());
    }

    #[test]
    fn falls_back_to_original_participants_when_side_lists_are_missing() {
        let document = parse_document(
            r#"
            previous_war = {
              name = "Originals Only"
              history = {
                battle = {
                  name = "Test Battle"
                  location = 9
                  result = yes
                  attacker = {
                    country = ENG
                    losses = 10
                  }
                  defender = {
                    country = FRA
                    losses = 5
                  }
                }
              }
              original_attacker = ENG
              original_defender = FRA
            }
            "#,
        )
        .unwrap();

        let wars = extract_wars(&document);
        let view = build_parsed_savefile_view("originals.v2".to_string(), 1, &wars);

        assert_eq!(view.previous_wars[0].attackers, vec!["ENG".to_string()]);
        assert_eq!(view.previous_wars[0].defenders, vec!["FRA".to_string()]);
    }
}
