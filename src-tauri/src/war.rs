use std::collections::BTreeMap;

use crate::parser::{Block, Document, Statement, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarKind {
    Previous,
    Active,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WarCollection {
    pub previous_wars: Vec<WarData>,
    pub active_wars: Vec<WarData>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WarData {
    pub kind: WarKind,
    pub name: String,
    pub history: WarHistory,
    pub attackers: Vec<String>,
    pub defenders: Vec<String>,
    pub original_attacker: String,
    pub original_defender: String,
    pub original_wargoals: Vec<Value>,
    pub actions: Vec<Value>,
    pub war_goals: Vec<Value>,
    pub great_wars_enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WarHistory {
    pub battles: Vec<Battle>,
    pub dated_entries: Vec<DatedWarHistoryEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Battle {
    pub name: String,
    pub location: i64,
    pub attacker_won: Option<bool>,
    pub attacker: BattleSide,
    pub defender: BattleSide,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BattleSide {
    pub country: Option<String>,
    pub leader: Option<String>,
    pub losses: Option<FixedPoint32>,
    pub unit_counts: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatedWarHistoryEntry {
    pub date: SaveDate,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SaveDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedPoint32 {
    raw: i32,
}

impl WarData {
    pub fn attacker_total_losses(&self) -> f64 {
        self.history
            .battles
            .iter()
            .map(Battle::attacker_losses)
            .sum()
    }

    pub fn defender_total_losses(&self) -> f64 {
        self.history
            .battles
            .iter()
            .map(Battle::defender_losses)
            .sum()
    }

    pub fn total_losses(&self) -> f64 {
        self.attacker_total_losses() + self.defender_total_losses()
    }
}

impl Battle {
    pub fn attacker_losses(&self) -> f64 {
        self.attacker.losses_amount().unwrap_or(0.0)
    }

    pub fn defender_losses(&self) -> f64 {
        self.defender.losses_amount().unwrap_or(0.0)
    }

    pub fn total_losses(&self) -> f64 {
        self.attacker_losses() + self.defender_losses()
    }
}

impl BattleSide {
    pub fn losses_amount(&self) -> Option<f64> {
        self.losses.map(FixedPoint32::to_f64)
    }
}

impl FixedPoint32 {
    const SCALE: i64 = 1_000;
    const OVERFLOW_WRAP: i64 = 1_i64 << 32;

    fn from_value(value: &Value) -> Option<Self> {
        let scaled = match value {
            Value::Integer(value) => value.checked_mul(Self::SCALE)?,
            Value::Decimal(value) => (value * Self::SCALE as f64).round() as i64,
            _ => return None,
        };

        Some(Self {
            raw: i32::try_from(scaled).ok()?,
        })
    }

    fn corrected_raw(self) -> u64 {
        let raw = i64::from(self.raw);
        let corrected = if raw < 0 {
            raw + Self::OVERFLOW_WRAP
        } else {
            raw
        };

        corrected as u64
    }

    fn to_f64(self) -> f64 {
        Self::raw_to_f64(self.corrected_raw())
    }

    fn raw_to_f64(raw: u64) -> f64 {
        raw as f64 / Self::SCALE as f64
    }
}

impl SaveDate {
    pub fn to_iso_string(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

pub fn extract_wars(document: &Document) -> WarCollection {
    WarCollection {
        previous_wars: extract_wars_for_key(document, "previous_war", WarKind::Previous),
        active_wars: extract_wars_for_key(document, "active_war", WarKind::Active),
    }
}

fn extract_wars_for_key(
    document: &Document,
    top_level_key: &str,
    war_kind: WarKind,
) -> Vec<WarData> {
    document
        .statements
        .iter()
        .filter(|statement| statement.key == top_level_key)
        .filter_map(|statement| parse_war(statement, war_kind.clone()))
        .collect()
}

fn parse_war(statement: &Statement, kind: WarKind) -> Option<WarData> {
    let Some(Value::Block(crate::parser::Block::Statements(fields))) = &statement.value else {
        return None;
    };

    let mut name = None;
    let mut history = None;
    let mut attackers = Vec::new();
    let mut defenders = Vec::new();
    let mut original_attacker = None;
    let mut original_defender = None;
    let mut original_wargoals = Vec::new();
    let mut actions = Vec::new();
    let mut war_goals = Vec::new();
    let mut great_wars_enabled = None;

    for field in fields {
        match field.key.as_str() {
            "name" => {
                name = field.value.as_ref().and_then(value_to_string);
            }
            "history" => {
                history = field.value.as_ref().and_then(parse_war_history);
            }
            "attacker" => {
                if let Some(value) = field.value.as_ref().and_then(value_to_string) {
                    attackers.push(value);
                }
            }
            "defender" => {
                if let Some(value) = field.value.as_ref().and_then(value_to_string) {
                    defenders.push(value);
                }
            }
            "original_attacker" => {
                if let Some(value) = field.value.as_ref().and_then(value_to_string) {
                    assert!(
                        original_attacker.is_none(),
                        "duplicate original_attacker in war '{}'",
                        name.as_deref().unwrap_or("<unknown>")
                    );
                    original_attacker = Some(value);
                }
            }
            "original_defender" => {
                if let Some(value) = field.value.as_ref().and_then(value_to_string) {
                    assert!(
                        original_defender.is_none(),
                        "duplicate original_defender in war '{}'",
                        name.as_deref().unwrap_or("<unknown>")
                    );
                    original_defender = Some(value);
                }
            }
            "original_wargoal" => {
                if let Some(value) = field.value.clone() {
                    original_wargoals.push(value);
                }
            }
            "action" => {
                if let Some(value) = field.value.clone() {
                    actions.push(value);
                }
            }
            "war_goal" => {
                if let Some(value) = field.value.clone() {
                    war_goals.push(value);
                }
            }
            "great_wars_enabled" => {
                great_wars_enabled = field.value.as_ref().and_then(value_to_bool);
            }
            _ => {}
        }
    }

    Some(WarData {
        kind,
        name: name?,
        history: history?,
        attackers,
        defenders,
        original_attacker: original_attacker?,
        original_defender: original_defender?,
        original_wargoals,
        actions,
        war_goals,
        great_wars_enabled,
    })
}

fn parse_war_history(value: &Value) -> Option<WarHistory> {
    let Value::Block(Block::Statements(history_fields)) = value else {
        return None;
    };

    let mut history = WarHistory {
        battles: Vec::new(),
        dated_entries: Vec::new(),
    };

    for field in history_fields {
        if field.key == "battle" {
            if let Some(value) = field.value.as_ref().and_then(parse_battle_from_value) {
                history.battles.push(value);
            }

            continue;
        }

        let Some(date) = parse_save_date(&field.key) else {
            continue;
        };

        if let Some(value) = field.value.clone() {
            collect_battles_from_history_value(&value, &mut history.battles);
            history
                .dated_entries
                .push(DatedWarHistoryEntry { date, value });
        }
    }

    Some(history)
}

fn collect_battles_from_history_value(value: &Value, battles: &mut Vec<Battle>) {
    match value {
        Value::Block(Block::Statements(fields)) => {
            for field in fields {
                if field.key == "battle" {
                    if let Some(battle) = field.value.as_ref().and_then(parse_battle_from_value) {
                        battles.push(battle);
                    }

                    continue;
                }

                if let Some(value) = field.value.as_ref() {
                    collect_battles_from_history_value(value, battles);
                }
            }
        }
        Value::Block(Block::Values(values)) => {
            for value in values {
                collect_battles_from_history_value(value, battles);
            }
        }
        _ => {}
    }
}

pub(crate) fn parse_battle_from_value(value: &Value) -> Option<Battle> {
    let Value::Block(Block::Statements(fields)) = value else {
        return None;
    };

    let mut name = None;
    let mut location = None;
    let mut attacker_won = None;
    let mut attacker = None;
    let mut defender = None;

    for field in fields {
        match field.key.as_str() {
            "name" => {
                name = field.value.as_ref().and_then(value_to_string);
            }
            "location" => {
                location = field.value.as_ref().and_then(value_to_i64);
            }
            "result" => {
                attacker_won = field.value.as_ref().and_then(value_to_bool);
            }
            "attacker" => {
                attacker = field.value.as_ref().and_then(parse_battle_side);
            }
            "defender" => {
                defender = field.value.as_ref().and_then(parse_battle_side);
            }
            _ => {}
        }
    }

    Some(Battle {
        name: name?,
        location: location?,
        attacker_won,
        attacker: attacker?,
        defender: defender?,
    })
}

fn parse_battle_side(value: &Value) -> Option<BattleSide> {
    let Value::Block(Block::Statements(fields)) = value else {
        return None;
    };

    let mut side = BattleSide {
        country: None,
        leader: None,
        losses: None,
        unit_counts: BTreeMap::new(),
    };

    for field in fields {
        match field.key.as_str() {
            "country" => {
                side.country = field.value.as_ref().and_then(value_to_string);
            }
            "leader" => {
                side.leader = field.value.as_ref().and_then(value_to_string);
            }
            "losses" => {
                side.losses = field.value.as_ref().and_then(FixedPoint32::from_value);
            }
            unit_key => {
                if let Some(count) = field.value.as_ref().and_then(value_to_i64) {
                    side.unit_counts.insert(unit_key.to_string(), count);
                }
            }
        }
    }

    Some(side)
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Identifier(value) | Value::String(value) => Some(value.clone()),
        Value::Integer(value) => Some(value.to_string()),
        Value::Decimal(value) => Some(value.to_string()),
        Value::Block(_) => None,
    }
}

fn value_to_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Identifier(value) if value == "yes" => Some(true),
        Value::Identifier(value) if value == "no" => Some(false),
        Value::Integer(1) => Some(true),
        Value::Integer(0) => Some(false),
        _ => None,
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

fn parse_save_date(value: &str) -> Option<SaveDate> {
    let mut parts = value.split('.');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Some(SaveDate { year, month, day })
}

#[cfg(test)]
mod tests {
    use super::{WarKind, extract_wars};
    use crate::parser::{Statement, Value, parse_document};

    #[test]
    fn extracts_previous_and_active_wars_into_war_data() {
        let document = parse_document(
            r#"
            previous_war = {
              name = "The Brothers War"
              history = {
                battle = {
                  name = "Kavala"
                  location = 823
                  result = no
                  attacker = {
                    country = "ITA"
                    leader = "Pirzio Colonna"
                    artillery = 63000
                    guard = 24000
                    hussar = 12000
                    infantry = 24000
                    losses = 83643
                  }
                  defender = {
                    country = "TUR"
                    leader = "Lutfi Bey"
                    artillery = 90000
                    losses = 19697
                  }
                }
                1836.12.9 = {
                  add_attacker = USA
                }
              }
              original_attacker = ENG
              original_defender = FRA
              original_wargoal = {
                casus_belli = cut_down_to_size
              }
              action = {
                month = 4
              }
            }
            active_war = {
              name = "Great War"
              history = {
                battle = {
                  name = "Edirne"
                  location = 456
                  result = yes
                  attacker = {
                    country = ENG
                    artillery = 4000
                    infantry = 8000
                    losses = 1200
                  }
                  defender = {
                    country = FRA
                    artillery = 3000
                    losses = 900
                  }
                }
                1914.8.4 = {
                  mobilized = yes
                }
              }
              attacker = ENG
              attacker = USA
              defender = FRA
              defender = NGF
              original_attacker = ENG
              original_defender = FRA
              original_wargoal = {
                casus_belli = take_from_sphere
              }
              action = {
                month = 7
              }
              great_wars_enabled = yes
              war_goal = {
                actor = ENG
              }
            }
            "#,
        )
        .unwrap();

        let wars = extract_wars(&document);

        assert_eq!(wars.previous_wars.len(), 1);
        assert_eq!(wars.active_wars.len(), 1);

        let previous_war = &wars.previous_wars[0];
        assert_eq!(previous_war.kind, WarKind::Previous);
        assert_eq!(previous_war.name, "The Brothers War");
        assert_eq!(previous_war.original_attacker, "ENG");
        assert_eq!(previous_war.original_defender, "FRA");
        assert_eq!(previous_war.actions.len(), 1);
        assert_eq!(previous_war.original_wargoals.len(), 1);
        assert!(previous_war.war_goals.is_empty());
        let history = &previous_war.history;
        assert_eq!(history.battles.len(), 1);
        assert_eq!(history.battles[0].name, "Kavala");
        assert_eq!(history.battles[0].location, 823);
        assert_eq!(history.battles[0].attacker_won, Some(false));
        assert_eq!(history.battles[0].attacker.country.as_deref(), Some("ITA"));
        assert_eq!(
            history.battles[0].attacker.leader.as_deref(),
            Some("Pirzio Colonna")
        );
        assert_eq!(history.battles[0].attacker.losses_amount(), Some(83643.0));
        assert_eq!(
            history.battles[0].attacker.unit_counts.get("artillery"),
            Some(&63000)
        );
        assert_eq!(history.battles[0].defender.country.as_deref(), Some("TUR"));
        assert_eq!(history.battles[0].defender.losses_amount(), Some(19697.0));
        assert_eq!(history.battles[0].attacker_losses(), 83643.0);
        assert_eq!(history.battles[0].defender_losses(), 19697.0);
        assert_eq!(history.battles[0].total_losses(), 103340.0);
        assert_eq!(previous_war.attacker_total_losses(), 83643.0);
        assert_eq!(previous_war.defender_total_losses(), 19697.0);
        assert_eq!(previous_war.total_losses(), 103340.0);
        assert_eq!(history.dated_entries.len(), 1);
        assert_eq!(history.dated_entries[0].date.year, 1836);
        assert_eq!(history.dated_entries[0].date.month, 12);
        assert_eq!(history.dated_entries[0].date.day, 9);
        assert_eq!(
            history.dated_entries[0].value,
            Value::Block(crate::parser::Block::Statements(vec![Statement {
                key: "add_attacker".to_string(),
                value: Some(Value::Identifier("USA".to_string())),
            }]))
        );

        let active_war = &wars.active_wars[0];
        assert_eq!(active_war.kind, WarKind::Active);
        assert_eq!(active_war.name, "Great War");
        assert_eq!(
            active_war.attackers,
            vec!["ENG".to_string(), "USA".to_string()]
        );
        assert_eq!(
            active_war.defenders,
            vec!["FRA".to_string(), "NGF".to_string()]
        );
        assert_eq!(active_war.original_attacker, "ENG");
        assert_eq!(active_war.original_defender, "FRA");
        assert_eq!(active_war.great_wars_enabled, Some(true));
        assert_eq!(active_war.actions.len(), 1);
        assert_eq!(active_war.original_wargoals.len(), 1);
        assert_eq!(active_war.war_goals.len(), 1);
        assert_eq!(
            active_war.history.battles[0]
                .attacker
                .unit_counts
                .get("infantry"),
            Some(&8000)
        );
        assert_eq!(active_war.history.battles[0].attacker_won, Some(true));
        assert_eq!(active_war.history.dated_entries[0].date.year, 1914);
    }

    #[test]
    fn extracts_active_war_battles_nested_under_dated_history_entries() {
        let document = parse_document(
            r#"
            active_war = {
              name = "Dated Battle War"
              history = {
                1901.2.3 = {
                  theater = {
                    battle = {
                      name = "Kavala"
                      location = 823
                      result = 1
                      attacker = {
                        country = ENG
                        losses = 600
                      }
                      defender = {
                        country = FRA
                        losses = 400
                      }
                    }
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
        let war = &wars.active_wars[0];

        assert_eq!(war.history.dated_entries.len(), 1);
        assert_eq!(war.history.battles.len(), 1);
        assert_eq!(war.history.battles[0].name, "Kavala");
        assert_eq!(war.history.battles[0].location, 823);
        assert_eq!(war.history.battles[0].attacker_won, Some(true));
        assert_eq!(war.history.battles[0].total_losses(), 1000.0);
        assert_eq!(war.attacker_total_losses(), 600.0);
        assert_eq!(war.defender_total_losses(), 400.0);
        assert_eq!(war.total_losses(), 1000.0);
    }

    #[test]
    #[should_panic(expected = "duplicate original_attacker")]
    fn panics_when_original_attacker_repeats() {
        let document = parse_document(
            r#"
            previous_war = {
              name = "Repeated Attacker"
              history = {
                battle = {
                  name = "Kavala"
                  location = 823
                  result = no
                  attacker = {
                    country = "ITA"
                    losses = 1
                  }
                  defender = {
                    country = "TUR"
                    losses = 2
                  }
                }
              }
              original_attacker = ENG
              original_attacker = USA
              original_defender = FRA
            }
            "#,
        )
        .unwrap();

        let _ = extract_wars(&document);
    }

    #[test]
    fn corrects_overflowed_battle_losses() {
        let document = parse_document(
            r#"
            active_war = {
              name = "Overflow War"
              history = {
                battle = {
                  name = "Big Battle"
                  location = 1
                  result = yes
                  attacker = {
                    country = ENG
                    losses = -1294967.296
                  }
                  defender = {
                    country = FRA
                    losses = 0
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
        let battle = &wars.active_wars[0].history.battles[0];

        assert_eq!(battle.attacker.losses_amount(), Some(3000000.0));
        assert_eq!(battle.attacker_losses(), 3000000.0);
        assert_eq!(battle.defender_losses(), 0.0);
        assert_eq!(battle.total_losses(), 3000000.0);
        assert_eq!(wars.active_wars[0].attacker_total_losses(), 3000000.0);
        assert_eq!(wars.active_wars[0].defender_total_losses(), 0.0);
        assert_eq!(wars.active_wars[0].total_losses(), 3000000.0);
    }
}
