# Victoria 2 War Analyzer Roadmap

## Current Status

- The project already has the barebones application structure in place.
- We can already read a Victoria 2 savefile.
- The next roadmap pass should focus on turning parsed data into useful war history views and addressing the latest user-requested features.

## Goal

Build a Rust-first application that lets players inspect current and previous wars from a Victoria 2 savefile, understand their casualties clearly, and keep battle context visible while navigating the UI.

## Next Milestones

### 1. Battle browsing and layout

- Give the war list and battle list their own scroll containers so selecting a battle does not push the battle breakdown out of view
- Keep the selected battle breakdown pinned and readable while browsing wars and battles


### 2. Country and geography data

- Resolve country names from mod files first and fall back to base game data using the three-letter country tag
- Resolve country flags from mod files first and fall back to base game flag assets for the same tag
- Group battles by continent and state by resolving province ownership and geography from game files
- Replace raw province-only battle labels with richer geographic labels when source data is available

## Known Issues and Risks

- Some values may overflow while being processed
- Battle and timeline data are not encoded the same way for all wars, so both direct battle entries and dated history entries need test coverage
- Mod and base game lookups need a clear precedence order so country names and flags match the loaded save context
