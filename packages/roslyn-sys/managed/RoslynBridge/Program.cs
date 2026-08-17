using System;
using System.Text.Json;
using CodeGraph.RoslynBridge.Models;

namespace CodeGraph.RoslynBridge;

public static class Program
{
    public static int Main(string[] args)
    {
        if (args.Length < 1 || string.IsNullOrWhiteSpace(args[0]))
        {
            Console.Error.WriteLine("usage: RoslynBridge <path-to-csproj>");
            return 2;
        }

        var request = JsonSerializer.Serialize(new AnalyzeRequest { Path = args[0] });
        var json = Bridge.Analyze(request);
        Console.Out.WriteLine(json);
        return json.Contains("\"error\"") ? 1 : 0;
    }
}