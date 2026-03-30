#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(dead_code)]

mod parser;
mod view;
mod war;

use std::fs;

#[tauri::command]
fn parse_savefile(path: String) -> Result<view::ParsedSavefileView, String> {
    let bytes =
        fs::read(&path).map_err(|error| format!("Failed to read savefile '{path}': {error}"))?;
    let contents = decode_windows_1252(&bytes);
    let document = parser::parse_document(&contents)
        .map_err(|error| format!("Failed to parse savefile '{path}': {error}"))?;
    let wars = war::extract_wars(&document);

    Ok(view::build_parsed_savefile_view(
        path,
        document.statements.len(),
        &wars,
    ))
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
    use super::decode_windows_1252;

    #[test]
    fn decodes_windows_1252_bytes() {
        let decoded = decode_windows_1252(&[0x43, 0x61, 0x66, 0xE9, 0x20, 0x97, 0x20, 0x80]);

        assert_eq!(decoded, "Caf\u{00E9} \u{2014} \u{20AC}");
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![parse_savefile])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
