using System.Text.Json.Serialization;

namespace CodeGraph.RoslynBridge.Models;

internal sealed class AnalyzeRequest
{
    [JsonPropertyName("path")]
    public string Path { get; set; } = string.Empty;
}