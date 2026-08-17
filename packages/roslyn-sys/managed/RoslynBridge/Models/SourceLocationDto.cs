using System.Text.Json.Serialization;

namespace CodeGraph.RoslynBridge.Models;

internal sealed class SourceLocationDto
{
    [JsonPropertyName("file_path")] public string FilePath { get; set; } = string.Empty;
    [JsonPropertyName("start_line")] public int StartLine { get; set; }
    [JsonPropertyName("start_column")] public int StartColumn { get; set; }
    [JsonPropertyName("end_line")] public int EndLine { get; set; }
    [JsonPropertyName("end_column")] public int EndColumn { get; set; }
}