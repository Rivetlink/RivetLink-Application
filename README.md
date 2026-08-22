# RivetLink Application

Cross-platform desktop app for [RivetLink](https://github.com/Rivetlink/RivetLink) —
a zero-trust, end-to-end-encrypted remote-control platform.

Built with **Tauri 2** (Rust backend) + **Vue 3** + **Vuetify**. The Rust
backend is a thin layer over `rivetlink-sdk`, so the app, the `rivet-client`
CLI, and third-party integrators all share one codebase. The server is never a
trusted authority; session content is encrypted end to end.

## Status

Client-mode MVP: configure a relay → sign in → list devices → capture one
screenshot from a host (the host may prompt its operator to approve). Live
video, input control, and Host mode are on the roadmap.

On **Ubuntu Desktop 24.04 LTS or newer**, use **Settings → Ubuntu physical
console** with a permanent HDMI dummy/EDID emulator. After one owner-approved
PolicyKit action, RivetLink starts a non-root broker before Ubuntu login and
captures the real logged-in GNOME seat0 session through a narrowly scoped
worker. Stock GNOME/Mutter intentionally does not permit RivetLink to capture
or inject input into the existing physical GDM login display; GNOME's supported
Remote Login uses a separate headless RDP display instead.
Setup offers direct **Local network**, **Via relay**, or both. Local network
uses a pinned, encrypted trusted-device connection and needs no relay, account
or internet; relay registration uses the current signed-in session, so no relay
token is shown, copied or stored in a service. It does not enable automatic
login, remote shell, RDP/VNC or file access. See the main repository's
`docs/ubuntu-physical-console-broker.md`.

## Prerequisites

- [Node.js](https://nodejs.org/) 20+ and npm
- The [Rust toolchain](https://rustup.rs/)
- Tauri's OS dependencies — on Debian/Ubuntu:

  ```bash
  sudo apt update && sudo apt install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev \
    libjavascriptcoregtk-4.1-dev build-essential curl wget file libssl-dev \
    libayatana-appindicator3-dev
  ```

  (macOS: Xcode command-line tools. Windows: WebView2 + MSVC build tools.)
- A sibling checkout of the main `RivetLink` repo next to this one — the
  backend depends on `../RivetLink/crates/rivetlink-sdk` via a path dependency.

## Develop

```bash
npm install
npm run tauri dev      # launches the app with hot-reload
```

## Lint

ESLint 9 (flat config) + `@stylistic` owns formatting (4-space indent, double
quotes, semicolons, object properties each on their own line).

```bash
npm run lint           # check
npm run lint:fix       # autofix
```

## Build

```bash
npm run tauri build    # native installer for the current OS
```

## Layout

```
.
├── src/                 Vue 3 + Vuetify frontend
│   ├── App.vue          client-mode UI (connect → login → devices → capture)
│   └── main.ts          app + Vuetify bootstrap
├── src-tauri/           Rust backend (Tauri 2)
│   └── src/lib.rs       Tauri commands over rivetlink-sdk
└── eslint.config.js     house style
```
