using System.Text.Json.Serialization;

namespace CodeGraph.RoslynBridge.Models;

internal sealed class MethodNodeDto
{
    [JsonPropertyName("id")] public string Id { get; set; } = string.Empty;
    [JsonPropertyName("name")] public string Name { get; set; } = string.Empty;
    [JsonPropertyName("fully_qualified_name")] public string FullyQualifiedName { get; set; } = string.Empty;
    [JsonPropertyName("display_name")] public string DisplayName { get; set; } = string.Empty;
    [JsonPropertyName("containing_type")] public string ContainingType { get; set; } = string.Empty;
    [JsonPropertyName("file_path")] public string FilePath { get; set; } = string.Empty;
    [JsonPropertyName("location")] public SourceLocationDto Location { get; set; } = new();
}