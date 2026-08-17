#!/usr/bin/env bash
#
# Publish the managed Roslyn helper as a self-contained bundle for a single
# OS. The bundle embeds the .NET runtime, so end users of CodeGraph never
# need a .NET SDK/runtime installed — the app simply spawns the bundle's
# native executable for the current OS.
#
# Build-time requirement only: the .NET SDK (to publish the bundle).
#
# Usage:
#   ./scripts/build-roslyn-bridge.sh <rid>
#
#   <rid>  one of osx-arm64, osx-x64, win-x64, win-arm64, linux-x64,
#          linux-arm64
#
#   ./scripts/build-roslyn-bridge.sh osx-arm64
#
# The bundle is written to
#   packages/roslyn-sys/managed/RoslynBridge/bin/Release/net10.0/<rid>/publish/
set -euo pipefail

rid="${1:-}"
if [[ -z "$rid" ]]; then
  echo "usage: $0 <rid>" >&2
  echo "rid must be one of: osx-arm64 osx-x64 win-x64 win-arm64 linux-x64 linux-arm64" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper_csproj="$repo_root/packages/roslyn-sys/managed/RoslynBridge/RoslynBridge.csproj"

echo "Publishing RoslynBridge ($rid)..."
dotnet publish "$helper_csproj" -c Release -r "$rid" --self-contained true -v minimal

echo "Done. Bundle is in packages/roslyn-sys/managed/RoslynBridge/bin/Release/net10.0/$rid/publish/"
