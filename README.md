# CodeGraph

A desktop developer tool that visualizes **method-level dependencies** in a
codebase. Methods are graph nodes; calls between them are directed edges.
Selecting a method highlights its callers and callees.

Initially supports **C#/.NET** (Roslyn). The frontend is React + React Flow,
the backend is Rust (Tauri), and the analyzer is a managed Roslyn helper run
as a subprocess.

```
apps/desktop            Tauri v2 desktop app (React 18 + reactflow 11 frontend,
                        Rust backend in src-tauri/)
packages/roslyn-sys     Rust crate + C# managed helper that wrap Roslyn
examples/OrderSystem   Sample .NET 8 project used to try the tool
scripts/                Build tooling (e.g. self-contained helper bundles)
```

## Quick start

Requirements: **Rust**, **Node.js**, and — only to build the analyzer helper —
the **.NET SDK**. End users do **not** need .NET installed: the helper ships as
self-contained bundles (one per OS) that embed the runtime, and the app picks
the right one automatically.

```sh
# 1. Build the C# helper as a self-contained bundle for your OS. Requires
#    the .NET SDK. (RIDs: osx-arm64, osx-x64, win-x64, win-arm64,
#    linux-x64, linux-arm64.)
./scripts/build-roslyn-bridge.sh osx-arm64

#    Run it again for any other OS you want a bundle for, e.g.:
#    ./scripts/build-roslyn-bridge.sh win-x64
#    ./scripts/build-roslyn-bridge.sh linux-x64

# 2. Rust tests (no dotnet needed — uses JSON fixtures)
cargo test --manifest-path packages/roslyn-sys/Cargo.toml

# 3. Frontend dev deps
npm --prefix apps/desktop install

# 4. Run the app
npm --prefix apps/desktop run tauri:dev
```

> Skip step 1 if bundles already exist in
> `packages/roslyn-sys/managed/RoslynBridge/bin/Release/net10.0/<rid>/publish/`.
> Missing bundles surface a clear error at analyze time, pointing at the script.

Then type the path to `examples/OrderSystem` (a directory, `.csproj`, or
`.sln`) and click **Analyze**.

## More

The engineering handoff with the full architecture, data flow, ID scheme,
known limitations, and next steps lives in **[CLAUDE.md](./CLAUDE.md)** — read
that before continuing development.
