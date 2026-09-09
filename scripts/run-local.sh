#!/usr/bin/env bash
#
# Build everything and run CodeGraph locally.
#
# Usage:
#   ./scripts/run-local.sh [options]
#
# Options:
#   --rid <rid>         .NET RID to build the RoslynBridge bundle for.
#                       Auto-detected from uname when omitted.
#   --rebuild-bridge    Re-publish the RoslynBridge bundle even if one exists.
#   --skip-tests        Skip `cargo test` on roslyn-sys.
#   --skip-install      Skip `npm install` even if node_modules is missing.
#   --build             Build a release bundle (`tauri build`) instead of
#                       launching dev (`tauri dev`).
#   --check-only        Build everything but do not launch the app.
#   -h, --help          Show this help.
#
# Steps:
#   1. Detect the .NET RID for this machine (override with --rid).
#   2. Publish the self-contained RoslynBridge bundle (needs .NET SDK once;
#      skipped when the bundle already exists).
#   3. `cargo test` on packages/roslyn-sys (JSON fixtures, no dotnet needed).
#   4. `npm install` in apps/desktop (skipped when node_modules exists).
#   5. Launch `tauri dev` (or `tauri build` with --build).
#
# After the app opens, analyze `examples/OrderSystem` (a directory, .csproj,
# or .sln path) to see the sample call graph.
set -euo pipefail

RID=""
REBUILD_BRIDGE=0
SKIP_TESTS=0
SKIP_INSTALL=0
BUILD_MODE=0
CHECK_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rid) RID="${2:-}"; shift 2 ;;
    --rebuild-bridge) REBUILD_BRIDGE=1; shift ;;
    --skip-tests) SKIP_TESTS=1; shift ;;
    --skip-install) SKIP_INSTALL=1; shift ;;
    --build) BUILD_MODE=1; shift ;;
    --check-only) CHECK_ONLY=1; shift ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed -e 's/^# \?//' -e '/^set -euo pipefail$/d'; exit 0 ;;
    *) echo "unknown argument: $1 (see --help)" >&2; exit 2 ;;
  esac
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper_csproj="$repo_root/packages/roslyn-sys/managed/RoslynBridge/RoslynBridge.csproj"
frontend_dir="$repo_root/apps/desktop"

detect_rid() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Darwin) case "$arch" in
      arm64) echo "osx-arm64" ;;
      x86_64) echo "osx-x64" ;;
      *) echo "unsupported macOS arch: $arch" >&2; exit 1 ;;
    esac ;;
    Linux) case "$arch" in
      x86_64) echo "linux-x64" ;;
      aarch64|arm64) echo "linux-arm64" ;;
      *) echo "unsupported Linux arch: $arch" >&2; exit 1 ;;
    esac ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT) case "$arch" in
      x86_64|AMD64) echo "win-x64" ;;
      aarch64|arm64|ARM64) echo "win-arm64" ;;
      *) echo "unsupported Windows arch: $arch" >&2; exit 1 ;;
    esac ;;
    *) echo "unsupported OS: $os" >&2; exit 1 ;;
  esac
}

if [[ -z "$RID" ]]; then
  RID="$(detect_rid)"
fi

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1 ($2)" >&2
    exit 1
  fi
}

need cargo "install Rust via https://rustup.rs"
need node  "install Node.js 18+ via https://nodejs.org"
need npm   "ships with Node.js"

echo "==> CodeGraph local run (rid=$RID)"

# 1. RoslynBridge self-contained bundle (embeds the .NET runtime, so the app
#    runs without .NET installed; building it needs the .NET SDK once).
bundle_exe="$repo_root/packages/roslyn-sys/managed/RoslynBridge/bin/Release/net10.0/$RID/publish/RoslynBridge"
if [[ "$REBUILD_BRIDGE" -eq 1 || ! -x "$bundle_exe" ]]; then
  need dotnet "install the .NET 10 SDK to build the analyzer helper once"
  echo "==> Publishing RoslynBridge ($RID)..."
  "$repo_root/scripts/build-roslyn-bridge.sh" "$RID"
else
  echo "==> RoslynBridge bundle exists, skipping publish ($bundle_exe)"
fi
if [[ ! -x "$bundle_exe" ]]; then
  echo "RoslynBridge bundle still missing at $bundle_exe" >&2
  exit 1
fi

# 2. Rust tests (fixture-based, no dotnet needed).
if [[ "$SKIP_TESTS" -eq 1 ]]; then
  echo "==> Skipping cargo tests (--skip-tests)"
else
  echo "==> Running roslyn-sys tests..."
  cargo test --manifest-path "$repo_root/packages/roslyn-sys/Cargo.toml"
fi

# 3. Frontend dependencies.
if [[ "$SKIP_INSTALL" -eq 1 ]]; then
  echo "==> Skipping npm install (--skip-install)"
elif [[ -d "$frontend_dir/node_modules" ]]; then
  echo "==> node_modules exists, skipping npm install"
else
  echo "==> Installing frontend dependencies..."
  npm --prefix "$frontend_dir" install
fi

# 4. Launch (or just verify with --check-only).
if [[ "$CHECK_ONLY" -eq 1 ]]; then
  echo "==> Check-only: everything built, not launching."
  exit 0
fi

if [[ "$BUILD_MODE" -eq 1 ]]; then
  echo "==> Building release bundle..."
  npm --prefix "$frontend_dir" run tauri:build
else
  echo "==> Starting CodeGraph (tauri dev)..."
  echo "    When the app opens, analyze: $repo_root/examples/OrderSystem"
  npm --prefix "$frontend_dir" run tauri:dev
fi
