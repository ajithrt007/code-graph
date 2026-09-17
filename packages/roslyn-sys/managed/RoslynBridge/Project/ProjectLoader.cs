using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.Text;
using System.Xml.Linq;

namespace CodeGraph.RoslynBridge.Project;

/// <summary>
/// Builds an AdhocWorkspace project from a .csproj path. The set of C# files
/// added is taken from the project file itself (explicit Compile items, or
/// the SDK's default **/*.cs glob) rather than blindly globbing the directory.
/// </summary>
internal static class ProjectLoader
{
    /// <summary>
    /// Load the project at <paramref name="csprojPath"/> into
    /// <paramref name="workspace"/>. Returns null when the input is not an
    /// existing .csproj file.
    /// </summary>
    public static Microsoft.CodeAnalysis.Project? LoadProject(AdhocWorkspace workspace, string csprojPath)
    {
        return LoadProjectWithReferences(workspace, csprojPath, null);
    }

    /// <summary>
    /// Load <paramref name="csprojPath"/> plus sibling projects so
    /// cross-project calls resolve. <paramref name="siblingCsprojPaths"/> is
    /// the full sibling set (e.g. every project in the solution); only the
    /// ones actually named by <c>&lt;ProjectReference&gt;</c> entries are
    /// wired as Roslyn <see cref="ProjectReference"/>s. Each project is still
    /// compiled from its own sources with runtime (BCL) references; sibling
    /// assemblies come from project references, NuGet packages remain
    /// unresolved (documented limitation).
    /// </summary>
    public static Microsoft.CodeAnalysis.Project? LoadProjectWithReferences(
        AdhocWorkspace workspace,
        string csprojPath,
        IEnumerable<string>? siblingCsprojPaths)
    {
        var full = Path.GetFullPath(csprojPath);
        if (!full.EndsWith(".csproj", StringComparison.OrdinalIgnoreCase) ||
            !File.Exists(full))
        {
            return null;
        }

        // Load the root plus any existing sibling csproj files into the same
        // workspace so compilations can reference each other.
        var toLoad = new List<string> { full };
        if (siblingCsprojPaths is not null)
        {
            foreach (var sibling in siblingCsprojPaths)
            {
                try
                {
                    var siblingFull = Path.GetFullPath(sibling);
                    if (File.Exists(siblingFull) &&
                        siblingFull.EndsWith(".csproj", StringComparison.OrdinalIgnoreCase) &&
                        !toLoad.Contains(siblingFull, StringComparer.OrdinalIgnoreCase))
                    {
                        toLoad.Add(siblingFull);
                    }
                }
                catch
                {
                    // Ignore unresolvable sibling paths; the root analysis
                    // must still succeed.
                }
            }
        }

        var csprojToProjectId = new Dictionary<string, ProjectId>(StringComparer.OrdinalIgnoreCase);
        foreach (var csproj in toLoad)
        {
            var name = Path.GetFileNameWithoutExtension(csproj);
            var projectInfo = ProjectInfo.Create(
                ProjectId.CreateNewId(),
                VersionStamp.Default,
                name: name,
                assemblyName: name,
                language: LanguageNames.CSharp,
                filePath: csproj,
                metadataReferences: RuntimeReferences());
            workspace.AddProject(projectInfo);
            csprojToProjectId[csproj] = projectInfo.Id;

            foreach (var file in CsprojFileEnumerator.Enumerate(csproj))
            {
                workspace.AddDocument(DocumentInfo.Create(
                    DocumentId.CreateNewId(projectInfo.Id),
                    Path.GetFileName(file),
                    loader: new FileTextLoader(file, System.Text.Encoding.UTF8),
                    filePath: file));
            }
        }

        // Wire ProjectReferences based on each csproj's own
        // <ProjectReference Include="..."> entries (resolved relative to the
        // referencing csproj). Only references whose target was loaded above
        // are wired; the rest (missing files, NuGet) stay unresolved.
        foreach (var csproj in toLoad)
        {
            if (!csprojToProjectId.TryGetValue(csproj, out var fromId)) continue;
            foreach (var referenced in EnumerateProjectReferencePaths(csproj))
            {
                if (csprojToProjectId.TryGetValue(referenced, out var toId))
                {
                    var solution = workspace.CurrentSolution;
                    if (!solution.GetProject(fromId)!.ProjectReferences.Any(r => r.ProjectId == toId))
                    {
                        workspace.TryApplyChanges(solution.AddProjectReference(fromId, new ProjectReference(toId)));
                    }
                }
            }
        }

        // Re-fetch: each AddDocument/TryApplyChanges produced new solution
        // snapshots, so earlier Project handles are stale.
        return workspace.CurrentSolution.GetProject(csprojToProjectId[full]);
    }

    /// <summary>Absolute paths named by <c>&lt;ProjectReference&gt;</c> in a
    /// csproj. Plain XML read (no MSBuild evaluation); <c>Include</c> paths
    /// are resolved relative to the csproj directory.</summary>
    private static IEnumerable<string> EnumerateProjectReferencePaths(string csproj)
    {
        var dir = Path.GetDirectoryName(Path.GetFullPath(csproj)) ?? ".";
        XDocument? doc;
        try { doc = XDocument.Load(csproj); }
        catch { yield break; }
        if (doc is null) yield break;
        foreach (var el in doc.Descendants("ProjectReference"))
        {
            var include = el.Attribute("Include")?.Value;
            if (string.IsNullOrWhiteSpace(include)) continue;
            var normalized = include.Replace('\\', Path.DirectorySeparatorChar).Replace('/', Path.DirectorySeparatorChar);
            string combined;
            try { combined = Path.GetFullPath(Path.Combine(dir, normalized)); }
            catch { continue; }
            yield return combined;
        }
    }

    /// <summary>
    /// Reference the base class library so the semantic model can bind calls
    /// to BCL members. Only methods declared in source are kept as graph
    /// nodes, so these references never bloat the graph.
    /// </summary>
    private static List<PortableExecutableReference> RuntimeReferences()
    {
        var list = new List<PortableExecutableReference>();
        var runtimeDir = RuntimeEnvironment.GetRuntimeDirectory();
        if (!Directory.Exists(runtimeDir)) return list;
        foreach (var dll in Directory.EnumerateFiles(runtimeDir, "*.dll"))
        {
            try
            {
                list.Add(MetadataReference.CreateFromFile(dll));
            }
            catch
            {
                // Skip assemblies that aren't valid PEs.
            }
        }
        return list;
    }
}
