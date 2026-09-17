using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace CodeGraph.RoslynBridge.Models;

internal sealed class AnalyzeRequest
{
    [JsonPropertyName("path")]
    public string Path { get; set; } = string.Empty;

    /// <summary>
    /// Sibling <c>.csproj</c> paths discovered in the same solution/workspace.
    /// The loader wires only the ones actually named by
    /// <c>&lt;ProjectReference&gt;</c> entries; the rest are ignored. Empty
    /// for single-project analysis (behavior unchanged).
    /// </summary>
    [JsonPropertyName("references")]
    public List<string> References { get; set; } = new();
}