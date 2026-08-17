using System.Collections.Generic;
using System.Linq;
using Microsoft.CodeAnalysis;
using CodeGraph.RoslynBridge.Models;

namespace CodeGraph.RoslynBridge.Analysis;

/// <summary>
/// Renders Roslyn symbols into the stable string IDs and DTOs used on the
/// wire. The ID is the fully-qualified signature, so overloads that differ by
/// type (not just arity) get distinct IDs and same-named methods in different
/// namespaces/types can never collide.
/// </summary>
internal static class MethodRenderer
{
    public static bool InSource(ISymbol symbol) =>
        symbol.Locations.Any(l => l.IsInSource);

    public static MethodNodeDto ToMethodDto(
        IMethodSymbol symbol,
        string fallbackFile,
        Dictionary<ISymbol, string> symbolToId)
    {
        var location = symbol.Locations.FirstOrDefault(l => l.IsInSource);
        var id = MakeId(symbol);
        symbolToId[symbol] = id;
        return new MethodNodeDto
        {
            Id = id,
            Name = symbol.Name,
            // CSharpErrorMessageFormat renders the fully-qualified signature
            // (namespace + containing type + name + qualified parameter
            // types). `FullyQualifiedFormat` skips the containing type for
            // methods in Roslyn 4.x, so we avoid it for FQNs.
            FullyQualifiedName = symbol.ToDisplayString(SymbolDisplayFormat.CSharpErrorMessageFormat),
            DisplayName = BuildDisplayName(symbol),
            ContainingType = symbol.ContainingType?.ToDisplayString() ?? string.Empty,
            FilePath = location?.SourceTree?.FilePath ?? fallbackFile,
            Location = ToLocation(location),
        };
    }

    public static string MakeId(IMethodSymbol symbol) =>
        $"csharp:{symbol.ToDisplayString(SymbolDisplayFormat.CSharpErrorMessageFormat)}";

    private static string BuildDisplayName(IMethodSymbol symbol)
    {
        var type = symbol.ContainingType?.Name ?? "?";
        var parens = "(" + string.Join(", ", symbol.Parameters.Select(p => p.Type.ToDisplayString(SymbolDisplayFormat.MinimallyQualifiedFormat))) + ")";
        return $"{type}.{symbol.Name}{parens}";
    }

    private static SourceLocationDto ToLocation(Location? loc)
    {
        if (loc is null || loc.SourceTree is null || !loc.IsInSource)
            return new SourceLocationDto();
        var span = loc.GetLineSpan();
        return new SourceLocationDto
        {
            FilePath = loc.SourceTree.FilePath,
            StartLine = span.StartLinePosition.Line + 1,
            StartColumn = span.StartLinePosition.Character + 1,
            EndLine = span.EndLinePosition.Line + 1,
            EndColumn = span.EndLinePosition.Character + 1,
        };
    }
}