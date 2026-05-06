using System;
using System.Collections.Generic;
using System.IO;
using NUnit.Framework;
using UnityEngine;

namespace Linalab.LuxEditor.Tests
{
    public sealed class LuxRuntimeEventTests
    {
        string _tempDirectory;
        UnityEditor.LuxAiActionLog _log;

        [SetUp]
        public void SetUp()
        {
            _tempDirectory = Path.Combine(Path.GetTempPath(), "LuxRuntimeEventTests", Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(_tempDirectory);
            _log = new UnityEditor.LuxAiActionLog(logPath: GetTempLogPath());
            UnityEditor.LuxAiActionLogBroadcaster.ConfigureForTests(_log, null, () => 0.0);
        }

        [TearDown]
        public void TearDown()
        {
            UnityEditor.LuxAiActionLogBroadcaster.ConfigureForTests(null, null, null);
            _log?.Dispose();
            _log = null;

            if (Directory.Exists(_tempDirectory))
            {
                Directory.Delete(_tempDirectory, true);
            }
        }

        [Test]
        public void Log_AppendsRuntimeGameplayJsonlEntry()
        {
            var payload = new Dictionary<string, object>
            {
                ["enemyId"] = "goblin_01",
                ["score"] = 25
            };

            UnityEditor.LuxRuntimeEvent.Log("test_event", payload);
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string[] lines = File.ReadAllLines(GetTempLogPath());

            Assert.That(lines, Has.Length.EqualTo(1));
            Assert.That(lines[0], Does.StartWith("{"));
            Assert.That(lines[0], Does.EndWith("}"));
            Assert.That(lines[0], Does.Contain("\"source\":\"gameplay\""));
            Assert.That(lines[0], Does.Contain("\"category\":\"runtime\""));
            Assert.That(lines[0], Does.Contain("\"action\":\"test_event\""));
            Assert.That(lines[0], Does.Contain("\"eventType\":\"test_event\""));
            Assert.That(lines[0], Does.Contain("\"enemyId\":\"goblin_01\""));
            Assert.That(lines[0], Does.Contain("\"score\":\"25\""));
        }

        [Test]
        public void RuntimeEventChannel_RaisesRuntimeGameplayJsonlEntry()
        {
            var channel = ScriptableObject.CreateInstance<UnityEditor.LuxRuntimeEventChannel>();
            try
            {
                channel.Raise("channel_event");
                UnityEditor.LuxAiActionLogBroadcaster.Flush();

                string[] lines = File.ReadAllLines(GetTempLogPath());

                Assert.That(lines, Has.Length.EqualTo(1));
                Assert.That(lines[0], Does.Contain("\"source\":\"gameplay\""));
                Assert.That(lines[0], Does.Contain("\"category\":\"runtime\""));
                Assert.That(lines[0], Does.Contain("\"action\":\"channel_event\""));
                Assert.That(lines[0], Does.Contain("\"eventType\":\"channel_event\""));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(channel);
            }
        }

        string GetTempLogPath()
        {
            return Path.Combine(_tempDirectory, ".lux", "ai-action-log.jsonl");
        }
    }
}
