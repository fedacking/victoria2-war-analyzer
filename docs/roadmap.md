# Victoria 2 War Analyzer Roadmap

## Current Status

- The project already has the barebones application structure in place.
- We can already read a Victoria 2 savefile.
- We can now resolve country names and default flags from a selected mod folder first, then fall back to the base game folder.
- The next roadmap pass should focus on turning parsed data into richer geographic views and addressing the latest user-requested features.

## Goal

Build a Rust-first application that lets players inspect current and previous wars from a Victoria 2 savefile, understand their casualties clearly, and keep battle context visible while navigating the UI.

## Completed Milestones

### 1. Country data

- Resolve country names from mod files first and fall back to base game data using the three-letter country tag
- Resolve country flags from mod files first and fall back to base game flag assets for the same tag
- Let the user choose the base game folder and an optional mod folder directly from the app

## Next Milestones

### 2. Geography data

- Group battles by continent and state by resolving province ownership and geography from game files
- Replace raw province-only battle labels with richer geographic labels when source data is available

## Known Issues and Risks

- Geography data will need the same mod-first, base-second precedence so province and state names match the loaded save context
