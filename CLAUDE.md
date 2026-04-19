# Claude Consume and Peak Monitor

Cross-platform system tray app (Windows + macOS) that tracks Claude AI consumption (per project, session, mode, and subscription-plan limits) and notifies about peak usage hours with color-coded indicators.

## Tech Stack
- **Framework**: Tauri 2 (Rust backend + webview frontend)
- **Frontend**: React 19 + TypeScript 5.8 + Tailwind CSS 4.2 + Vite 7
- **Package manager**: pnpm

## Architecture
- `src/` - React frontend (popup + dashboard windows)
- `src-tauri/src/` - Rust backend (tray icon, peak detection, polling)
- Three-signal algorithm: time patterns (40%) + Anthropic status (35%) + local stats (25%)
- Data source: `~/.claude/projects/**/*.jsonl` (parsed by `stats_reader.rs`,
  symlink-guarded, 30-day mtime window, 64 MiB per-file cap)

## Commands
- `pnpm install` - Install dependencies
- `pnpm tauri dev` - Development mode
- `pnpm tauri build` - Production build (NSIS .exe + MSI installers, see README)
- `python scripts/generate_icons.py` - Regenerate icon set

## Versioning (IMPORTANT — bump before every release build)

Installer filenames embed the version (e.g. `..._0.2.0_x64_en-US.msi`), so
leaving the version static makes every rebuild indistinguishable from the
prior one. **Before running `pnpm tauri build`, bump the version in all
four places below**, matching semver:
- patch (`0.2.0` → `0.2.1`) for bug fixes
- minor (`0.2.1` → `0.3.0`) for new features
- major (`0.9.x` → `1.0.0`) only once we're out of the 0.x prototype range

Files to update (keep all four in lockstep):
1. `src-tauri/tauri.conf.json` — `"version"` (drives installer filename)
2. `src-tauri/Cargo.toml` — `version = "x.y.z"` (Cargo.lock auto-updates)
3. `package.json` — `"version"` (cosmetic, shown in pnpm output)
4. `src/components/settings/SettingsPanel.tsx` — the `v0.x.y` shown in
   the About section of the dashboard Settings tab

Commit the bump alongside the feature/fix, not as a separate commit, so
`git log` shows exactly which version contains which change.

## Key Files
- `src-tauri/src/peak_engine.rs` - Peak scoring algorithm
- `src-tauri/src/tray.rs` - System tray management
- `src-tauri/src/scheduler.rs` - Background polling tasks
- `src/components/popup/` - Popup window components
- `src/components/dashboard/` - Dashboard window components
