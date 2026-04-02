#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(dead_code)]

mod country;
mod encoding;
mod parser;
mod view;
mod war;

use std::fs;

#[tauri::command]
fn parse_savefile(path: String) -> Result<view::ParsedSavefileView, String> {
    let bytes =
        fs::read(&path).map_err(|error| format!("Failed to read savefile '{path}': {error}"))?;
    let contents = encoding::decode_windows_1252(&bytes);
    let document = parser::parse_document(&contents)
        .map_err(|error| format!("Failed to parse savefile '{path}': {error}"))?;
    let wars = war::extract_wars(&document);

    Ok(view::build_parsed_savefile_view(
        path,
        document.statements.len(),
        &wars,
    ))
}

#[tauri::command]
fn resolve_country_catalog(
    game_path: String,
    mod_path: Option<String>,
    country_tags: Vec<String>,
) -> Result<country::CountryCatalogView, String> {
    country::resolve_country_catalog(game_path, mod_path, country_tags)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            parse_savefile,
            resolve_country_catalog
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
