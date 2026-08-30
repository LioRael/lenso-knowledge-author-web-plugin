#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

rg -q '^publish = true$' Cargo.toml
cargo_command="${LENSO_CARGO:-cargo}"
package_files="$("$cargo_command" package --list --locked --allow-dirty)"
printf '%s\n' "$package_files" | rg -q '^Cargo\.toml$'
printf '%s\n' "$package_files" | rg -q '^LICENSE$'
printf '%s\n' "$package_files" | rg -q '^README\.md$'
printf '%s\n' "$package_files" | rg -q '^src/lib\.rs$'
printf '%s\n' "$package_files" | rg -q '^src/assets\.rs$'

if printf '%s\n' "$package_files" | rg -q '^target/|^\.git/'; then
  echo "Build or repository internals must not enter the public crate." >&2
  exit 1
fi
