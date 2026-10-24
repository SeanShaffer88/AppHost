## TL;DR

**AppHost** is the WinDevKit app skeleton: a generator plus a Rust
template that starts a new Windows desktop app with single-instance,
tray, settings, logging, crash capture, background jobs, child-process
supervision, and an update check **already wired to the shipped
WinDevKit kits and covered by tests** — instead of hand-wiring all of
it again per app (MuseDesktop and ytTrayDL both did).

```sh
python3 scaffold-app.py --name DemoApp --out ../demo-app
cd ../demo-app
cargo test            # wiring tests through the real kits
cargo build --release # on Windows: the GUI app
```

## Quickstart

1. Scaffold: `scaffold-app.py --name NAME --out DIR [--id APPID]`.
   `APPID` defaults to a slug of the name and becomes the
   single-instance mutex suffix and the `%APPDATA%` folder name.
   The generator refuses to overwrite a non-empty directory.
2. Build the generated app: `cargo test`, then
   `cargo build --release` on Windows.
3. Run it: dark window + tray icon, demo job and demo child in the
   tray menu, settings and log under `%APPDATA%/<APPID>\`.

What works out of the box: single instance (second launch wakes the
first), tray menu, window-position persistence, a launch counter
proving settings round-trip end to end, house-format logging with a
diagnostic startup line, crash minidumps, a ~2 s progress job with
percent + ETA in the status line, a captured-output demo child, and
an update check that is **off** until `updates.manifest_url` is set
in settings — it never phones home by default.

## Wiring map

| Kit | Role in the skeleton | Code |
|---|---|---|
| SettingsKit | `settings.json` load/save (window position, launch counter, manifest URL) | `template/src/wiring/settings.rs` |
| LogKit | `app.log` house-format logging (startup milestone, demo actions) | `template/src/wiring/logging.rs` |
| CrashKit | Minidump + JSON sidecar into `crashes/` at startup install | `template/src/wiring/crash.rs` |
| JobKit | Demo job: 3 weighted steps, checkpoints, percent + ETA | `template/src/wiring/jobs.rs` |
| ProcessKit | Demo child (`cmd /C echo` / `/bin/echo`) with captured output | `template/src/wiring/proc.rs` |
| UpdateKit | Manifest check via `updatekit::check_update`; disabled with empty URL | `template/src/wiring/updates.rs` |
| *(app code)* | Single-instance mutex, window, tray menu in the wndproc | `template/src/main.rs` |
| *(app code)* | Known-folder data dir, composition root, self-check | `template/src/platform.rs`, `template/src/wiring/mod.rs`, `template/src/lib.rs` |

All six kits are really wired — five through the kits' native/FFI
Rust surfaces in their rlibs, UpdateKit through its native
`check_update`. Nothing is stubbed or mocked; the generated app's
integration tests (`template/tests/wiring.rs`) run the real kits.

## Pinned kit revisions

Recorded at template creation (2026-10-24) from `git ls-remote`;
ProcessKit and JobKit pin their `v0.1.0` tag targets, the older kits
had no tags so their current `main` revisions are pinned:

| Kit | Ref | Revision |
|---|---|---|
| SettingsKit | main | `8430800f252eaa0ee28fe35d846b87da6e39825f` |
| LogKit | main | `305ffbc732b615f2708541566816b526200571a8` |
| CrashKit | main | `0ab8e1533b0d7eb09fccbb913a77f779d5c7899e` |
| UpdateKit | main | `7b193e093e0efe8f2ef721cbc23adbf16f8d5fd2` |
| ProcessKit | tag `v0.1.0` | `01ea9b136ecec55f09129300d18d1edcd9d09a6e` |
| JobKit | tag `v0.1.0` | `1d9fd334f1573aa64d53b8a660a0c19dde33e524` |

Bump procedure: one kit at a time — update the `rev` in
`template/Cargo.toml`, read that kit's `AGENTS.md` for contract
changes, scaffold a smoke app, run its full gate, then update this
table.

## Honest limitations

- The generated GUI (window, tray menu, single-instance wake) is
  **build-verified in CI only**. Its runtime behaviour on a real
  Windows desktop is **Unverified** until someone runs it there —
  the TrayClick precedent: ship, then field-verify.
- The settings / logging / crash / job / process / update wiring
  **is test-verified** (host + CI) through the real kits. CrashKit
  is Windows-only by design; off Windows its install reports
  "unsupported" and startup continues.
- The template ships the stock Windows application icon, no
  installer, and no settings UI — see [ROADMAP.md](ROADMAP.md).
- AppHost's own CI scaffolds a smoke app from `template/` and gates
  *it*; the template does not compile in raw `{{token}}` form, by
  design.

## Layout

| Path | What |
|---|---|
| `scaffold-app.py` | Generator (mirrors WinDevKit's `scaffold-dll.py` UX) |
| `template/` | The app skeleton copied + token-substituted per app |
| `ci.yml` | AppHost CI: scaffold smoke app → build + test (Windows + Ubuntu) |
| `ROADMAP.md` | v0.2 plans |

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 Sean Shaffer.
