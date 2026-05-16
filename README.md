# Claude Usage Widget

A minimal desktop widget that shows your **Claude Max subscription usage** — 5-hour and 7-day token limits — with live countdown timers to the next reset.

Built with [Tauri 2](https://v2.tauri.app/) (Rust + vanilla JS). No API keys to configure; credentials are read automatically from your browser.

---

## Features

- Live countdown to limit reset (updates every second)
- Progress bars with color thresholds: normal → warning (70 %) → danger (90 %)
- Auto-refresh every 2 minutes
- Pin-on-top toggle — keep the widget visible above all other windows
- State persists across restarts (pin preference saved in `localStorage`)

---

## Platform support

| Platform | Status  | Notes |
|----------|---------|-------|
| macOS    | ✅ Full | Reads cookies from Claude desktop app, Chrome, Brave, Edge, Chromium, or Firefox |
| Windows  | 🚧 Planned | Cookie decryption via DPAPI not yet implemented |

---

## macOS

### Requirements

- macOS 11 Big Sur or later
- [Rust](https://rustup.rs/) (stable channel)
- Xcode Command Line Tools: `xcode-select --install`
- Python 3 and OpenSSL — both ship with macOS by default
- A **Claude Max** subscription
- Logged in to **claude.ai** in at least one of: Claude desktop app, Chrome, Brave, Edge, Chromium, or Firefox

### Build and run

```bash
# Clone the repo
git clone <repo-url>
cd wcte

# Development build with DevTools
cargo tauri dev

# Production build
cargo tauri build
# → src-tauri/target/release/bundle/macos/Claude Token Counter.app
```

### Auto-start at login

1. Build: `cargo tauri build`
2. Copy `Claude Token Counter.app` to `/Applications`
3. Open **System Settings → General → Login Items & Extensions**
4. Click **+** under "Open at Login" and select **Claude Token Counter.app**

The widget will open automatically on every login.

### How credentials are read

The widget searches for your `sessionKey` cookie in the following order, stopping at the first match:

1. Claude desktop app — `~/Library/Application Support/Claude/Cookies`
2. Google Chrome — `~/Library/Application Support/Google/Chrome/Default/Cookies`
3. Brave — `~/Library/Application Support/BraveSoftware/Brave-Browser/Default/Cookies`
4. Microsoft Edge — `~/Library/Application Support/Microsoft Edge/Default/Cookies`
5. Chromium — `~/Library/Application Support/Chromium/Default/Cookies`
6. Firefox — `~/Library/Application Support/Firefox/Profiles/*/cookies.sqlite` (unencrypted)

For Chromium-based browsers, cookies are encrypted with AES-128-CBC. The widget decrypts them using:
- The browser's safe-storage key from macOS Keychain
- PBKDF2-HMAC-SHA1 key derivation (salt: `saltysalt`, 1003 iterations, 16-byte key)
- Embedded IV from the cookie's `v10` format

No credentials are ever written to disk or sent anywhere except claude.ai.

### Troubleshooting (macOS)

| Error | Fix |
|-------|-----|
| `sessionKey not found` | Log in to [claude.ai](https://claude.ai) in Chrome, Brave, Edge, or Firefox, then click Refresh |
| `Claude Safe Storage not found` | Log out and back in to claude.ai in your browser |
| `Auth failed (403)` | Your session has expired — log in to claude.ai again |
| `API error 404` | The cached org ID was stale; it will be cleared automatically — click Refresh |
| No numbers visible | Check the status bar at the bottom of the widget for the error message |

---

## Windows

> **Windows support is not yet implemented.**
>
> The current cookie-reading logic uses macOS-specific tools (`security`, macOS Keychain paths). Windows support requires:
> - DPAPI-based decryption for Chrome/Edge/Brave cookies
> - Updated cookie paths (`%LOCALAPPDATA%\Google\Chrome\User Data\Default\...`)
> - Updated Cargo features (remove `apple-native` from `keyring`)
>
> Contributions welcome.

---

## Project structure

```
wcte/
├── src/
│   ├── index.html      # Widget UI
│   ├── main.js         # Tauri bridge, refresh loop, pin toggle
│   └── styles.css      # Dark/light theme, progress bars
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/
    │   └── default.json
    └── src/
        └── lib.rs      # Cookie decryption, org ID cache, usage API
```

---

## License

MIT
