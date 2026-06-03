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
            System.Environment.SetEnvironmentVariable("LUX_CLI_PATH", null);
            UnityAiBridgeTcpServer.StopShared();
            DiscoveryFileCleanup.DeleteDiscoveryFile();
        }

        [TearDown]
        public void TearDown()
        {
            UnityAiBridgeTcpServer.StopShared();
            DiscoveryFileCleanup.DeleteDiscoveryFile();
            EditorPrefs.DeleteKey(UnityAiBridgeMenu.AutoStartPreferenceKey);
            System.Environment.SetEnvironmentVariable("LUX_CLI_PATH", null);
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
        public void BuildMcpServerCommand_TargetsLuxMcpStdioServer()
        {
            var projectPath = Directory.GetCurrentDirectory();
            var command = UnityAiBridgeMenu.BuildMcpServerCommand();

            Assert.That(command, Does.Contain("'lux' mcp"));
            Assert.That(command, Does.Contain($"--project-path '{projectPath}'"));
            Assert.That(command, Does.Not.Contain("McpHelper~"));
            Assert.That(command, Does.Not.Contain("node "));
        }

        [Test]
        public void BuildMcpServerCommand_QuotesSpecialCharactersForPosixShell()
        {
            var projectPath = "/tmp/Unity AI Bridge/$HOME/`whoami`/$(touch bad)/Bob's \"Project\"";
            var expectedProjectPath = "'/tmp/Unity AI Bridge/$HOME/`whoami`/$(touch bad)/Bob'\\''s \"Project\"'";
            var command = InvokeBuildMcpServerCommand(projectPath);

            Assert.That(command, Is.EqualTo($"'lux' mcp --project-path {expectedProjectPath}"));
            Assert.That(command, Does.Not.Contain("\"/tmp/Unity AI Bridge"));
        }

        [Test]
        public void BuildMcpServerCommand_UsesExplicitLuxCliPathWhenProvided()
        {
            System.Environment.SetEnvironmentVariable("LUX_CLI_PATH", "/Applications/Lux CLI/lux");

            var command = InvokeBuildMcpServerCommand("/tmp/MyProject");

            Assert.That(command, Is.EqualTo("'/Applications/Lux CLI/lux' mcp --project-path '/tmp/MyProject'"));
        }

        [Test]
        public void QuotePosixShellArgument_EscapesSingleQuotesWithoutEnablingExpansion()
        {
            var quoted = InvokeQuotePosixShellArgument("a b$c`d`$(e)'f\"g");

            Assert.That(quoted, Is.EqualTo("'a b$c`d`$(e)'\\''f\"g'"));
        }

        private static string InvokeBuildMcpServerCommand(string projectPath)
        {
            var buildCommand = typeof(UnityAiBridgeMenu).GetMethod("BuildMcpServerCommand", BindingFlags.Static | BindingFlags.NonPublic, null, new[] { typeof(string) }, null);

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
