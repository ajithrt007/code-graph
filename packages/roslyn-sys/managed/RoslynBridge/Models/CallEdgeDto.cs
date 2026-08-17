using System.Text.Json.Serialization;

namespace CodeGraph.RoslynBridge.Models;

internal sealed class CallEdgeDto
{
    [JsonPropertyName("source")] public string Source { get; set; } = string.Empty;
    [JsonPropertyName("target")] public string Target { get; set; } = string.Empty;
    [JsonPropertyName("kind")] public string Kind { get; set; } = "calls";
}