use serde::Serialize;

use crate::{
    parser::{Block, Value},
    war::{Battle, BattleSide, SaveDate, WarCollection, WarData, WarKind, parse_battle_from_value},
};

const MIN_WAR_TOTAL_LOSSES: f64 = 1_000.0;

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
    pub attacker_total_losses: f64,
    pub defender_total_losses: f64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
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
    let mut views: Vec<_> = wars
        .iter()
        .map(WarView::from)
        .filter(|war| war.total_losses >= MIN_WAR_TOTAL_LOSSES)
        .collect();
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
        let (attackers, defenders) = participant_lists(war);
        let (start_date, end_date) = war_date_range(war);

        Self {
            name: war.name.clone(),
            kind: WarKindView::from(&war.kind),
            attackers,
            defenders,
            battle_count: battles.len(),
            total_losses: war.total_losses(),
            attacker_total_losses: war.attacker_total_losses(),
            defender_total_losses: war.defender_total_losses(),
            start_date,
            end_date,
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

fn participant_lists(war: &WarData) -> (Vec<String>, Vec<String>) {
    derived_participant_lists(war)
}

fn derived_participant_lists(war: &WarData) -> (Vec<String>, Vec<String>) {
    let mut attackers = ParticipantAccumulator::new();
    let mut defenders = ParticipantAccumulator::new();

    attackers.note(&war.original_attacker, None);
    defenders.note(&war.original_defender, None);

    for attacker in &war.attackers {
        attackers.note(attacker, None);
    }

    for defender in &war.defenders {
        defenders.note(defender, None);
    }

    for entry in &war.history.dated_entries {
        collect_history_participants(&entry.value, entry.date, &mut attackers, &mut defenders);
    }

    for battle in &war.history.battles {
        note_battle_participants(battle, &mut attackers, &mut defenders);
    }

    (attackers.into_tags(), defenders.into_tags())
}

fn collect_history_participants(
    value: &Value,
    date: SaveDate,
    attackers: &mut ParticipantAccumulator,
    defenders: &mut ParticipantAccumulator,
) {
    match value {
        Value::Block(Block::Statements(fields)) => {
            for field in fields {
                match field.key.as_str() {
                    "add_attacker" => {
                        if let Some(country) = field.value.as_ref().and_then(value_to_country_tag) {
                            attackers.note(&country, Some(date));
                        }
                    }
                    "add_defender" => {
                        if let Some(country) = field.value.as_ref().and_then(value_to_country_tag) {
                            defenders.note(&country, Some(date));
                        }
                    }
                    "battle" => {
                        if let Some(battle) = field.value.as_ref().and_then(parse_battle_from_value)
                        {
                            note_battle_participants(&battle, attackers, defenders);
                        }
                    }
                    _ => {
                        if let Some(value) = field.value.as_ref() {
                            collect_history_participants(value, date, attackers, defenders);
                        }
                    }
                }
            }
        }
        Value::Block(Block::Values(values)) => {
            for value in values {
                collect_history_participants(value, date, attackers, defenders);
            }
        }
        _ => {}
    }
}

fn note_battle_participants(
    battle: &Battle,
    attackers: &mut ParticipantAccumulator,
    defenders: &mut ParticipantAccumulator,
) {
    if let Some(country) = battle.attacker.country.as_deref() {
        attackers.note(country, None);
    }

    if let Some(country) = battle.defender.country.as_deref() {
        defenders.note(country, None);
    }
}

fn value_to_country_tag(value: &Value) -> Option<String> {
    match value {
        Value::Identifier(value) | Value::String(value) => Some(value.clone()),
        Value::Integer(value) => Some(value.to_string()),
        Value::Decimal(value) => Some(value.to_string()),
        Value::Block(_) => None,
    }
}

fn war_date_range(war: &WarData) -> (Option<String>, Option<String>) {
    let start = war
        .history
        .dated_entries
        .iter()
        .map(|entry| entry.date)
        .min()
        .map(SaveDate::to_iso_string);
    let end = war
        .history
        .dated_entries
        .iter()
        .map(|entry| entry.date)
        .max()
        .map(SaveDate::to_iso_string);

    (start, end)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParticipantAccumulator {
    participants: Vec<ParticipantEntry>,
    next_seen_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParticipantEntry {
    tag: String,
    join_date: Option<SaveDate>,
    first_seen_index: usize,
}

impl ParticipantAccumulator {
    fn new() -> Self {
        Self {
            participants: Vec::new(),
            next_seen_index: 0,
        }
    }

    fn note(&mut self, tag: &str, join_date: Option<SaveDate>) {
        if let Some(participant) = self
            .participants
            .iter_mut()
            .find(|participant| participant.tag == tag)
        {
            match (participant.join_date, join_date) {
                (Some(existing), Some(next)) if next < existing => {
                    participant.join_date = Some(next);
                }
                (None, Some(next)) => {
                    participant.join_date = Some(next);
                }
                _ => {}
            }

            return;
        }

        self.participants.push(ParticipantEntry {
            tag: tag.to_string(),
            join_date,
            first_seen_index: self.next_seen_index,
        });
        self.next_seen_index += 1;
    }

    fn into_tags(mut self) -> Vec<String> {
        self.participants
            .sort_by(|left, right| match (left.join_date, right.join_date) {
                (Some(left_date), Some(right_date)) => left_date
                    .cmp(&right_date)
                    .then_with(|| left.first_seen_index.cmp(&right.first_seen_index)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => left.first_seen_index.cmp(&right.first_seen_index),
            });

        self.participants
            .into_iter()
            .map(|participant| participant.tag)
            .collect()
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
                    losses = 700
                  }
                  defender = {
                    country = FRA
                    infantry = 800
                    losses = 400
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
                    losses = 600
                  }
                  defender = {
                    country = TUR
                    infantry = 100
                    losses = 650
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
        assert_eq!(view.active_wars[0].attacker_total_losses, 3000.0);
        assert_eq!(view.active_wars[0].defender_total_losses, 1800.0);
        assert_eq!(view.active_wars[0].start_date, None);
        assert_eq!(view.active_wars[0].end_date, None);
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
        assert_eq!(view.previous_wars[0].total_losses, 1250.0);
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
        assert_eq!(war.attacker_total_losses, 3000000.0);
        assert_eq!(war.defender_total_losses, 0.0);
        assert_eq!(war.start_date, None);
        assert_eq!(war.end_date, None);
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
                    losses = 700
                  }
                  defender = {
                    country = FRA
                    losses = 400
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

    #[test]
    fn filters_out_wars_below_minimum_total_losses() {
        let document = parse_document(
            r#"
            active_war = {
              name = "Below Threshold"
              history = {
                battle = {
                  name = "Small Clash"
                  location = 1
                  result = yes
                  attacker = {
                    country = ENG
                    losses = 500
                  }
                  defender = {
                    country = FRA
                    losses = 499
                  }
                }
              }
              attacker = ENG
              defender = FRA
              original_attacker = ENG
              original_defender = FRA
            }
            active_war = {
              name = "At Threshold"
              history = {
                battle = {
                  name = "Big Enough"
                  location = 2
                  result = yes
                  attacker = {
                    country = PRU
                    losses = 600
                  }
                  defender = {
                    country = AUS
                    losses = 400
                  }
                }
              }
              attacker = PRU
              defender = AUS
              original_attacker = PRU
              original_defender = AUS
            }
            previous_war = {
              name = "No Casualties"
              history = {
                battle = {
                  name = "Standoff"
                  location = 3
                  result = no
                  attacker = {
                    country = RUS
                    losses = 0
                  }
                  defender = {
                    country = TUR
                    losses = 0
                  }
                }
              }
              original_attacker = RUS
              original_defender = TUR
            }
            "#,
        )
        .unwrap();

        let wars = extract_wars(&document);
        let view = build_parsed_savefile_view("thresholds.v2".to_string(), 3, &wars);

        assert_eq!(view.active_wars.len(), 1);
        assert_eq!(view.active_wars[0].name, "At Threshold");
        assert!(view.previous_wars.is_empty());
    }

    #[test]
    fn reconstructs_previous_war_participants_from_joins_and_battles() {
        let document = parse_document(
            r#"
            previous_war = {
              name = "Join History War"
              history = {
                1836.1.1 = {
                  add_attacker = ENG
                  add_defender = FRA
                }
                1836.1.2 = {
                  add_attacker = USA
                }
                1836.1.3 = {
                  add_defender = PRU
                }
                1836.1.4 = {
                  add_attacker = ITA
                }
                1836.1.5 = {
                  add_defender = NGF
                }
                1836.1.6 = {
                  theater = {
                    battle = {
                      name = "River Fight"
                      location = 90
                      result = yes
                      attacker = {
                        country = BRA
                        losses = 0
                      }
                      defender = {
                        country = ARG
                        losses = 0
                      }
                    }
                  }
                }
                battle = {
                  name = "Field Test"
                  location = 80
                  result = yes
                  attacker = {
                    country = BEL
                    losses = 700
                  }
                  defender = {
                    country = HOL
                    losses = 600
                  }
                }
              }
              attacker = USA
              defender = PRU
              original_attacker = ENG
              original_defender = FRA
            }
            "#,
        )
        .unwrap();

        let wars = extract_wars(&document);
        let view = build_parsed_savefile_view("joins.v2".to_string(), 1, &wars);
        let war = &view.previous_wars[0];

        assert_eq!(
            war.attackers,
            vec![
                "ENG".to_string(),
                "USA".to_string(),
                "ITA".to_string(),
                "BRA".to_string(),
                "BEL".to_string(),
            ]
        );
        assert_eq!(
            war.defenders,
            vec![
                "FRA".to_string(),
                "PRU".to_string(),
                "NGF".to_string(),
                "ARG".to_string(),
                "HOL".to_string(),
            ]
        );
        assert_eq!(war.start_date.as_deref(), Some("1836-01-01"));
        assert_eq!(war.end_date.as_deref(), Some("1836-01-06"));
    }

    #[test]
    fn orders_previous_war_participants_by_dated_join_entries_not_declaration_order() {
        let document = parse_document(
            r#"
            previous_war = {
              name = "Out Of Order"
              history = {
                1836.1.3 = {
                  add_attacker = USA
                }
                1836.1.1 = {
                  add_attacker = ENG
                }
                1836.1.2 = {
                  add_attacker = PRU
                }
                battle = {
                  name = "Enough Losses"
                  location = 8
                  result = yes
                  attacker = {
                    country = ENG
                    losses = 700
                  }
                  defender = {
                    country = FRA
                    losses = 400
                  }
                }
              }
              attacker = USA
              attacker = ENG
              attacker = PRU
              original_attacker = USA
              original_defender = FRA
            }
            "#,
        )
        .unwrap();

        let wars = extract_wars(&document);
        let view = build_parsed_savefile_view("ordered.v2".to_string(), 1, &wars);

        assert_eq!(
            view.previous_wars[0].attackers,
            vec!["ENG".to_string(), "PRU".to_string(), "USA".to_string()]
        );
    }

    #[test]
    fn orders_active_war_participants_by_dated_join_entries_not_declaration_order() {
        let document = parse_document(
            r#"
            active_war = {
              name = "The Great War"
              history = {
                1900.7.13 = {
                  add_attacker = "ITA"
                }
                1900.7.13 = {
                  add_defender = "TUR"
                }
                1900.7.13 = {
                  add_defender = "GER"
                }
                1900.10.26 = {
                  add_attacker = "IBR"
                }
                1901.6.10 = {
                  add_defender = "NET"
                }
                1901.6.12 = {
                  add_attacker = "FRA"
                }
                1901.6.13 = {
                  add_defender = "RUS"
                }
                1901.6.13 = {
                  add_attacker = "JAP"
                }
                1901.6.13 = {
                  add_attacker = "ENG"
                }
                1901.6.13 = {
                  add_attacker = "MEX"
                }
                1901.6.13 = {
                  add_attacker = "COM"
                }
                1901.7.16 = {
                  add_defender = "SCA"
                }
                1902.1.3 = {
                  add_defender = "ARG"
                }
              }
              attacker = "FRA"
              attacker = "ITA"
              attacker = "IBR"
              attacker = "JAP"
              attacker = "ENG"
              attacker = "MEX"
              attacker = "COM"
              defender = "GER"
              defender = "TUR"
              defender = "NET"
              defender = "RUS"
              defender = "SCA"
              defender = "ARG"
              original_attacker = "ITA"
              original_defender = "TUR"
              original_wargoal = {
                state_province_id = 674
                casus_belli = "acquire_state_small"
                actor = "ITA"
                receiver = "TUR"
                score = 0.000
                change = 0.000
                date = "-1.1.1"
                is_fulfilled = no
              }
              action = "1908.6.29"
              great_wars_enabled = yes
              war_goal = {
                state_province_id = 674
                casus_belli = "acquire_state_small"
                actor = "ITA"
                receiver = "TUR"
                score = -100.000
                change = 0.000
                date = "1900.7.13"
                is_fulfilled = no
              }
            }
            "#,
        )
        .unwrap();

        let wars = extract_wars(&document);
        let war = super::WarView::from(&wars.active_wars[0]);

        assert_eq!(
            war.attackers,
            vec![
                "ITA".to_string(),
                "IBR".to_string(),
                "FRA".to_string(),
                "JAP".to_string(),
                "ENG".to_string(),
                "MEX".to_string(),
                "COM".to_string(),
            ]
        );
        assert_eq!(
            war.defenders,
            vec![
                "TUR".to_string(),
                "GER".to_string(),
                "NET".to_string(),
                "RUS".to_string(),
                "SCA".to_string(),
                "ARG".to_string(),
            ]
        );
        assert_eq!(war.start_date.as_deref(), Some("1900-07-13"));
        assert_eq!(war.end_date.as_deref(), Some("1902-01-03"));
    }

    #[test]
    fn derives_war_date_ranges_from_history_entries() {
        let document = parse_document(
            r#"
            active_war = {
              name = "No Dates"
              history = {
                battle = {
                  name = "Known Fight"
                  location = 1
                  result = yes
                  attacker = {
                    country = ENG
                    losses = 700
                  }
                  defender = {
                    country = FRA
                    losses = 500
                  }
                }
              }
              attacker = ENG
              defender = FRA
              original_attacker = ENG
              original_defender = FRA
            }
            active_war = {
              name = "Single Date"
              history = {
                1836.2.3 = {
                  mobilized = yes
                }
                battle = {
                  name = "Known Fight"
                  location = 2
                  result = yes
                  attacker = {
                    country = PRU
                    losses = 800
                  }
                  defender = {
                    country = AUS
                    losses = 400
                  }
                }
              }
              attacker = PRU
              defender = AUS
              original_attacker = PRU
              original_defender = AUS
            }
            active_war = {
              name = "Many Dates"
              history = {
                1836.5.7 = {
                  mobilized = yes
                }
                1836.1.4 = {
                  add_attacker = ITA
                }
                1836.3.2 = {
                  add_defender = NGF
                }
                battle = {
                  name = "Known Fight"
                  location = 3
                  result = yes
                  attacker = {
                    country = RUS
                    losses = 900
                  }
                  defender = {
                    country = TUR
                    losses = 500
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
        let view = build_parsed_savefile_view("dates.v2".to_string(), 3, &wars);
        let no_dates = view
            .active_wars
            .iter()
            .find(|war| war.name == "No Dates")
            .expect("expected no-dates war");
        let single_date = view
            .active_wars
            .iter()
            .find(|war| war.name == "Single Date")
            .expect("expected single-date war");
        let many_dates = view
            .active_wars
            .iter()
            .find(|war| war.name == "Many Dates")
            .expect("expected many-dates war");

        assert_eq!(no_dates.start_date, None);
        assert_eq!(no_dates.end_date, None);
        assert_eq!(single_date.start_date.as_deref(), Some("1836-02-03"));
        assert_eq!(single_date.end_date.as_deref(), Some("1836-02-03"));
        assert_eq!(many_dates.start_date.as_deref(), Some("1836-01-04"));
        assert_eq!(many_dates.end_date.as_deref(), Some("1836-05-07"));
    }
}
