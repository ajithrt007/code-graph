using System.Collections.Generic;
using System.Linq;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using CodeGraph.RoslynBridge.Models;

namespace CodeGraph.RoslynBridge.Analysis;

/// <summary>
/// Builds the in-code method graph from a compilation.
///
/// Pass 1 visits every method/constructor declaration and registers a node.
/// Pass 2 visits every method invocation and object creation, resolving the
/// *actually bound* symbol via the semantic model (never name matching) and
/// emitting an edge from the enclosing method. Only symbols declared in
/// source become nodes or edge endpoints, so BCL members and implicitly
/// declared constructors never appear in the graph.
/// </summary>
internal static class GraphBuilder
{
    public static AnalysisResult BuildGraph(Compilation compilation)
    {
        var result = new AnalysisResult();
        // Symbol-equality keeps overloads distinct within the compilation so
        // a call always maps to the exact IMethodSymbol it was bound to.
        var symbolToId = new Dictionary<ISymbol, string>(SymbolEqualityComparer.Default);
        // Guards against emitting the same node twice when a method is first
        // discovered as a call target (other tree) and later as a declaration.
        var emittedIds = new HashSet<string>();

        foreach (var tree in compilation.SyntaxTrees)
        {
            var root = tree.GetRoot();
            var semanticModel = compilation.GetSemanticModel(tree);
            var filePath = tree.FilePath;

            // Pass 1: every method/constructor declaration is a node.
            foreach (var member in root.DescendantNodes().OfType<BaseMethodDeclarationSyntax>())
            {
                var symbol = semanticModel.GetDeclaredSymbol(member) as IMethodSymbol;
                if (symbol is null || !MethodRenderer.InSource(symbol)) continue;
                var dto = MethodRenderer.ToMethodDto(symbol, filePath, symbolToId);
                if (!emittedIds.Add(dto.Id)) continue;
                result.Methods.Add(dto);
            }

            // Pass 2: method invocations become edges.
            foreach (var invocation in root.DescendantNodes().OfType<InvocationExpressionSyntax>())
            {
                var target = semanticModel.GetSymbolInfo(invocation).Symbol as IMethodSymbol;
                if (target is null || !MethodRenderer.InSource(target)) continue;
                var ancestor = invocation.FirstAncestorOrSelf<BaseMethodDeclarationSyntax>();
                if (ancestor is null) continue;
                var source = semanticModel.GetDeclaredSymbol(ancestor) as IMethodSymbol;
                if (source is null || !MethodRenderer.InSource(source)) continue;
                AddEdge(result, symbolToId, emittedIds, source, target, filePath);
            }

            // Object creations become edges to their constructors.
            foreach (var creation in root.DescendantNodes().OfType<ObjectCreationExpressionSyntax>())
            {
                var ctor = semanticModel.GetSymbolInfo(creation).Symbol as IMethodSymbol;
                if (ctor is null || ctor.IsImplicitlyDeclared || !MethodRenderer.InSource(ctor)) continue;
                var ancestor = creation.FirstAncestorOrSelf<BaseMethodDeclarationSyntax>();
                if (ancestor is null) continue;
                var source = semanticModel.GetDeclaredSymbol(ancestor) as IMethodSymbol;
                if (source is null || !MethodRenderer.InSource(source)) continue;
                AddEdge(result, symbolToId, emittedIds, source, ctor, filePath);
            }
        }
        return result;
    }

    private static void AddEdge(
        AnalysisResult result,
        Dictionary<ISymbol, string> symbolToId,
        HashSet<string> emittedIds,
        IMethodSymbol source,
        IMethodSymbol target,
        string filePath)
    {
        var sourceId = EnsureNode(result, symbolToId, emittedIds, source, filePath);
        var targetId = EnsureNode(result, symbolToId, emittedIds, target, filePath);
        result.Edges.Add(new CallEdgeDto
        {
            Source = sourceId,
            Target = targetId,
            Kind = "calls",
        });
    }

    /// <summary>Return the stable ID for `symbol`, registering its node the
    /// first time it is seen.</summary>
    private static string EnsureNode(
        AnalysisResult result,
        Dictionary<ISymbol, string> symbolToId,
        HashSet<string> emittedIds,
        IMethodSymbol symbol,
        string filePath)
    {
        if (symbolToId.TryGetValue(symbol, out var existing))
            return existing;
        var dto = MethodRenderer.ToMethodDto(symbol, filePath, symbolToId);
        if (emittedIds.Add(dto.Id))
            result.Methods.Add(dto);
        return dto.Id;
    }
}