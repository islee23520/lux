using System.IO;
using NUnit.Framework;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    public sealed class UnityAiBridgeInfrastructureTests
    {
        [Test]
        public void DeleteDiscoveryFile_RemovesFileAndEmptyDirectory()
        {
            var discoveryPath = DiscoveryFileCleanup.DiscoveryFilePath;
            var discoveryDirectory = Path.GetDirectoryName(discoveryPath);

            Assert.That(discoveryDirectory, Is.Not.Null);

            Directory.CreateDirectory(discoveryDirectory);
            File.WriteAllText(discoveryPath, "{\"port\":12345}");

            Assert.That(File.Exists(discoveryPath), Is.True);

            DiscoveryFileCleanup.DeleteDiscoveryFile();

            Assert.That(File.Exists(discoveryPath), Is.False);
            Assert.That(Directory.Exists(discoveryDirectory), Is.False);
        }

        [Test]
        public void TemporaryAssetScope_RemovesCreatedAssetWhenDisposed()
        {
            string assetPath;

            using (var scope = TemporaryAssetScope.Create())
            {
                assetPath = scope.CreateTextAsset("bridge-test", "temporary");
                var asset = AssetDatabase.LoadAssetAtPath<TextAsset>(assetPath);
                Assert.That(asset, Is.Not.Null);
            }

            AssetDatabase.Refresh();

            Assert.That(AssetDatabase.LoadAssetAtPath<Object>(assetPath), Is.Null);
        }

        [Test]
        public void ExportDefaultContext_KeepsLegacyExportShape()
        {
            var outputPath = Path.Combine(Directory.GetCurrentDirectory(), UnityAiBridge.DefaultContextDirectory, UnityAiBridge.DefaultContextFileName);
            var outputDirectory = Path.GetDirectoryName(outputPath);
            var hadExistingFile = File.Exists(outputPath);
            var previousJson = hadExistingFile ? File.ReadAllText(outputPath) : null;

            try
            {
                var result = UnityAiBridge.ExportDefaultContext();
                var loaded = JsonUtility.FromJson<UnityAiContext>(result.Json);

                Assert.That(result.OutputPath, Is.EqualTo(outputPath));
                Assert.That(File.Exists(outputPath), Is.True);
                Assert.That(result.Context.aiTool, Is.EqualTo(AiToolKind.OpenCode.ToString()));
                Assert.That(result.Context.projectName, Is.EqualTo(Application.productName));
                Assert.That(result.Context.projectPath, Is.EqualTo(Directory.GetCurrentDirectory()));
                Assert.That(result.Context.unityVersion, Is.EqualTo(Application.unityVersion));
                Assert.That(result.Context.selection, Is.Not.Null);
                Assert.That(result.Context.packages, Is.Not.Null);
                Assert.That(result.Context.assemblies, Is.Not.Null);
                Assert.That(loaded.packages, Is.Not.Null);
                Assert.That(loaded.assemblies, Is.Not.Null);
                Assert.That(result.Json, Does.Not.Contain("selectedFiles"));
                Assert.That(result.Json, Does.Not.Contain("selectedFileContext"));
            }
            finally
            {
                if (hadExistingFile)
                {
                    File.WriteAllText(outputPath, previousJson);
                }
                else if (File.Exists(outputPath))
                {
                    File.Delete(outputPath);
                }

                if (!hadExistingFile && !string.IsNullOrEmpty(outputDirectory) && Directory.Exists(outputDirectory) && Directory.GetFileSystemEntries(outputDirectory).Length == 0)
                {
                    Directory.Delete(outputDirectory);
                }
            }
        }

        [Test]
        public void LuxPlayLogger_UsesGatewayUrlFromBridgeSettingsBeforeEditorPrefs()
        {
            var projectRoot = Directory.GetCurrentDirectory();
            var settingsPath = Path.Combine(projectRoot, "UserSettings", "LuxBridgeSettings.json");
            var settingsDirectory = Path.GetDirectoryName(settingsPath);
            var hadExistingFile = File.Exists(settingsPath);
            var previousJson = hadExistingFile ? File.ReadAllText(settingsPath) : null;
            var stalePrefsUrl = "http://127.0.0.1:1";
            var settingsUrl = "http://127.0.0.1:45678/";

            Assert.That(settingsDirectory, Is.Not.Null);

            try
            {
                Directory.CreateDirectory(settingsDirectory);
                File.WriteAllText(settingsPath, "{\n  \"gateway_url\": \"" + settingsUrl + "\"\n}\n");
                EditorPrefs.SetString("Linalab.Lux.PlayLogger.ServerUrl", stalePrefsUrl);

                var buildEndpoint = typeof(LuxPlayLogger).GetMethod("BuildEndpointUrl", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);

                Assert.That(buildEndpoint, Is.Not.Null);
                Assert.That(
                    buildEndpoint.Invoke(null, new object[] { "/api/lux/play/event" }),
                    Is.EqualTo("http://127.0.0.1:45678/api/lux/play/event"));
            }
            finally
            {
                EditorPrefs.DeleteKey("Linalab.Lux.PlayLogger.ServerUrl");

                if (hadExistingFile)
                {
                    File.WriteAllText(settingsPath, previousJson);
                }
                else if (File.Exists(settingsPath))
                {
                    File.Delete(settingsPath);
                }

                if (!hadExistingFile && !string.IsNullOrEmpty(settingsDirectory) && Directory.Exists(settingsDirectory) && Directory.GetFileSystemEntries(settingsDirectory).Length == 0)
                {
                    Directory.Delete(settingsDirectory);
                }
            }
        }
    }
}
