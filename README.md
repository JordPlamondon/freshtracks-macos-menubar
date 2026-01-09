# FreshTracks Menu Bar

A macOS menu bar companion app for [FreshTracks](https://github.com/JordPlamondon/freshtracks). Start and stop timers without leaving your current window.

## Features

- Live timer display in the menu bar with elapsed time
- Weekly view of time entries with day-by-day navigation
- Start new timers by selecting client and project
- Stop, restart, edit, and delete entries
- Real-time sync with the web app via WebSockets
- Global keyboard shortcut (Cmd+Shift+T) to toggle timer
- Native macOS notifications
- Launches at login (optional)

## Tech Stack

- **Tauri 2.0** - Native app shell with Rust backend
- **Vue 3** - Frontend UI
- **Rust** - Menu bar integration, HTTP client, WebSocket handling

## Requirements

- macOS (menu bar apps are macOS-only)
- FreshTracks web app running locally or accessible via network
- Rust toolchain (for building)
- Node.js 18+

## Development

```bash
# Install dependencies
npm install

# Run in development mode (hot reload)
npm run tauri dev
```

## Building

```bash
# Build for production
npm run tauri build
```

The built app will be in `src-tauri/target/release/bundle/`.

## Configuration

The app connects to the FreshTracks API at `http://localhost:8000` by default. To change this, edit:

- `src/config.ts` - Frontend API URL
- `src-tauri/src/lib.rs` - Backend API and WebSocket URLs (search for `API_BASE`, `REVERB_HOST`)

## How It Works

The menu bar widget communicates with FreshTracks through a widget-specific API that uses a shared secret for authentication (no user login required). Timer events sync in real-time via Laravel Reverb WebSockets.

## License

MIT
