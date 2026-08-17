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
```

## Quick start

Requirements: **.NET 8 SDK**, **Rust**, **Node.js** (all must be on `PATH`).

```sh
# C# helper (also auto-built on first analysis)
dotnet build packages/roslyn-sys/managed/RoslynBridge/RoslynBridge.csproj -c Release

# Rust tests (no dotnet needed — uses JSON fixtures)
cargo test --manifest-path packages/roslyn-sys/Cargo.toml

# Frontend dev deps
npm --prefix apps/desktop install

# Run the app
npm --prefix apps/desktop run tauri:dev
```

Then type the path to `examples/OrderSystem` (a directory, `.csproj`, or
`.sln`) and click **Analyze**.

## More

The engineering handoff with the full architecture, data flow, ID scheme,
known limitations, and next steps lives in **[CLAUDE.md](./CLAUDE.md)** — read
that before continuing development.