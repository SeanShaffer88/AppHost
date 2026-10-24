#!/usr/bin/env python3
"""Scaffold a new Windows desktop app from AppHost's template.

Usage:
  scaffold-app.py --name DemoApp --out ~/workspace/demo-app
  scaffold-app.py --name "Demo App" --id com.example.demo --out ./demo

The template is copied to OUT and these tokens are substituted in text
file contents and in output paths:

  {{APP_NAME}}  package/binary-safe slug derived from --name
  {{APP_ID}}    --id, or the same slug by default (mutex + data folder)
  {{VERSION}}   0.1.0

An existing non-empty OUT is refused. Nothing outside OUT is written.
"""

import argparse
import os
import re
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
TEMPLATE = os.path.join(ROOT, "template")
VERSION = "0.1.0"
SKIP_DIRS = {"target", "__pycache__", ".git"}


def slugify(name):
    """Return a Cargo-package-safe slug for a display/app name."""
    value = name.strip().replace("_", "-")
    value = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "-", value)
    value = re.sub(r"(?<=[A-Z])(?=[A-Z][a-z])", "-", value)
    value = re.sub(r"[^A-Za-z0-9]+", "-", value)
    value = re.sub(r"-+", "-", value).strip("-").lower()
    if not value:
        raise ValueError("name does not contain any letters or digits")
    if value[0].isdigit():
        value = "app-" + value
    return value


def valid_app_id(app_id):
    return bool(app_id) and re.fullmatch(r"[A-Za-z0-9._-]+", app_id) is not None


def substituted(text, tokens):
    for token, value in tokens.items():
        text = text.replace(token, value)
    return text


def collect_outputs(out, tokens):
    outputs = []
    seen = set()
    for root, dirs, files in os.walk(TEMPLATE):
        dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS)
        for filename in sorted(files):
            src = os.path.join(root, filename)
            rel = os.path.relpath(src, TEMPLATE)
            rel = substituted(rel, tokens)
            if rel in seen:
                raise ValueError(f"template paths collide after substitution: {rel}")
            seen.add(rel)
            dst = os.path.join(out, rel)
            with open(src, "rb") as fh:
                data = fh.read()
            try:
                text = data.decode("utf-8")
            except UnicodeDecodeError:
                payload = data
            else:
                payload = substituted(text, tokens).encode("utf-8")
            outputs.append((dst, payload))
    return outputs


def main():
    parser = argparse.ArgumentParser(description="Scaffold an AppHost Windows app")
    parser.add_argument("--name", required=True, help="app name, e.g. DemoApp")
    parser.add_argument("--id", dest="app_id", help="app id (default: slug of --name)")
    parser.add_argument("--out", required=True, help="new app directory")
    args = parser.parse_args()

    try:
        app_name = slugify(args.name)
    except ValueError as exc:
        sys.exit(f"invalid --name: {exc}")
    app_id = args.app_id or app_name
    if not valid_app_id(app_id):
        sys.exit("invalid --id: use only letters, digits, dot, underscore, and hyphen")

    out = os.path.abspath(os.path.expanduser(args.out))
    if os.path.exists(out):
        if not os.path.isdir(out):
            sys.exit(f"refusing to overwrite existing file: {out}")
        if os.listdir(out):
            sys.exit(f"refusing to overwrite non-empty directory: {out}")

    tokens = {
        "{{APP_NAME}}": app_name,
        "{{APP_ID}}": app_id,
        "{{VERSION}}": VERSION,
    }
    try:
        outputs = collect_outputs(out, tokens)
    except (OSError, ValueError) as exc:
        sys.exit(f"cannot read template: {exc}")

    try:
        os.makedirs(out, exist_ok=True)
        for dst, payload in outputs:
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            with open(dst, "wb") as fh:
                fh.write(payload)
            print("wrote", dst)
    except OSError as exc:
        sys.exit(f"cannot write app: {exc}")

    print(f"Done. App: {app_name} ({app_id}), version {VERSION}")
    print("Next steps:")
    print(f"  cd {out}")
    print("  cargo test")
    print("  cargo build --release")


if __name__ == "__main__":
    main()
