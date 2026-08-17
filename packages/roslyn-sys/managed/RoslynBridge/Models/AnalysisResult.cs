using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace CodeGraph.RoslynBridge.Models;

/// <summary>
/// Result document emitted over stdout: either an `error` or the graph
/// (methods + edges) built from the analyzed project.
/// </summary>
internal sealed class AnalysisResult
{
    [JsonPropertyName("error")]
    public string? Error { get; set; }

    [JsonPropertyName("methods")]
    public List<MethodNodeDto> Methods { get; } = new();

    [JsonPropertyName("edges")]
    public List<CallEdgeDto> Edges { get; } = new();
}