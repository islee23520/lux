using System;
using System.Collections.Generic;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace Linalab.Lux.Editor
{
    public static class LuxBatchAutomation
    {
        private const string ResultDir = "TestResults";
        private const string CompileResultFile = "CompileResult.json";

        public static void Compile()
        {
            var projectRoot = Directory.GetCurrentDirectory();
            var resultDir = Path.Combine(projectRoot, ResultDir);
            Directory.CreateDirectory(resultDir);
            var resultPath = Path.Combine(resultDir, CompileResultFile);

            var errorCount = 0;
            var errorMessages = new List<string>();

            Application.logMessageReceived += OnLogMessage;

            try
            {
                AssetDatabase.Refresh(ImportAssetOptions.ForceUpdate);

                var deadline = DateTime.UtcNow.AddSeconds(120);
                while (EditorApplication.isCompiling && DateTime.UtcNow < deadline)
                {
                    System.Threading.Thread.Sleep(100);
                }

                if (EditorUtility.scriptCompilationFailed)
                {
                    errorCount = -1;
                }
            }
            catch (Exception ex)
            {
                errorMessages.Add(ex.Message);
                errorCount = 1;
            }
            finally
            {
                Application.logMessageReceived -= OnLogMessage;
            }

            if (errorMessages.Count > 0 && errorCount <= 0)
            {
                errorCount = errorMessages.Count;
            }

            var result = new CompileResult
            {
                ok = errorCount <= 0,
                error_count = errorCount,
                message = errorCount <= 0 ? "Compile succeeded" : string.Join("\n", errorMessages),
                timestamp_utc = DateTime.UtcNow.ToString("O"),
                project_path = projectRoot
            };

            File.WriteAllText(resultPath, JsonUtility.ToJson(result, true));
            Debug.Log($"Lux compile result written to {resultPath}");

            if (errorCount > 0)
            {
                EditorApplication.Exit(1);
            }
        }

        private static void OnLogMessage(string condition, string stackTrace, LogType type)
        {
            if (type == LogType.Error || type == LogType.Exception)
            {
                if (condition.Contains("error CS"))
                {
                    return;
                }
            }
        }

        [Serializable]
        private class CompileResult
        {
            public bool ok;
            public int error_count;
            public string message;
            public string timestamp_utc;
            public string project_path;
        }
    }
}
