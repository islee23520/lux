using System;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace Linalab.Lux.Editor
{
    public static class LuxUnityContext
    {
        public static void Refresh()
        {
            var projectRoot = Directory.GetCurrentDirectory();
            var outputPath = Path.Combine(projectRoot, "UserSettings", "LuxUnityContext.json");

            try
            {
                var result = Linalab.UnityAiBridge.Editor.UnityAiBridge.ExportContext(
                    Linalab.UnityAiBridge.Editor.AiToolKind.OpenCode,
                    outputPath);
                Debug.Log($"LuxUnityContext written to {result.OutputPath}");
            }
            catch (Exception ex)
            {
                Debug.LogError($"LuxUnityContext.Refresh failed: {ex.Message}");
                var errorResult = new ContextError
                {
                    ok = false,
                    error = ex.Message,
                    timestamp_utc = DateTime.UtcNow.ToString("O")
                };

                var settingsDir = Path.GetDirectoryName(outputPath);
                if (!string.IsNullOrEmpty(settingsDir))
                {
                    Directory.CreateDirectory(settingsDir);
                }

                File.WriteAllText(outputPath, JsonUtility.ToJson(errorResult, true));
                EditorApplication.Exit(1);
            }
        }

        [Serializable]
        private class ContextError
        {
            public bool ok;
            public string error;
            public string timestamp_utc;
        }
    }
}
