# Victoria 2 War Analyzer Roadmap

## Current Status

- The project already has the barebones application structure in place.
- We can already read a Victoria 2 savefile.
- The roadmap should now focus on turning parsed data into useful war history views.

## Goal

Build a Rust-first application that lets players inspect previous wars from a Victoria 2 savefile and understand their casualties clearly.

## Next Milestones

### 1. Previous wars list

- Parse and expose previous wars from the savefile data
- Show a list of historical wars in the UI
- Include the key identifying information for each war, such as participants and dates when available

### 2. War details view

- Let the user select a previous war and inspect its details
- Show attackers, defenders, leaders, and other relevant metadata
- Surface the information in a way that is easy to scan and compare

### 3. Casualties view

- Show casualties for the selected war
- Break casualties down by side and by country when the save data allows it
- Highlight totals and the biggest losses so the results are immediately useful

### 4. Configurable flags

- Allow the user to choose the flag set or custom flag assets used by the app, including modded games

## Known Issue

- Some values may overflow while being processed
- If we encounter negative numbers, we should apply a best-effort correction instead of displaying obviously invalid data
