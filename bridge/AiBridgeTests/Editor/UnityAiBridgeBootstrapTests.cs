using System.IO;
using System.Reflection;
using NUnit.Framework;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    public sealed class UnityAiBridgeBootstrapTests
    {
        [SetUp]
        public void SetUp()
        {
            UnityAiBridgeMenu.SetAutoStartEnabled(true);
            UnityAiBridgeTcpServer.StopShared();
            DiscoveryFileCleanup.DeleteDiscoveryFile();
        }

        [TearDown]
        public void TearDown()
        {
            UnityAiBridgeTcpServer.StopShared();
            DiscoveryFileCleanup.DeleteDiscoveryFile();
            UnityAiBridgeMenu.SetAutoStartEnabled(true);
        }

        [Test]
        public void BootstrapStartIfEnabled_RespectsBatchModeStartupGuard()
        {
            var startIfEnabled = typeof(UnityAiBridgeBootstrap).GetMethod("StartIfEnabled", BindingFlags.NonPublic | BindingFlags.Static);

            Assert.That(startIfEnabled, Is.Not.Null);
            Assert.DoesNotThrow(() => startIfEnabled.Invoke(null, null));
            Assert.That(File.Exists(UnityAiBridgeMenu.GetServerDiscoveryPath()), Is.EqualTo(!Application.isBatchMode));
        }
    }
}
