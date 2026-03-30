# Victoria 2 War Analyzer Roadmap

## Current Status

- The project already has the barebones application structure in place.
- We can already read a Victoria 2 savefile.
- The next roadmap pass should focus on turning parsed data into useful war history views and addressing the latest user-requested features.

## Goal

Build a Rust-first application that lets players inspect current and previous wars from a Victoria 2 savefile, understand their casualties clearly, and keep battle context visible while navigating the UI.

## Next Milestones

### 1. War list quality and metadata

- Parse and expose previous wars from the savefile data
- Show a list of historical wars in the UI
- Remove transfer wars from the main results when their total losses are zero or lower
- Include key identifying information for each war, such as participants, join order, and dates when available
- Show every attacker and defender in previous wars ordered by the date they joined
- Show war dates when they can be derived from the save history
- Show total losses per side, not only a single total for the whole war

### 2. Battle parsing and naming

- Let the user select a previous war and inspect its details
- Handle battles recorded directly under the `battle` key
- Handle current battles that are stored under dated history entries instead of the `battle` key
- Rename repeated battles within the same war so they are distinguishable in the UI, such as first and second battles of the same location
- Show the battle winner by decoding the parsed result flag into the winning side
- Surface attackers, defenders, leaders, winners, and other relevant metadata in a way that is easy to scan and compare

### 3. Battle browsing and layout

- Keep the war list scroll and battle list scroll separate from the main window so battle details stay visible
- Keep the selected battle breakdown pinned and readable while browsing wars and battles
- Show casualties for the selected war
- Break casualties down by side and by country when the save data allows it
- Highlight totals and the biggest losses so the results are immediately useful

### 4. Country and geography data

- Resolve country names from mod files first and fall back to base game data using the three-letter country tag
- Resolve country flags from mod files first and fall back to base game flag assets for the same tag
- Group battles by continent and state by resolving province ownership and geography from game files
- Replace raw province-only battle labels with richer geographic labels when source data is available

## Known Issues and Risks

- Some values may overflow while being processed
- If we encounter negative numbers, we should apply a best-effort correction instead of displaying obviously invalid data
- Battle and timeline data are not encoded the same way for all wars, so both direct battle entries and dated history entries need test coverage
- Mod and base game lookups need a clear precedence order so country names and flags match the loaded save context
