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
            Console.Error.WriteLine("usage: RoslynBridge <path-to-csproj> [--reference <csproj>...]");
            return 2;
        }

        var references = new System.Collections.Generic.List<string>();
        for (int i = 1; i < args.Length; i++)
        {
            if ((args[i] == "--reference" || args[i] == "-r") && i + 1 < args.Length)
            {
                references.Add(args[++i]);
            }
        }

        var request = JsonSerializer.Serialize(new AnalyzeRequest { Path = args[0], References = references });
        var json = Bridge.Analyze(request);
        Console.Out.WriteLine(json);
        return json.Contains("\"error\"") ? 1 : 0;
    }
}