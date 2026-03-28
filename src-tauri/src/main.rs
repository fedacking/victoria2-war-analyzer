#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod parser;

use std::fs;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParseSummary {
    path: String,
    top_level_statement_count: usize,
}

#[tauri::command]
fn parse_savefile(path: String) -> Result<ParseSummary, String> {
    let bytes =
        fs::read(&path).map_err(|error| format!("Failed to read savefile '{path}': {error}"))?;
    let contents = decode_windows_1252(&bytes);
    let document = parser::parse_document(&contents)
        .map_err(|error| format!("Failed to parse savefile '{path}': {error}"))?;

    print_war_keys(&document, "previous_war");
    print_war_keys(&document, "active_war");

    Ok(ParseSummary {
        path,
        top_level_statement_count: document.statements.len(),
    })
}

fn print_war_keys(document: &parser::Document, war_key: &str) {
    for (index, keys) in collect_raw_war_keys(document, war_key).iter().enumerate() {
        println!("{war_key}[{}]", index);

        for key in keys {
            println!("{key}");
        }
    }
}

fn collect_raw_war_keys(document: &parser::Document, war_key: &str) -> Vec<Vec<String>> {
    document
        .statements
        .iter()
        .filter(|statement| statement.key == war_key)
        .map(|statement| match &statement.value {
            Some(parser::Value::Block(parser::Block::Statements(war_statements))) => war_statements
                .iter()
                .map(|war_statement| war_statement.key.clone())
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match byte {
            0x80 => '\u{20AC}',
            0x82 => '\u{201A}',
            0x83 => '\u{0192}',
            0x84 => '\u{201E}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02C6}',
            0x89 => '\u{2030}',
            0x8A => '\u{0160}',
            0x8B => '\u{2039}',
            0x8C => '\u{0152}',
            0x8E => '\u{017D}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201C}',
            0x94 => '\u{201D}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02DC}',
            0x99 => '\u{2122}',
            0x9A => '\u{0161}',
            0x9B => '\u{203A}',
            0x9C => '\u{0153}',
            0x9E => '\u{017E}',
            0x9F => '\u{0178}',
            _ => char::from(*byte),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{collect_raw_war_keys, decode_windows_1252};
    use crate::parser::parse_document;

    #[test]
    fn decodes_windows_1252_bytes() {
        let decoded = decode_windows_1252(&[0x43, 0x61, 0x66, 0xE9, 0x20, 0x97, 0x20, 0x80]);

        assert_eq!(decoded, "Caf\u{00E9} \u{2014} \u{20AC}");
    }

    #[test]
    fn collects_raw_war_keys_in_order() {
        let document = parse_document(
            r#"
            previous_war = {
              name = "Brothers War"
              attackers = { ENG }
              defenders = { FRA }
            }
            previous_war = {
              name = "Second War"
              name = "Still Second War"
              war_goal = acquire_state
              defenders = { USA }
            }
            active_war = {
              name = "Current War"
              war_score = 12
            }
            "#,
        )
        .unwrap();

        let previous_wars = collect_raw_war_keys(&document, "previous_war");
        let active_wars = collect_raw_war_keys(&document, "active_war");

        assert_eq!(
            previous_wars,
            vec![
                vec![
                    "name".to_string(),
                    "attackers".to_string(),
                    "defenders".to_string(),
                ],
                vec![
                    "name".to_string(),
                    "name".to_string(),
                    "war_goal".to_string(),
                    "defenders".to_string(),
                ],
            ]
        );

        assert_eq!(
            active_wars,
            vec![vec!["name".to_string(), "war_score".to_string()]]
        );
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![parse_savefile])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
