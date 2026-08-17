# CLAUDE.md — CodeGraph engineering handoff

Context memory for agents continuing work on CodeGraph. Read this first.
Also see [README.md](./README.md) for the one-paragraph summary and quick start.

> **State as of this doc:** the vertical slice is implemented end-to-end in
> code, but it has **never been compiled or run** on a machine with the
> toolchain (no `dotnet`/`cargo`/`node` were available at write time). It was
> reviewed manually against the APIs of Tauri 2, React Flow 11, and
> Roslyn 4.8. Treat the first task as *make it build and run* (see
> "Verification").

---

## 1. The idea

- Analyze a C#/.NET codebase and render a directed **method-call graph**.
- Nodes = methods; edges = CALLS relationships.
- Selecting a method visually highlights the selected node, its callers, its
  callees, and the connecting edges; everything else is de-emphasized.
- The whole point: understand local dependency relationships without guessing.
  Call resolution must use Roslyn's **semantic model** (the actual bound
  symbol), not name matching — so overloads, same-named methods in different
  types, namespaces, and generics are handled correctly.

Example target graph shape:

```
OrderProcessor.Process(int)
       |  CALLS
       v
  OrderService.GetOrder(int)
       |  CALLS
       v
  OrderRepository.FindById(int)
```

## 2. Repository layout

```
apps/desktop/                 Tauri v2 app
  src/                        React + TypeScript frontend
    main.tsx, App.tsx
    api/tauriClient.ts        the ONLY file that calls Tauri `invoke`
    domain/method.ts          frontend mirror of the graph domain types
    graph/adapter.ts          domain graph -> React Flow nodes/edges + selection
    graph/useGraphData.ts     state hook; owns loaded graph + selection
    components/GraphView.tsx  React Flow canvas (presentation only)
    components/MethodNode.tsx custom node renderer (reads data.role)
    components/Sidebar.tsx    open-path form + selected method's callers/callees
  src-tauri/src/
    main.rs, lib.rs           entry; manages AppState { graph: Arc<GraphService> }
    ipc/commands.rs           Tauri command surface (thin)
    app/graph_service.rs      owns current graph, answers read queries
    app/errors.rs             AppError (serialized to frontend)
    domain/                   language-independent graph model (Rust)
    analysis/                 C# analyzer orchestration + solution loader

packages/roslyn-sys/          Rust <-> Roslyn bridge crate
  src/bridge.rs               resolves & spawns the managed helper; parses JSON
  src/dto.rs                  wire-format DTOs -> roslyn-sys domain types
  src/domain/                 mirror of the graph domain (keeps crate standalone)
  src/Cargo.toml              no coreclr dep anymore (subprocess strategy)
  managed/RoslynBridge/
    RoslynBridge.csproj       net8.0 Exe, UseAppHost=false
    Bridge.cs                 the actual Roslyn analyzer (AdhocWorkspace)

examples/OrderSystem/         sample .NET 8 project to try the tool on
```

There is **no root Cargo workspace** — `apps/desktop/src-tauri` and
`packages/roslyn-sys` are independent crates; the desktop crate depends on
roslyn-sys by path. `docs/` and `scripts/` are empty. The git branch `init`
has **no commits yet** (all code is untracked).

## 3. Architecture & boundaries (keep these)

| Layer | Location | Depends on |
|---|---|---|
| React UI | `apps/desktop/src/components` | frontend domain types via props only |
| Graph presentation | `apps/desktop/src/graph` | React Flow types are constructed only here (adapter) |
| Tauri communication | `apps/desktop/src/api` + `src-tauri/src/ipc` | thin |
| Rust application | `src-tauri/src/app` | domain types |
| Domain model | `src-tauri/src/domain` | language-independent, no Roslyn/React/ReactFlow/SQLite |
| C# analysis | `packages/roslyn-sys` | Roslyn, isolated behind JSON |

Rules that must not be broken:
- **Roslyn never leaks.** The managed helper returns JSON only. `roslyn-sys`
  exposes domain value types, never `IMethodSymbol`.
- **React Flow never leaks** into domain/backend code. The frontend converts
  domain graphs to `Node[]`/`Edge[]` only in `graph/adapter.ts`.
- Frontend and backend do **not** duplicate business logic. Callers/callees
  are computed on the backend (`GraphService`) and re-derivable from edges via
  `domain/method.ts` `graphOps` for the local adapter highlight.
- Stable string **IDs** identify methods everywhere (see §5); display names
  are cosmetic.
- Commands are query-oriented and independent of React Flow shapes:
  `analyze_solution`, `get_graph`, `get_method`, `get_callers`, `get_callees`.

### Data flow (one full analysis)

```
Sidebar "Analyze" (path)
 -> tauriClient.analyzeSolution(path)
 -> invoke("analyze_solution", { args: { path } })     // note `args:` wrapper
 -> ipc:analyze_solution -> GraphService.analyze
 -> CSharpAnalyzer.analyze_path
     - SolutionLoader.discover_projects   (dotnet sln list / direct .csproj)
     - roslyn_sys::Bridge::init()         (ensure dotnet; locate or build helper)
     - per project: Bridge.analyze_to_json(csproj)
         = spawn `dotnet <RoslynBridge.dll> <csproj>`
         = C# Bridge.Main -> AnalyzeCore -> BuildGraph
         = JSON { "methods": [...], "edges": [...], "error": "..." } on stdout
     - parse_graph -> dto::AnalysisGraph -> into_domain() [roslyn-sys domain]
     - .into() via From impls            [roslyn-sys domain -> desktop domain]
     - merge_into across projects
 -> LoadedGraph { source_path, graph }   -> JSON -> frontend
Frontend:
 -> useGraphData stores LoadedGraph
 -> GraphView: adapter.toReactFlowNodes/Edges -> applySelection(base, selectedId)
 -> node click -> select(id) -> parallel get_method/get_callers/get_callees
 -> adapter re-highlights via roles
```

### Tauri argument shapes (frontend contract)

Command args must match the Rust parameter names. The commands take a single
`args` struct, so calls look like `invoke("get_method", { args: { id } })`.
The commands run against `State<AppState>`, not `State<GraphService>` — the
commands deref `state.graph` (an `Arc<GraphService>`). Keep it that way.

## 4. The C# analyzer (`managed/RoslynBridge/Bridge.cs`)

Runs as `dotnet RoslynBridge.dll <path>`; prints exactly one JSON doc to
stdout. Reference notes:

- **AdhocWorkspace** project created from the .csproj path. All `*.cs` under
  the project dir are added (skipping `bin`/`obj`). After adding documents it
  re-fetches `workspace.CurrentSolution.GetProject(id)` because every
  `AddDocument` snapshots a new solution.
- **Runtime references**: enumerates `RuntimeEnvironment.GetRuntimeDirectory()`
  (`*.dll`) as metadata references so the semantic model can bind BCL members.
  Framework code is *filtered out* of the graph, so this never adds BCL nodes.
- **Pass 1** — nodes: every `BaseMethodDeclarationSyntax` (methods, ctors,
  operators) declared in source becomes a node.
- **Pass 2** — edges: `InvocationExpressionSyntax` and
  `ObjectCreationExpressionSyntax`, resolved via
  `GetSymbolInfo(...).Symbol as IMethodSymbol`. Enclosing context is the
  nearest `BaseMethodDeclarationSyntax`.
- **Filter**: only symbols with `Locations.Any(IsInSource)` become nodes or
  edge endpoints. Implicitly-declared ctors (e.g. record/parameterless default
  ctors) are skipped as targets.
- Symbol-to-ID mapping uses `SymbolEqualityComparer.Default` so overloads are
  distinct and call resolution is exact.
- Result DTO field names are snake_case and match the Rust DTOs exactly.

## 5. Stable IDs

`csharp:<fully-qualified-signature>` where the signature includes
fully-qualified parameter types, e.g.

```
csharp:OrderSystem.Services.OrderRepository.FindById(int)
csharp:OrderSystem.Services.OrderRepository.FindById(int, bool)
```

The parameter list is what disambiguates overloads (same arity, different
types) and same-named methods in different namespaces/types. IDs are opaque
strings — the domain never parses them.

> Implementation note: Roslyn 4.x `SymbolDisplayFormat.FullyQualifiedFormat`
> silently omits the containing type (and parameter list) for *method*
> symbols, so the helper builds FQNs/IDs from
> `SymbolDisplayFormat.CSharpErrorMessageFormat` instead (probe-verified).

## 6. Bridge resolution & build

`Bridge::init()`:
1. env `CODEGRAPH_ROSLYN_BRIDGE` → the DLL directly, else
2. `<repo>/packages/roslyn-sys/managed/RoslynBridge/bin/{Release,Debug}/net8.0/RoslynBridge.dll`,
   else
3. runs `dotnet build -c Release` of the helper project once.

Requires .NET 8 SDK + NuGet restore on first build. `dotnet` failure produces
a clear error surfaced to the UI.

## 7. Frontend highlighting

- `adapter.ts` computes `role: "default" | "selected" | "caller" | "callee"`
  for nodes, and per-role inline `edge.style` (React Flow v11 applies
  `edge.style` to the edge path; **`[data-role]` CSS attribute selectors do
  NOT work** — do not reintroduce them).
- Colors: selected `--accent`, callers `--accent-caller`, callees
  `--accent-callee` (see `components/styles.css`). Unrelated nodes/edges
  render at reduced opacity.
- Layout is currently a fixed grid in the adapter; a real layout (dagre/elk)
  belongs in the adapter so GraphView stays untouched.

## 8. Expected output for `examples/OrderSystem`

9 methods, 7 edges:

```
nodes: Program.Main, OrderService..ctor(OrderRepository),
       OrderService.GetOrder(int), OrderService.Print(Order),
       OrderService.PrintFooter(), OrderRepository.FindById(int),
       OrderRepository.FindById(int, bool),
       OrderProcessor..ctor(OrderService), OrderProcessor.Process(int)

edges: Main -> OrderProcessor..ctor
       Main -> OrderService..ctor
       Main -> OrderProcessor.Process(int)
       GetOrder(int) -> OrderRepository.FindById(int)
       Print(Order) -> PrintFooter()
       Process(int) -> GetOrder(int)
       Process(int) -> Print(Order)
```

BCL/synthetic targets (`Console.WriteLine`, `Dictionary.TryGetValue`, the
implicit `Order` record ctor) are correctly excluded. `FindById(int)` vs
`FindById(int, bool)` are distinct nodes. Use this as a smoke-test oracle.

## 9. Known limitations (documented-in-code, don't re-discover)

- **No project references**: each project is compiled standalone in the
  helper; calls into *other* projects of an sln aren't resolved (Roslyn
  runtime refs alone don't provide sibling assemblies). Cross-project edges
  are currently missed — likely the highest-value improvement (see §10).
- **File inclusion heuristic**: helper globs `*.cs` recursively under the
  project dir (skip bin/obj). Linked files, multi-targeting subdirs, and
  files outside the dir are missed.
- **Helper .sln handling**: when handed a `.sln` it analyzes only the first
  project. The Rust loader normally passes `.csproj` paths, so this is just
  defensive/dead in the current flow.
- Only CALLS relationships exist (`RelationshipKind::Calls`); the enum is
  ready for overrides/interface-impl kinds.
- Grid layout only; no fit-to-content after selection (initial `fitView`).
- Repeated `dotnet` spawn per project — slow for large solutions.
- `packages/roslyn-sys/src/analysis.rs` is a reserved placeholder (empty).

## 10. Suggested next steps (in priority order)

1. **Build & run it** on a machine with Rust + Node + .NET 8 (`cargo build`,
   `cargo test`, `npm run build`, `tauri dev`), fix whatever the manual
   review missed, and confirm §8's oracle for `examples/OrderSystem`.
2. **Project references**: pass the list of sibling project paths to the
   helper (or use `MSBuildWorkspace` / `dotnet` metadata refs) so
   cross-project edges resolve.
3. **Automated tests**: Rust unit tests (graph_service, commands), C# helper
   golden tests over the example, and a frontend component test for
   `applySelection` roles.
4. **Layout**: replace the grid in `adapter.toReactFlowNodes` with a proper
   layered layout so the CALLS direction reads top-to-bottom.
5. **More relationship kinds** (overrides, implements) and then a second
   language analyzer behind the same `analysis` module seam.
6. Optional: SQLite persistence of graphs (the domain is already
   infra-independent; a storage module can be added without touching domain
   types).

## 11. Commands cheat-sheet

```sh
# C# helper
dotnet build packages/roslyn-sys/managed/RoslynBridge/RoslynBridge.csproj -c Release

# Rust crates (separate — no root workspace)
cargo build --manifest-path packages/roslyn-sys/Cargo.toml
cargo test  --manifest-path packages/roslyn-sys/Cargo.toml
cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml

# Frontend
npm --prefix apps/desktop install
npm --prefix apps/desktop run tauri:dev    # dev servers + tauri window
npm --prefix apps/desktop run tauri:build  # release bundle
```

The `dto.rs` tests use `Bridge::from_json` fixtures and do **not** require
dotnet, so `cargo test` on roslyn-sys is safe anywhere.