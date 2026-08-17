using System;
using System.Text.Json;
using System.Text.Json.Serialization;
using CodeGraph.RoslynBridge.Analysis;
using CodeGraph.RoslynBridge.Models;
using CodeGraph.RoslynBridge.Project;
using Microsoft.CodeAnalysis;

namespace CodeGraph.RoslynBridge;

/// <summary>
/// FFI-shaped entry point: receives a JSON request (an <see cref="AnalyzeRequest"/>
/// naming the .csproj to analyze) and returns a JSON document shaped exactly
/// like the Rust <c>MethodGraph</c> domain type.
///
/// The Rust <c>roslyn-sys</c> crate spawns this assembly via
/// <c>dotnet RoslynBridge.dll &lt;path&gt;</c> and reads the result from
/// stdout; only JSON strings cross the process boundary, so Roslyn's object
/// graph never leaks into the application layer.
/// </summary>
public static class Bridge
{
    /// <summary>
    /// Analyze the project named by <paramref name="requestJson"/> and return
    /// a JSON document matching <see cref="AnalysisResult"/>. This is the
    /// subprocess protocol entry point: request string in, result string out.
    /// </summary>
    public static string Analyze(string requestJson)
    {
        var result = AnalyzeCore(requestJson);
        return JsonSerializer.Serialize(result, new JsonSerializerOptions
        {
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        });
    }

    private static AnalysisResult AnalyzeCore(string requestJson)
    {
        try
        {
            var request = JsonSerializer.Deserialize<AnalyzeRequest>(requestJson)
                ?? throw new InvalidOperationException("could not parse request");
            using var workspace = new AdhocWorkspace();
            var project = ProjectLoader.LoadProject(workspace, request.Path);
            if (project is null)
            {
                return new AnalysisResult { Error = $"could not load project at {request.Path}" };
            }

            var compilation = project.GetCompilationAsync().GetAwaiter().GetResult();
            if (compilation is null)
            {
                return new AnalysisResult { Error = "no compilation produced" };
            }

            return GraphBuilder.BuildGraph(compilation);
        }
        catch (Exception ex)
        {
            return new AnalysisResult { Error = ex.Message };
        }
    }
}