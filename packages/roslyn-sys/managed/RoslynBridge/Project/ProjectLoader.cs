using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.Text;

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
        var full = Path.GetFullPath(csprojPath);
        if (!full.EndsWith(".csproj", StringComparison.OrdinalIgnoreCase) ||
            !File.Exists(full))
        {
            return null;
        }

        var name = Path.GetFileNameWithoutExtension(full);
        var projectInfo = ProjectInfo.Create(
            ProjectId.CreateNewId(),
            VersionStamp.Default,
            name: name,
            assemblyName: name,
            language: LanguageNames.CSharp,
            filePath: full,
            metadataReferences: RuntimeReferences());

        var project = workspace.AddProject(projectInfo);

        foreach (var file in CsprojFileEnumerator.Enumerate(full))
        {
            workspace.AddDocument(DocumentInfo.Create(
                DocumentId.CreateNewId(project.Id),
                Path.GetFileName(file),
                loader: new FileTextLoader(file, System.Text.Encoding.UTF8),
                filePath: file));
        }

        // Re-fetch the project: each AddDocument produced a new solution
        // snapshot, so the Project returned by AddProject is now stale.
        return workspace.CurrentSolution.GetProject(project.Id);
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
