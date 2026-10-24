# AppHost Roadmap

## v0.1.0 (this release)

- `scaffold-app.py` generator mirroring `scaffold-dll.py`'s UX:
  `--name`, `--out`, optional `--id`; refuses non-empty targets.
- Template app with all six shipped kits really wired (SettingsKit,
  LogKit, CrashKit, JobKit, ProcessKit, UpdateKit), pinned by exact
  revision, plus single-instance mutex and a wndproc-handled tray.
- Generated-app integration tests through the real kits; AppHost CI
  scaffolds a smoke app and gates it on Windows + Ubuntu.

## v0.2 (planned)

- **Icon assets** — a real generated `.ico` wired into `app.rc`,
  the window class, and the tray icon, replacing the stock
  `IDI_APPLICATION` placeholder.
- **Installer template** — an Inno Setup script (per-user install,
  Start-menu shortcut, uninstaller, settings preserved) generated
  alongside the app, following the TrayClick release shape.
- **Settings UI page** — an in-app settings window editing the
  SettingsKit document (window behaviour, manifest URL) instead of
  hand-editing `settings.json`.
- **Dark titlebar polish** — `DwmSetWindowAttribute`
  dark-mode caption + a considered accent treatment, verified on a
  real desktop before it is claimed.
- Field-verify the v0.1 GUI runtime on a real Windows machine
  (tray menu, single-instance wake, minimize-to-tray) and record
  the results here.

## Later / unscheduled

- Optional kit features as scaffold flags (e.g. `--with-goldenkit`
  test scaffolding) once a second generated app proves the need.
- A `bump-kit.py` helper automating the one-at-a-time pin bump +
  smoke gate.
