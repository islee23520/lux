using System.Collections.Generic;
using System.IO;
using UnityEditor;
using UnityEditor.Compilation;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    internal static class UnityAiContextCollector
    {
        public static string[] GetSelectionPaths()
        {
            var paths = new List<string>();
            foreach (var item in Selection.objects)
            {
                var path = AssetDatabase.GetAssetPath(item);
                paths.Add(string.IsNullOrEmpty(path) ? item.name : path);
            }

            return paths.ToArray();
        }

        public static string[] GetPackageNames()
        {
            var manifestPath = Path.Combine(Directory.GetCurrentDirectory(), "Packages", "manifest.json");
            if (!File.Exists(manifestPath))
            {
                return new string[0];
            }

            var names = new List<string>();
            foreach (var line in File.ReadAllLines(manifestPath))
            {
                var trimmed = line.Trim();
                if (!trimmed.StartsWith("\""))
                {
                    continue;
                }

                var separatorIndex = trimmed.IndexOf("\":", System.StringComparison.Ordinal);
                if (separatorIndex > 1)
                {
                    names.Add(trimmed.Substring(1, separatorIndex - 1));
                }
            }

            return names.ToArray();
        }

        public static string[] GetAssemblyNames()
        {
            var assemblies = CompilationPipeline.GetAssemblies();
            var names = new string[assemblies.Length];
            for (var i = 0; i < assemblies.Length; i++)
            {
                names[i] = assemblies[i].name;
            }

            return names;
        }
    }
}
