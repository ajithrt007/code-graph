using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace CodeGraph.RoslynBridge.Project;

/// <summary>
/// Resolves the *.cs files that belong to a project by reading its .csproj.
///
/// Explicit <c>&lt;Compile Include="..."&gt;</c> items (wildcards allowed) are
/// honored and <c>&lt;Compile Remove="..."&gt;</c> items are always excluded.
/// When a project declares no Compile Include items at all (typical SDK-style
/// project), the SDK default glob — <c>**/*.cs</c> under the project
/// directory, skipping bin/obj — is used. Files, whether from csproj items or
/// the fallback glob, must live under the project directory; links
/// (<c>&lt;Link&gt;</c>) to files elsewhere remain a known limitation.
/// </summary>
internal static class CsprojFileEnumerator
{
    public static IEnumerable<string> Enumerate(string csproj)
    {
        var projectDir = Path.GetDirectoryName(csproj) ?? ".";
        var candidates = Directory.EnumerateFiles(projectDir, "*.cs", SearchOption.AllDirectories);

        var doc = TryLoadXml(csproj);
        IEnumerable<string> files = candidates;
        if (doc is not null)
        {
            var compileItems = doc.Descendants("Compile").ToList();
            var includes = compileItems
                .Attributes("Include")
                .Select(a => a.Value)
                .ToList();
            var removes = compileItems
                .Attributes("Remove")
                .Select(r => new GlobMatcher(r.Value, projectDir))
                .ToArray();

            if (includes.Count > 0)
            {
                var includeMatchers = includes
                    .Select(i => new GlobMatcher(i, projectDir))
                    .ToArray();
                files = candidates.Where(f => includeMatchers.Any(m => m.IsMatch(f)));
            }
            if (removes.Length > 0)
            {
                files = files.Where(f => !removes.Any(m => m.IsMatch(f)));
            }
        }

        return files.Where(f => !IsBuildOutput(f, projectDir));
    }

    private static XDocument? TryLoadXml(string csproj)
    {
        try
        {
            return XDocument.Load(csproj);
        }
        catch
        {
            // Malformed/unreadable project file — fall back to the SDK glob.
            return null;
        }
    }

    private static bool IsBuildOutput(string file, string projectDir)
    {
        var rel = Path.GetRelativePath(projectDir, file);
        var head = rel.Split(Path.DirectorySeparatorChar, 2)[0];
        return head.Equals("bin", StringComparison.OrdinalIgnoreCase) ||
               head.Equals("obj", StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>Matches a csproj Include/Remove path (which may contain **, *
    /// and ? wildcards) against files relative to the project dir. `**/`
    /// matches zero or more directory levels, matching MSBuild semantics (so
    /// <c>**/*.cs</c> also matches root-level files).</summary>
    private sealed class GlobMatcher
    {
        private readonly string _root;
        private readonly Regex _regex;

        public GlobMatcher(string pattern, string projectDir)
        {
            _root = projectDir.TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
            var normalized = pattern.Replace('\\', '/');
            while (normalized.StartsWith("./"))
                normalized = normalized.Substring(2);
            normalized = normalized.TrimStart('/').Replace("/./", "/");
            _regex = new Regex(GlobToRegex(normalized), RegexOptions.IgnoreCase | RegexOptions.Compiled);
        }

        public bool IsMatch(string file)
        {
            if (!file.StartsWith(_root, StringComparison.OrdinalIgnoreCase))
                return false;
            var rel = file.Substring(_root.Length).Replace('\\', '/');
            return _regex.IsMatch(rel);
        }

        private static string GlobToRegex(string glob)
        {
            var sb = new StringBuilder("^");
            for (int i = 0; i < glob.Length; i++)
            {
                switch (glob[i])
                {
                    case '*':
                        if (i + 1 < glob.Length && glob[i + 1] == '*')
                        {
                            i++;
                            if (i + 1 < glob.Length && glob[i + 1] == '/')
                            {
                                // `**/` = zero or more directory levels
                                i++;
                                sb.Append("(?:[^/][^/]*/)*");
                            }
                            else
                            {
                                sb.Append(".*");
                            }
                        }
                        else
                        {
                            sb.Append("[^/]*");
                        }
                        break;
                    case '?':
                        sb.Append("[^/]");
                        break;
                    default:
                        sb.Append(Regex.Escape(glob[i].ToString()));
                        break;
                }
            }
            sb.Append('$');
            return sb.ToString();
        }
    }
}