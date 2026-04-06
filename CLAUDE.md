# Claude Peak Monitor

Cross-platform system tray app (Windows + macOS) that notifies about Claude AI peak usage hours with color-coded indicators.

## Tech Stack
- **Framework**: Tauri 2 (Rust backend + webview frontend)
- **Frontend**: React 19 + TypeScript 5.8 + Tailwind CSS 4.2 + Vite 7
- **Package manager**: pnpm

## Architecture
- `src/` - React frontend (popup + dashboard windows)
- `src-tauri/src/` - Rust backend (tray icon, peak detection, polling)
- Three-signal algorithm: time patterns (40%) + Anthropic status (35%) + local stats (25%)
- Data source: `~/.claude/stats-cache.json`

## Commands
- `pnpm install` - Install dependencies
- `pnpm tauri dev` - Development mode
- `pnpm tauri build` - Production build

## Key Files
- `src-tauri/src/peak_engine.rs` - Peak scoring algorithm
- `src-tauri/src/tray.rs` - System tray management
- `src-tauri/src/scheduler.rs` - Background polling tasks
- `src/components/popup/` - Popup window components
- `src/components/dashboard/` - Dashboard window components
