#!/usr/bin/env bash
#
# Stage the self-contained RoslynBridge bundle for the current platform into
# the Tauri app so it ships inside the installer (`bundle.resources` in
# `tauri.conf.json` picks up `apps/desktop/src-tauri/resources/`).
#
# Usage:
#   ./scripts/collect-roslyn-bridge.sh <rid>
#
#   <rid>  one of osx-arm64, osx-x64, win-x64, win-arm64, linux-x64,
#          linux-arm64
#
# Requires the bundle to already be published for <rid> (see
# `scripts/build-roslyn-bridge.sh`). The staged copy is gitignored and
# regenerated on every CI/dev build.
set -euo pipefail

rid="${1:-}"
if [[ -z "$rid" ]]; then
  echo "usage: $0 <rid>" >&2
  echo "rid must be one of: osx-arm64 osx-x64 win-x64 win-arm64 linux-x64 linux-arm64" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
publish_dir="$repo_root/packages/roslyn-sys/managed/RoslynBridge/bin/Release/net10.0/$rid/publish"
dest_dir="$repo_root/apps/desktop/src-tauri/resources/roslyn-bridge"

if [[ ! -d "$publish_dir" ]]; then
  echo "RoslynBridge bundle missing at $publish_dir" >&2
  echo "Publish it first: ./scripts/build-roslyn-bridge.sh $rid" >&2
  exit 1
fi

rm -rf "$dest_dir"
mkdir -p "$dest_dir"
# Trailing `/.` copies contents (including dotfiles) and preserves exec bits.
cp -R "$publish_dir/." "$dest_dir/"

echo "Staged RoslynBridge ($rid) into $dest_dir"
ls "$dest_dir"
