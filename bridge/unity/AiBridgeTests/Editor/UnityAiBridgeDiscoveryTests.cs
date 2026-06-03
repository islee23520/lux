using NUnit.Framework;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    public sealed class UnityAiBridgeDiscoveryTests
    {
        [Test]
        public void DiscoveryDto_RoundTripsThroughJsonUtility()
        {
            var discovery = new UnityAiBridgeDiscovery
            {
                host = "127.0.0.1",
                port = 51234,
                token = "token-123",
                protocolVersion = UnityAiBridgeProtocol.ProtocolVersion,
                projectPath = "/project/path",
                pid = 12345,
                startedAtUtc = "2026-04-28T00:00:00.0000000Z"
            };

            var json = JsonUtility.ToJson(discovery);
            var loaded = JsonUtility.FromJson<UnityAiBridgeDiscovery>(json);

            Assert.That(loaded.host, Is.EqualTo("127.0.0.1"));
            Assert.That(loaded.port, Is.EqualTo(51234));
            Assert.That(loaded.token, Is.EqualTo("token-123"));
            Assert.That(loaded.protocolVersion, Is.EqualTo(UnityAiBridgeProtocol.ProtocolVersion));
            Assert.That(loaded.projectPath, Is.EqualTo("/project/path"));
            Assert.That(loaded.pid, Is.EqualTo(12345));
            Assert.That(loaded.startedAtUtc, Is.EqualTo("2026-04-28T00:00:00.0000000Z"));
        }
    }
}
