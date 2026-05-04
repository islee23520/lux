using System;
using System.IO;
using UnityEditor;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Linalab.UnityAiBridge.Editor
{
    public static class UnityAiBridge
    {
        public const string DefaultContextDirectory = ".lux/context";
        public const string DefaultContextFileName = "unity-context.json";

        public static UnityAiContext CaptureContext(AiToolKind toolKind)
        {
            var activeScene = SceneManager.GetActiveScene();
            return new UnityAiContext
            {
                schemaVersion = 1,
                capturedAtUtc = DateTime.UtcNow.ToString("O"),
                aiTool = toolKind.ToString(),
                projectName = Application.productName,
                projectPath = Directory.GetCurrentDirectory(),
                unityVersion = Application.unityVersion,
                platform = Application.platform.ToString(),
                activeScenePath = activeScene.path,
                activeSceneName = activeScene.name,
                selection = UnityAiContextCollector.GetSelectionPaths(),
                packages = UnityAiContextCollector.GetPackageNames(),
                assemblies = UnityAiContextCollector.GetAssemblyNames()
            };
        }

        public static UnityAiContextExportResult ExportDefaultContext(AiToolKind toolKind = AiToolKind.OpenCode)
        {
            var outputPath = Path.Combine(Directory.GetCurrentDirectory(), DefaultContextDirectory, DefaultContextFileName);
            return ExportContext(toolKind, outputPath);
        }

        public static UnityAiContextExportResult ExportContext(AiToolKind toolKind, string outputPath)
        {
            if (string.IsNullOrWhiteSpace(outputPath))
            {
                throw new ArgumentException("Output path is required.", nameof(outputPath));
            }

            var context = CaptureContext(toolKind);
            var directory = Path.GetDirectoryName(outputPath);
            if (!string.IsNullOrEmpty(directory))
            {
                Directory.CreateDirectory(directory);
            }

            var json = JsonUtility.ToJson(context, true);
            File.WriteAllText(outputPath, json);

            return new UnityAiContextExportResult(outputPath, json, context);
        }

        [MenuItem("Tools/Linalab/Lux/AI Bridge/Export Unity Context")]
        private static void ExportContextMenuItem()
        {
            var result = ExportDefaultContext();
            Debug.Log($"Unity AI context exported: {result.OutputPath}");
            EditorUtility.RevealInFinder(result.OutputPath);
        }
    }
}
