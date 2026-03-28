# Victoria 2 War Analyzer Roadmap

## Goal

Build a Rust GUI application that can read a Victoria 2 savefile and show useful information about countries, wars, and military strength.

## Initial Milestones

### 1. Savefile support

- Open a Victoria 2 savefile
- Support the save formats we care about
- Read basic metadata such as date, player country, and version

### 2. Country stats

- List countries found in the save
- Show important country stats
- Allow sorting and filtering

### 3. Military stats

- Show army size and unit breakdown
- Show navy size and ship breakdown
- Show mobilization and other military-related values

### 4. War overview

- List active wars
- Show attackers, defenders, war leader, and war goals
- Show current war score and other relevant war data

### 5. War analysis

- Compare the strength of both sides in a war
- Highlight the main reasons one side is ahead
- Surface useful derived metrics instead of only raw save data

### 6. GUI and usability

- Build the project as a Tauri + React GUI app
- Export useful data to JSON or CSV
- Keep performance good on large savefiles
