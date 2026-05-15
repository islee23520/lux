using System;
using System.IO;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Linalab.Lux.Editor
{
    public static class LuxSceneSmoke
    {
        private const string ResultDir = "TestResults";
        private const string ResultFile = "LuxSceneSmokeResult.json";

        public static void Run()
        {
            var projectRoot = Directory.GetCurrentDirectory();
            var resultDir = Path.Combine(projectRoot, ResultDir);
            Directory.CreateDirectory(resultDir);
            var resultPath = Path.Combine(resultDir, ResultFile);

            var scenePath = Environment.GetEnvironmentVariable("LUX_SCENE_SMOKE_SCENE_PATH");
            var objectCountStr = Environment.GetEnvironmentVariable("LUX_SCENE_SMOKE_OBJECT_COUNT");
            int.TryParse(objectCountStr, out var expectedCount);

            var ok = true;
            var message = "Scene smoke test passed";
            var actualCount = 0;

            try
            {
                if (!string.IsNullOrEmpty(scenePath))
                {
                    EditorSceneManager.OpenScene(scenePath);
                }

                var scene = SceneManager.GetActiveScene();
                var rootObjects = scene.GetRootGameObjects();
                actualCount = rootObjects.Length;

                if (expectedCount > 0 && actualCount != expectedCount)
                {
                    ok = false;
                    message = $"Expected {expectedCount} root objects but found {actualCount} in scene '{scene.path}'";
                }
                else
                {
                    message = $"Scene '{scene.path}' loaded with {actualCount} root objects";
                }
            }
            catch (Exception ex)
            {
                ok = false;
                message = $"Scene smoke test failed: {ex.Message}";
            }

            var result = new SceneSmokeResult
            {
                ok = ok,
                message = message,
                scene_path = scenePath ?? SceneManager.GetActiveScene().path,
                object_count = actualCount,
                timestamp_utc = DateTime.UtcNow.ToString("O")
            };

            File.WriteAllText(resultPath, JsonUtility.ToJson(result, true));
            Debug.Log($"LuxSceneSmoke result written to {resultPath}");

            if (!ok)
            {
                EditorApplication.Exit(1);
            }
        }

        [Serializable]
        private class SceneSmokeResult
        {
            public bool ok;
            public string message;
            public string scene_path;
            public int object_count;
            public string timestamp_utc;
        }
    }
}
