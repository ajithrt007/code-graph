using System.Collections.Generic;
using System.Linq;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using CodeGraph.RoslynBridge.Models;

namespace CodeGraph.RoslynBridge.Analysis;

/// <summary>
/// Builds the in-code method graph from a compilation.
///
/// Pass 1 visits every method/constructor/operator declaration plus property
/// accessors and local functions and registers a node. Pass 2 visits every
/// method invocation, object creation, constructor initializer
/// (<c>: base()</c>/<c>: this()</c>), and delegate method-group reference
/// (<c>Action a = Foo;</c>, <c>evt += Handler;</c>), resolving the *actually
/// bound* symbol via the semantic model (never name matching) and emitting
/// an edge from the enclosing method. Only symbols declared in source become
/// nodes or edge endpoints, so BCL members and implicitly declared
/// constructors never appear in the graph.
///
/// Known non-goals (documented, not bugs): reflection
/// (<c>MethodInfo.Invoke</c>), <c>Activator.CreateInstance</c>,
/// <c>dynamic</c> dispatch, and calls in field initializers / top-level
/// statements with no enclosing method are skipped — the semantic model
/// returns no statically-bound in-source method symbol for them.
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

            // Pass 1: every method/constructor/accessor/local-function is a node.
            foreach (var member in root.DescendantNodes().OfType<BaseMethodDeclarationSyntax>())
            {
                var symbol = semanticModel.GetDeclaredSymbol(member) as IMethodSymbol;
                if (symbol is null || !MethodRenderer.InSource(symbol)) continue;
                // Syntax-node Span covers the complete declaration/body. The
                // symbol location covers only the name token for methods.
                RegisterNode(result, symbolToId, emittedIds,
                    MethodRenderer.ToMethodDto(symbol, filePath, symbolToId, Location.Create(tree, member.Span)));
            }

            foreach (var accessor in root.DescendantNodes().OfType<AccessorDeclarationSyntax>())
            {
                var symbol = semanticModel.GetDeclaredSymbol(accessor) as IMethodSymbol;
                if (symbol is null || !MethodRenderer.InSource(symbol)) continue;
                RegisterNode(result, symbolToId, emittedIds,
                    MethodRenderer.ToMethodDto(symbol, filePath, symbolToId, Location.Create(tree, accessor.Span)));
            }

            foreach (var local in root.DescendantNodes().OfType<LocalFunctionStatementSyntax>())
            {
                var symbol = semanticModel.GetDeclaredSymbol(local) as IMethodSymbol;
                if (symbol is null || !MethodRenderer.InSource(symbol)) continue;
                RegisterNode(result, symbolToId, emittedIds,
                    MethodRenderer.ToMethodDto(symbol, filePath, symbolToId, Location.Create(tree, local.Span)));
            }

            // Pass 2: method invocations become edges.
            foreach (var invocation in root.DescendantNodes().OfType<InvocationExpressionSyntax>())
            {
                var target = semanticModel.GetSymbolInfo(invocation).Symbol as IMethodSymbol;
                if (target is null || !MethodRenderer.InSource(target)) continue;
                if (!TryGetEnclosingMethod(semanticModel, invocation, out var source)) continue;
                AddEdge(result, symbolToId, emittedIds, source, target, filePath);
            }

            // Object creations become edges to their constructors.
            foreach (var creation in root.DescendantNodes().OfType<ObjectCreationExpressionSyntax>())
            {
                var ctor = semanticModel.GetSymbolInfo(creation).Symbol as IMethodSymbol;
                if (ctor is null || ctor.IsImplicitlyDeclared || !MethodRenderer.InSource(ctor)) continue;
                if (!TryGetEnclosingMethod(semanticModel, creation, out var source)) continue;
                AddEdge(result, symbolToId, emittedIds, source, ctor, filePath);
            }

            // Constructor initializers (`: base(x)` / `: this(x)`) become
            // edges from the declaring constructor to the base/this ctor.
            foreach (var initializer in root.DescendantNodes().OfType<ConstructorInitializerSyntax>())
            {
                var target = semanticModel.GetSymbolInfo(initializer).Symbol as IMethodSymbol;
                if (target is null || target.IsImplicitlyDeclared || !MethodRenderer.InSource(target)) continue;
                if (!TryGetEnclosingMethod(semanticModel, initializer, out var source)) continue;
                AddEdge(result, symbolToId, emittedIds, source, target, filePath);
            }

            // Delegate method-group references (`Action a = Foo;`,
            // `evt += Handler;`, `Call(Foo);` where the parameter is a
            // delegate type) become edges. The referenced name binds to the
            // exact overload via GetSymbolInfo; the delegate check (converted
            // type has an Invoke method) keeps `nameof(Foo)` and plain
            // identifiers out.
            foreach (var candidate in root.DescendantNodes().OfType<IdentifierNameSyntax>())
            {
                // The invoked name in `Foo()` is handled by the invocation
                // pass above; the `Foo` in `svc.Foo` is handled via the
                // outer MemberAccess node. Skip both to avoid double edges.
                if (candidate.Parent is InvocationExpressionSyntax invocation &&
                    invocation.Expression == candidate)
                    continue;
                if (candidate.Parent is MemberAccessExpressionSyntax memberAccess &&
                    memberAccess.Name == candidate)
                    continue;
                if (!IsDelegateMethodGroup(semanticModel, candidate, out var target)) continue;
                if (!TryGetEnclosingMethod(semanticModel, candidate, out var source)) continue;
                AddEdge(result, symbolToId, emittedIds, source, target, filePath);
            }

            foreach (var candidate in root.DescendantNodes().OfType<MemberAccessExpressionSyntax>())
            {
                if (candidate.Parent is InvocationExpressionSyntax invocation &&
                    invocation.Expression == candidate)
                    continue;
                if (!IsDelegateMethodGroup(semanticModel, candidate, out var target)) continue;
                if (!TryGetEnclosingMethod(semanticModel, candidate, out var source)) continue;
                AddEdge(result, symbolToId, emittedIds, source, target, filePath);
            }
        }
        return result;
    }

    private static void RegisterNode(
        AnalysisResult result,
        Dictionary<ISymbol, string> symbolToId,
        HashSet<string> emittedIds,
        MethodNodeDto dto)
    {
        if (emittedIds.Add(dto.Id))
        {
            result.Methods.Add(dto);
        }
        else
        {
            // A call from an earlier syntax tree may have registered
            // this node first with only its identifier location.
            // Replace it with the declaration-wide source span.
            var index = result.Methods.FindIndex(node => node.Id == dto.Id);
            if (index >= 0) result.Methods[index] = dto;
        }
    }

    /// <summary>
    /// Resolve the enclosing in-source method for a call site. Uses the
    /// semantic model's enclosing symbol first (handles methods, accessors,
    /// and local functions uniformly), falling back to a syntax-ancestor walk
    /// for robustness. Returns false for field initializers, top-level
    /// statements with no enclosing method, and non-method containers.
    /// </summary>
    private static bool TryGetEnclosingMethod(
        SemanticModel semanticModel,
        SyntaxNode node,
        out IMethodSymbol source)
    {
        source = null!;
        var enclosing = semanticModel.GetEnclosingSymbol(node.SpanStart) as IMethodSymbol;
        if (enclosing is not null && !enclosing.IsImplicitlyDeclared && MethodRenderer.InSource(enclosing))
        {
            source = enclosing;
            return true;
        }

        var ancestor = node.FirstAncestorOrSelf<SyntaxNode>(n =>
            n is LocalFunctionStatementSyntax ||
            n is AccessorDeclarationSyntax ||
            n is BaseMethodDeclarationSyntax);
        if (ancestor is not null)
        {
            var declared = semanticModel.GetDeclaredSymbol(ancestor) as IMethodSymbol;
            if (declared is not null && !declared.IsImplicitlyDeclared && MethodRenderer.InSource(declared))
            {
                source = declared;
                return true;
            }
        }

        // If the semantic model's answer was an implicitly-declared wrapper
        // (e.g. the synthesized top-level Main) but a real ancestor exists,
        // the fallback above already handled it. Anything left (field
        // initializers, global attributes) has no method container.
        return false;
    }

    /// <summary>
    /// True when <paramref name="node"/> names an in-source method that is
    /// being converted to a delegate type (method-group edge). Reflection,
    /// <c>dynamic</c>, and <c>nameof</c> never satisfy the delegate check.
    /// </summary>
    private static bool IsDelegateMethodGroup(
        SemanticModel semanticModel,
        ExpressionSyntax node,
        out IMethodSymbol target)
    {
        target = null!;
        var symbol = semanticModel.GetSymbolInfo(node).Symbol as IMethodSymbol;
        if (symbol is null || symbol.MethodKind != MethodKind.Ordinary || !MethodRenderer.InSource(symbol))
            return false;
        var converted = semanticModel.GetTypeInfo(node).ConvertedType as INamedTypeSymbol;
        if (converted?.DelegateInvokeMethod is null)
            return false;
        target = symbol;
        return true;
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
