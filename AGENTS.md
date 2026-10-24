# AGENTS.md — AppHost

Working notes for anyone (human or agent) changing AppHost or an app
generated from it. The load-bearing items come first.

## Load-bearing

- **Tray events are handled inside the window procedure.** The tray
  callback message (`WM_APP + 1`) is matched in `wnd_proc` in
  `template/src/main.rs` and the menu is shown/handled there.
  ytTrayDL lesson: tray events delivered by polling a global
  receiver channel from the UI loop were silently dead in
  production. **Never** move tray handling to a polled channel,
  timer check, or background thread.
- **Single-instance naming is contractual.** Mutex:
  `Local\<APP_ID>-SingleInstance`; wake message:
  `<APP_ID>-Wake` (registered window message); window class:
  `<APP_ID>-MainWindow`. Apps generated with different `APP_ID`s
  must never collide; the same app must always collide with itself.
- **Kits are pinned by exact ref** (`rev` in `template/Cargo.toml`)
  and recorded in the root README's pin table. Bump **one kit at a
  time**: update the rev, read that kit's `AGENTS.md` for contract
  changes, scaffold a smoke app, run its full gate
  (fmt / clippy host + windows-msvc / full test), then update the
  table. Never float a kit on a branch.
- **Never re-implement a kit inside the template.** If behaviour is
  missing, it belongs in the kit repo (or is a documented template
  gap in ROADMAP.md). The template's wiring modules stay thin
  adapters; each names the kit and exact API it uses in its header.
- **Update check never phones home by default.** Empty
  `updates.manifest_url` = disabled, and the disabled path returns
  before UpdateKit is called. Never invent a default URL.
- **Defaults are never written by reads** (SettingsKit rule): new
  settings keys get their default in `AppSettings::default()` and
  are only persisted when the document is saved for another reason.

## Generator rules

- `scaffold-app.py` substitutes exactly `{{APP_NAME}}`, `{{APP_ID}}`,
  `{{VERSION}}` in text contents and paths, refuses a non-empty
  target, and writes nothing outside the target. A generated tree
  must contain no `{{` — grep it after every template change.
- The template does not compile raw (placeholders in `Cargo.toml`).
  Gate the **generated smoke app**, like `ci.yml` does; never
  "fix" this by hardcoding a name into the template.

## Gate

- Repo: `python3 -m py_compile scaffold-app.py`, then scaffold a
  smoke app to /tmp and in it: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings` (host),
  `cargo clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings`,
  `cargo check --target x86_64-pc-windows-msvc --all-targets`,
  `cargo test` (host, full).
- GUI code is compile-checked for windows-msvc locally and linked
  only in CI. Runtime GUI behaviour stays **Unverified** until run
  on a real Windows desktop — say so; do not upgrade the label from
  build evidence.
