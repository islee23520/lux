using System.IO;
using System.Reflection;
using NUnit.Framework;
using UnityEditor;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    public sealed class UnityAiBridgeMenuTests
    {
        [SetUp]
        public void SetUp()
        {
            EditorPrefs.DeleteKey(UnityAiBridgeMenu.AutoStartPreferenceKey);
            UnityAiBridgeTcpServer.StopShared();
            DiscoveryFileCleanup.DeleteDiscoveryFile();
        }

        [TearDown]
        public void TearDown()
        {
            UnityAiBridgeTcpServer.StopShared();
            DiscoveryFileCleanup.DeleteDiscoveryFile();
            EditorPrefs.DeleteKey(UnityAiBridgeMenu.AutoStartPreferenceKey);
        }

        [Test]
        public void ExportDefaultContext_RemainsFunctional()
        {
            var result = UnityAiBridge.ExportDefaultContext();

            try
            {
                Assert.That(File.Exists(result.OutputPath), Is.True);
                Assert.That(result.Json, Does.Contain("\"schemaVersion\""));
                Assert.That(result.Context, Is.Not.Null);
            }
            finally
            {
                if (File.Exists(result.OutputPath))
                {
                    File.Delete(result.OutputPath);
                }
            }
        }

        [Test]
        public void AutoStartPreference_DefaultsEnabledAndCanToggle()
        {
            Assert.That(UnityAiBridgeMenu.GetAutoStartEnabled(), Is.True);

            UnityAiBridgeMenu.SetAutoStartEnabled(false);
            Assert.That(UnityAiBridgeMenu.GetAutoStartEnabled(), Is.False);

            UnityAiBridgeMenu.SetAutoStartEnabled(true);
            Assert.That(UnityAiBridgeMenu.GetAutoStartEnabled(), Is.True);
        }

        [Test]
        public void StartStopRestart_AreIdempotent()
        {
            var firstServer = UnityAiBridgeMenu.StartContextServer();
            var secondServer = UnityAiBridgeMenu.StartContextServer();

            Assert.That(firstServer, Is.SameAs(secondServer));
            Assert.That(firstServer.IsRunning, Is.True);
            Assert.That(File.Exists(UnityAiBridgeMenu.GetServerDiscoveryPath()), Is.True);

            UnityAiBridgeMenu.StopContextServer();
            UnityAiBridgeMenu.StopContextServer();

            Assert.That(firstServer.IsRunning, Is.False);
            Assert.That(File.Exists(UnityAiBridgeMenu.GetServerDiscoveryPath()), Is.False);

            var restartedServer = UnityAiBridgeMenu.RestartContextServer();

            Assert.That(restartedServer.IsRunning, Is.True);
            Assert.That(File.Exists(UnityAiBridgeMenu.GetServerDiscoveryPath()), Is.True);

            UnityAiBridgeMenu.StopContextServer();
        }

        [Test]
        public void RevealServerDiscovery_MissingFileIsGraceful()
        {
            Assert.That(File.Exists(UnityAiBridgeMenu.GetServerDiscoveryPath()), Is.False);

            Assert.DoesNotThrow(() => UnityAiBridgeMenu.RevealServerDiscovery());
        }

        [Test]
        public void BuildMcpHelperCommand_TargetsBuiltPackageEntrypoint()
        {
            var projectPath = Directory.GetCurrentDirectory();
            var expectedEntrypoint = Path.Combine(projectPath, "Packages", "com.linalab.lux", "McpHelper~", "dist", "src", "index.js");
            var command = UnityAiBridgeMenu.BuildMcpHelperCommand();

            Assert.That(command, Does.Contain($"UNITY_PROJECT_PATH='{projectPath}'"));
            Assert.That(command, Does.Contain($"node '{expectedEntrypoint}'"));
            Assert.That(command, Does.Not.Contain("McpHelper~/dist/index.js"));
        }

        [Test]
        public void BuildMcpHelperCommand_QuotesSpecialCharactersForPosixShell()
        {
            var projectPath = "/tmp/Unity AI Bridge/$HOME/`whoami`/$(touch bad)/Bob's \"Project\"";
            var expectedProjectPath = "'/tmp/Unity AI Bridge/$HOME/`whoami`/$(touch bad)/Bob'\\''s \"Project\"'";
            var expectedEntrypoint = "'/tmp/Unity AI Bridge/$HOME/`whoami`/$(touch bad)/Bob'\\''s \"Project\"/Packages/com.linalab.lux/McpHelper~/dist/src/index.js'";
            var command = InvokeBuildMcpHelperCommand(projectPath);

            Assert.That(command, Is.EqualTo($"UNITY_PROJECT_PATH={expectedProjectPath} node {expectedEntrypoint}"));
            Assert.That(command, Does.Not.Contain("\"/tmp/Unity AI Bridge"));
        }

        [Test]
        public void QuotePosixShellArgument_EscapesSingleQuotesWithoutEnablingExpansion()
        {
            var quoted = InvokeQuotePosixShellArgument("a b$c`d`$(e)'f\"g");

            Assert.That(quoted, Is.EqualTo("'a b$c`d`$(e)'\\''f\"g'"));
        }

        private static string InvokeBuildMcpHelperCommand(string projectPath)
        {
            var buildCommand = typeof(UnityAiBridgeMenu).GetMethod("BuildMcpHelperCommand", BindingFlags.Static | BindingFlags.NonPublic, null, new[] { typeof(string) }, null);

            Assert.That(buildCommand, Is.Not.Null);
            return (string)buildCommand.Invoke(null, new object[] { projectPath });
        }

        private static string InvokeQuotePosixShellArgument(string value)
        {
            var quoteArgument = typeof(UnityAiBridgeMenu).GetMethod("QuotePosixShellArgument", BindingFlags.Static | BindingFlags.NonPublic);

            Assert.That(quoteArgument, Is.Not.Null);
            return (string)quoteArgument.Invoke(null, new object[] { value });
        }
    }
}
