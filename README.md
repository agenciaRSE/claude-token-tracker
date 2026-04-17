# Claude Consume and Peak Monitor

System tray app that watches Claude AI peak usage hours and your local
Claude Code consumption, then surfaces a color-coded indicator (green →
yellow → orange → red) so you know when it's a good time to push heavy
prompts and when to back off.

- **Tray icon** that changes color with the current peak score.
- **Popup** with the current recommendation, today's tokens, sessions
  and estimated cost.
- **Dashboard** with overview, history (7-day token trend, hourly
  distribution, model breakdown) and settings.
- **Notifications** when the peak color changes or when a daily token
  threshold is hit.

The peak score is computed from three signals:

| Signal | Weight | Source |
|---|---|---|
| Time-of-day patterns | 40% | Built-in heuristics |
| Anthropic service status | 35% | `https://status.anthropic.com/api/v2/components.json` |
| Local Claude Code usage | 25% | `~/.claude/projects/**/*.jsonl` |

No data ever leaves your machine except the unauthenticated GET to the
public Anthropic status page.

---

## Install

Two installers are produced for Windows x64. Both install the **same**
application — pick the one that matches how you deploy software.

### Option 1 — NSIS `.exe` (recommended for most users)

**File:** `Claude Consume and Peak Monitor_0.1.0_x64-setup.exe` (~3.1 MB)

- Per-user install — no admin / UAC prompt required.
- Installs to `%LOCALAPPDATA%\Programs\Claude Consume and Peak Monitor\`.
- Adds a Start Menu shortcut and an entry under
  *Settings → Apps → Installed apps* for clean uninstall.
- Works on Windows 10 1809+ and Windows 11.

**Install:** double-click the `.exe` and follow the wizard. Done.

**Silent install** (e.g. for scripted machine setup):

```powershell
.\"Claude Consume and Peak Monitor_0.1.0_x64-setup.exe" /S
```

**Silent uninstall:**

```powershell
& "$env:LOCALAPPDATA\Programs\Claude Consume and Peak Monitor\uninstall.exe" /S
```

### Option 2 — MSI (for IT admins, GPO, Intune, SCCM)

**File:** `Claude Consume and Peak Monitor_0.1.0_x64_en-US.msi` (~4.5 MB)

- Standard Windows Installer package — perfect for managed deployment.
- Per-machine install (`ALLUSERS=1` is the WiX default), so it does
  prompt for admin elevation.
- Can be deployed via Group Policy, Intune Win32 app, SCCM, or any
  tooling that speaks `msiexec`.

**Silent install:**

```powershell
msiexec /i "Claude Consume and Peak Monitor_0.1.0_x64_en-US.msi" /qn /norestart
```

**Silent uninstall:**

```powershell
msiexec /x "Claude Consume and Peak Monitor_0.1.0_x64_en-US.msi" /qn /norestart
```

**Logged install** (useful for debugging deployment issues):

```powershell
msiexec /i "Claude Consume and Peak Monitor_0.1.0_x64_en-US.msi" /qn /l*v install.log
```

### Which one should I use?

| You are… | Use |
|---|---|
| A regular user installing on your own laptop | **NSIS `.exe`** |
| An IT admin pushing this to many machines | **MSI** |
| Installing on a machine without admin rights | **NSIS `.exe`** |
| Looking for the smallest download | **NSIS `.exe`** |

---

## After installing

1. Launch **Claude Consume and Peak Monitor** from the Start Menu.
2. The app drops into the system tray (look for the colored circle near
   the clock). The main window stays hidden — left-click the tray icon
   to open the popup, right-click to open the full dashboard.
3. By default the app auto-starts at login. You can disable that under
   *Dashboard → Settings → Auto-start at login*.

### Where things live

| Thing | Path |
|---|---|
| Application binary | `%LOCALAPPDATA%\Programs\Claude Consume and Peak Monitor\` (NSIS) or `%PROGRAMFILES%\Claude Consume and Peak Monitor\` (MSI) |
| User settings | `%APPDATA%\com.agencia-rse.claude-peak-monitor\store.json` |
| Source data the app reads | `~/.claude/projects/**/*.jsonl` (read-only) |

The app has **no** network access beyond the Anthropic status page, no
telemetry, and no analytics.

---

## Build from source

Requirements:

- Node.js 20+ and `pnpm`
- Rust stable toolchain (`rustup default stable`)
- Windows 10/11 with the Visual Studio Build Tools (C++ workload)
- For MSI bundling: WiX v3 (Tauri downloads it on first build)

```bash
git clone https://github.com/agenciaRSE/claude-peak-monitor.git
cd claude-peak-monitor
pnpm install

# Run in dev mode
pnpm tauri dev

# Produce installers
pnpm tauri build
```

Output:

```
src-tauri/target/release/
├── claude-peak-monitor.exe                                  # Portable binary
└── bundle/
    ├── nsis/Claude Consume and Peak Monitor_0.1.0_x64-setup.exe        # NSIS installer
    └── msi/Claude Consume and Peak Monitor_0.1.0_x64_en-US.msi         # MSI installer
```

To regenerate the icons (Python + Pillow required):

```bash
python scripts/generate_icons.py
```

---

## License

Copyright © 2026 Agencia RSE. All rights reserved.
