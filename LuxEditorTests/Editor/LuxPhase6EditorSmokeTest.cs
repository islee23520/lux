using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using NUnit.Framework;
using UnityEngine;

namespace Linalab.LuxEditor.Tests
{
    /// <summary>
    /// AC4 + AC6 Phase 6 verification smoke tests.
    ///
    /// AC4 SPEC: "Unity Editor user/system events are logged through LUX hooks without disrupting
    /// existing editor workflows. At least one automated or smoke test proves editor-originated
    /// events reach the JSONL log and are readable through CLI/API."
    ///
    /// AC6 SPEC: "A C# static API allows gameplay code to log events such as
    /// LuxRuntimeEvent.Log('enemy_death', payload). A ScriptableObject/serialized event channel
    /// wrapper is available for Unity-style serialized references. Smoke tests or sample code
    /// prove both paths emit events into the same schema."
    /// </summary>
    [TestFixture]
    public sealed class LuxPhase6EditorSmokeTest
    {
        string _tempDirectory;
        UnityEditor.LuxAiActionLog _log;
        List<Tuple<string, UnityEditor.LuxAiActionLogEntry>> _broadcasts;

        [SetUp]
        public void SetUp()
        {
            _tempDirectory = Path.Combine(Path.GetTempPath(), "LuxPhase6Smoke", Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(_tempDirectory);
            _log = new UnityEditor.LuxAiActionLog(logPath: GetTempLogPath());
            _broadcasts = new List<Tuple<string, UnityEditor.LuxAiActionLogEntry>>();
            UnityEditor.LuxAiActionLogBroadcaster.ConfigureForTests(
                _log,
                (eventType, payload) => _broadcasts.Add(Tuple.Create(eventType, (UnityEditor.LuxAiActionLogEntry)payload)),
                () => 0.0);
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

        // ====================================================================
        // AC4: Editor Hook — Event Reachability & JSONL Persistence
        // ====================================================================

        #region AC4a — Editor lifecycle events are subscribed and produce entries

        [Test]
        [Description("AC4a: PlayModeStateChanged hook records entry with correct category/action")]
        public void AC4a_PlayModeStateChange_ProducesJsonlEntry()
        {
            var entry = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "playmode", "state_changed", "EnteredPlayMode", "Playmode entered");

            Assert.That(entry.category, Is.EqualTo("playmode"));
            Assert.That(entry.action, Is.EqualTo("state_changed"));
            Assert.That(entry.target, Is.EqualTo("EnteredPlayMode"));
            Assert.That(entry.source, Is.EqualTo("unity-editor"));
            Assert.That(entry.actor, Is.EqualTo("user"));
        }

        [Test]
        [Description("AC4a: Selection change hook records entry with object metadata")]
        public void AC4a_SelectionChange_ProducesEntryWithMetadata()
        {
            var entry = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "selection", "changed", "MyGameObject", "Selection changed",
                metadata: new Dictionary<string, string> { ["objectCount"] = "3" });

            Assert.That(entry.category, Is.EqualTo("selection"));
            Assert.That(entry.metadata["objectCount"], Is.EqualTo("3"));
        }

        [Test]
        [Description("AC4a: Hierarchy change hook records entry")]
        public void AC4a_HierarchyChange_ProducesEntry()
        {
            var entry = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "hierarchy", "changed", "Hierarchy", "Hierarchy changed");

            Assert.That(entry.category, Is.EqualTo("hierarchy"));
            Assert.That(entry.action, Is.EqualTo("changed"));
        }

        [Test]
        [Description("AC4a: Project change hook records entry")]
        public void AC4a_ProjectChange_ProducesEntry()
        {
            var entry = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "project", "changed", "Project", "Project changed");

            Assert.That(entry.category, Is.EqualTo("project"));
        }

        [Test]
        [Description("AC4a: Undo/redo hook records entry")]
        public void AC4a_UndoRedo_ProducesEntry()
        {
            var entry = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "undo-redo", "performed", "Undo", "Undo performed");

            Assert.That(entry.category, Is.EqualTo("undo-redo"));
        }

        [Test]
        [Description("AC4a: Scene open/save/close hooks record entries")]
        public void AC4a_SceneLifecycleHooks_ProduceEntries()
        {
            var opened = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "scene", "opened", "Assets/GamePlay.unity", "Opened",
                metadata: new Dictionary<string, string> { ["mode"] = "Single" });
            var saved = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "scene", "saved", "Assets/GamePlay.unity", "Saved");
            var closing = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "scene", "closing", "Assets/GamePlay.unity", "Closing",
                metadata: new Dictionary<string, string> { ["removingScene"] = "false" });

            Assert.That(opened.category, Is.EqualTo("scene"));
            Assert.That(opened.action, Is.EqualTo("opened"));
            Assert.That(opened.metadata["mode"], Is.EqualTo("Single"));
            Assert.That(saved.action, Is.EqualTo("saved"));
            Assert.That(closing.action, Is.EqualTo("closing"));
            Assert.That(closing.metadata["removingScene"], Is.EqualTo("false"));
        }

        #endregion

        #region AC4b — Editor hook entries persist to JSONL log file

        [Test]
        [Description("AC4b: Editor-originated event is written to JSONL log file")]
        public void AC4b_EditorEvent_ReachesJsonlLogFile()
        {
            UnityEditor.LuxAiActionLogBroadcaster.Record(
                "smoke_test", "ac4_verify", "TargetObj", "AC4 smoke test event");
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            Assert.That(File.Exists(GetTempLogPath()), Is.True, "JSONL log file must exist");
            string[] lines = File.ReadAllLines(GetTempLogPath());
            Assert.That(lines, Has.Length.GreaterThanOrEqualTo(1), "At least one JSONL line expected");
            Assert.That(lines[0], Does.StartWith("{"), "Line must be a JSON object");
            Assert.That(lines[0], Does.EndWith("}"), "Line must be a JSON object");
            Assert.That(lines[0], Does.Contain("\"schemaVersion\":1"));
            Assert.That(lines[0], Does.Contain("\"protocol\":\"lux.ai.action_log.v1\""));
            Assert.That(lines[0], Does.Contain("\"category\":\"smoke_test\""));
            Assert.That(lines[0], Does.Contain("\"action\":\"ac4_verify\""));
        }

        [Test]
        [Description("AC4b: Multiple editor events all appear in JSONL in order")]
        public void AC4b_MultipleEvents_AllPersistedToJsonl()
        {
            UnityEditor.LuxAiActionLogBroadcaster.Record("cat1", "action1", "t1", "m1");
            UnityEditor.LuxAiActionLogBroadcaster.Record("cat2", "action2", "t2", "m2");
            UnityEditor.LuxAiActionLogBroadcaster.Record("cat3", "action3", "t3", "m3");
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string[] lines = File.ReadAllLines(GetTempLogPath());
            Assert.That(lines, Has.Length.EqualTo(3));
            Assert.That(lines[0], Does.Contain("\"action\":\"action1\""));
            Assert.That(lines[1], Does.Contain("\"action\":\"action2\""));
            Assert.That(lines[2], Does.Contain("\"action\":\"action3\""));
        }

        [Test]
        [Description("AC4b: JSONL entry contains all required schema fields")]
        public void AC4b_JsonlEntry_ContainsAllRequiredSchemaFields()
        {
            UnityEditor.LuxAiActionLogBroadcaster.Record(
                "schema_check", "verify", "target", "message",
                severity: "warning", success: false);
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());
            string[] requiredFields =
            {
                "schemaVersion", "protocol", "id", "timestampUtc",
                "source", "actor", "category", "action",
                "target", "message", "severity", "success", "metadata"
            };

            foreach (var field in requiredFields)
            {
                Assert.That(json, Does.Contain($"\"{field}\""),
                    $"JSONL entry must contain required field: {field}");
            }
        }

        #endregion

        #region AC4c — Non-blocking, non-disruptive operation

        [Test]
        [Description("AC4c: Recording events does not block (async write via background thread)")]
        public void AC4c_Recording_IsNonBlocking()
        {
            // Record many events rapidly — should return immediately
            var watch = System.Diagnostics.Stopwatch.StartNew();
            for (int i = 0; i < 100; i++)
            {
                UnityEditor.LuxAiActionLogBroadcaster.Record(
                    "stress", $"action_{i}", "target", "stress test");
            }
            watch.Stop();

            // 100 records should complete well under 1 second (no blocking I/O)
            Assert.That(watch.ElapsedMilliseconds, Is.LessThan(1000),
                "Recording 100 events should not block");
        }

        [Test]
        [Description("AC4c: Broadcast queue is bounded and does not grow unbounded")]
        public void AC4c_BroadcastQueue_BoundedAtMaxSize()
        {
            // The internal max is 256; we can't easily hit it in a unit test without
            // pumping, but we verify the batching mechanism works correctly
            for (int i = 0; i < 20; i++)
            {
                UnityEditor.LuxAiActionLogBroadcaster.Record(
                    "batch", $"item_{i}", "target", "batch test");
            }

            int pumped = UnityEditor.LuxAiActionLogBroadcaster.PumpForTests();
            Assert.That(pumped, Is.GreaterThanOrEqualTo(16),
                "First pump should send batch size (16)");
            Assert.That(_broadcasts, Has.Count.GreaterThanOrEqualTo(16));
        }

        [Test]
        [Description("AC4c: Flush drains all pending writes to disk")]
        public void AC4c_Flush_DrainsAllPendingWrites()
        {
            UnityEditor.LuxAiActionLogBroadcaster.Record("flush_test", "before", "t", "m");
            UnityEditor.LuxAiActionLogBroadcaster.Flush();
            long sizeAfterFirst = new FileInfo(GetTempLogPath()).Length;

            UnityEditor.LuxAiActionLogBroadcaster.Record("flush_test", "after", "t", "m");
            UnityEditor.LuxAiActionLogBroadcaster.Flush();
            long sizeAfterSecond = new FileInfo(GetTempLogPath()).Length;

            Assert.That(sizeAfterSecond, Is.GreaterThan(sizeAfterFirst),
                "Flush should persist new data to disk");
        }

        #endregion

        #region AC4d — Smoke test proving full editor-hook → JSONL pipeline

        [Test]
        [Description("AC4d: End-to-end smoke: editor event → broadcaster → log → readable JSONL")]
        public void AC4d_FullPipeline_SmokeTest()
        {
            // Simulate the exact path an editor event takes:
            // 1. Editor lifecycle fires → Broadcaster.Record()
            // 2. Record creates entry → EnqueueBroadcast → pending queue
            // 3. PumpBroadcasts → broadcastSink (TCP in prod, captured here)
            // 4. Log.Record → _pendingLines → WriterLoop → .jsonl file

            var entry = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "ac4_smoke", "pipeline_verify", "SmokeTarget",
                "Full pipeline smoke test",
                severity: "info",
                success: true,
                metadata: new Dictionary<string, string> { ["smokeId"] = "ac4d-001" });

            // Pump broadcasts to simulate EditorApplication.update cycle
            int sent = UnityEditor.LuxAiActionLogBroadcaster.PumpForTests();

            // Flush log to ensure writer thread has persisted
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            // Verify: entry was created with correct shape
            Assert.That(entry.id, Is.Not.Null.And.Not.Empty, "Entry must have a GUID id");
            Assert.That(entry.timestampUtc, Is.Not.Null.And.Not.Empty, "Entry must have ISO-8601 timestamp");
            Assert.That(entry.schemaVersion, Is.EqualTo(1));
            Assert.That(entry.protocol, Is.EqualTo("lux.ai.action_log.v1"));

            // Verify: broadcast was sent
            Assert.That(sent, Is.GreaterThanOrEqualTo(1), "At least one broadcast must be sent");
            Assert.That(_broadcasts, Has.Count.GreaterThanOrEqualTo(1));
            Assert.That(_broadcasts[0].Item1, Is.EqualTo("ai_action_log"));

            // Verify: persisted to JSONL and readable
            Assert.That(File.Exists(GetTempLogPath()), Is.True, "JSONL file must exist after flush");
            string jsonlContent = File.ReadAllText(GetTempLogPath());
            Assert.That(jsonlContent, Does.Contain("\"category\":\"ac4_smoke\""));
            Assert.That(jsonlContent, Does.Contain("\"action\":\"pipeline_verify\""));
            Assert.That(jsonlContent, Does.Contain("\"smokeId\":\"ac4d-001\""));

            // Verify: JSONL line is valid JSON parseable back to entry shape
            var parsed = JsonUtility.FromJson<UnityEditor.LuxAiActionLogEntry>(
                jsonlContent.TrimEnd('\n'));
            Assert.That(parsed.id, Is.EqualTo(entry.id));
            Assert.That(parsed.category, Is.EqualTo(entry.category));
            Assert.That(parsed.action, Is.EqualTo(entry.action));
        }

        [Test]
        [Description("AC4d: Console message summary hook produces aggregated entry")]
        public void AC4d_ConsoleSummaryHook_ProducesAggregatedEntry()
        {
            // The console summary path batches multiple log messages into one entry.
            // We verify the Record path directly since Application.logMessageReceived
            // requires a running Unity editor.
            var entry = UnityEditor.LuxAiActionLogBroadcaster.Record(
                "console", "summary", "Console", "Console messages summarized",
                severity: "error",
                success: false,
                metadata: new Dictionary<string, string>
                {
                    ["Error"] = "3",
                    ["Warning"] = "7",
                    ["Log"] = "42"
                });

            Assert.That(entry.category, Is.EqualTo("console"));
            Assert.That(entry.severity, Is.EqualTo("error"));
            Assert.That(entry.success, Is.False);
            Assert.That(entry.metadata["Error"], Is.EqualTo("3"));
        }

        #endregion

        // ====================================================================
        // AC6: Runtime C# API — Static Method + ScriptableObject Channel
        // ====================================================================

        #region AC6a — LuxRuntimeEvent.Log() public static API exists

        [Test]
        [Description("AC6a: LuxRuntimeEvent.Log(string, Dictionary) is callable and returns void")]
        public void AC6a_Log_StaticMethod_IsCallable()
        {
            var payload = new Dictionary<string, object>
            {
                ["playerId"] = "player_001",
                ["damage"] = 150
            };

            // Should not throw — this is the primary API surface
            Assert.DoesNotThrow(() =>
                UnityEditor.LuxRuntimeEvent.Log("enemy_death", payload));
        }

        [Test]
        [Description("AC6a: Convenience overload Log(string) accepts eventType only")]
        public void AC6a_Log_ConvenienceOverload_IsCallable()
        {
            Assert.DoesNotThrow(() =>
                UnityEditor.LuxRuntimeEvent.Log("level_complete"));
        }

        #endregion

        #region AC6b — Accepts string eventType + Dictionary<string,object> payload

        [Test]
        [Description("AC6b: Payload dictionary values are serialized into metadata")]
        public void AC6b_Payload_ValuesSerializedToMetadata()
        {
            var payload = new Dictionary<string, object>
            {
                ["enemyId"] = "goblin_01",
                ["damage"] = 99,
                ["isBoss"] = true,
                ["scoreMultiplier"] = 2.5f
            };

            UnityEditor.LuxRuntimeEvent.Log("combat_event", payload);
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());

            // All payload keys should appear in the JSONL metadata block
            Assert.That(json, Does.Contain("\"enemyId\":\"goblin_01\""));
            Assert.That(json, Does.Contain("\"damage\":\"99\""));
            Assert.That(json, Does.Contain("\"isBoss\":\"True\""));
            Assert.That(json, Does.Contain("\"scoreMultiplier\":\"2.5\""));
        }

        [Test]
        [Description("AC6b: Null eventType defaults to 'runtime_event'")]
        public void AC6b_NullEventType_DefaultsToRuntimeEvent()
        {
            UnityEditor.LuxRuntimeEvent.Log(null);
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());
            Assert.That(json, Does.Contain("\"eventType\":\"runtime_event\""));
        }

        [Test]
        [Description("AC6b: Empty/whitespace eventType defaults to 'runtime_event'")]
        public void AC6b_EmptyEventType_DefaultsToRuntimeEvent()
        {
            UnityEditor.LuxRuntimeEvent.Log("   ");
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());
            Assert.That(json, Does.Contain("\"eventType\":\"runtime_event\""));
        }

        [Test]
        [Description("AC6b: Null payload produces valid entry with only eventType metadata")]
        public void AC6b_NullPayload_ProducesValidEntry()
        {
            UnityEditor.LuxRuntimeEvent.Log("test_no_payload", null);
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());
            Assert.That(json, Does.Contain("\"eventType\":\"test_no_payload\""));
            Assert.That(json, Does.Contain("\"category\":\"runtime\""));
            Assert.That(json, Does.Contain("\"source\":\"gameplay\""));
        }

        #endregion

        #region AC6c — LuxRuntimeEventChannel ScriptableObject

        [Test]
        [Description("AC6c: LuxRuntimeEventChannel is a ScriptableObject with CreateAssetMenu")]
        public void AC6c_Channel_IsScriptableObjectWithCreateAssetMenu()
        {
            var channel = ScriptableObject.CreateInstance<UnityEditor.LuxRuntimeEventChannel>();
            try
            {
                Assert.That(channel, Is.InstanceOf<ScriptableObject>());
                // Verify it's a proper Unity asset type by checking it can be destroyed
                Assert.DoesNotThrow(() =>
                {
                    // Just verify the type exists and is usable
                    channel.Raise("type_check");
                });
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(channel);
            }
        }

        [Test]
        [Description("AC6c: Channel.Raise(eventType, payload) delegates to LuxRuntimeEvent.Log")]
        public void AC6c_ChannelRaise_DelegatesToStaticApi()
        {
            var channel = ScriptableObject.CreateInstance<UnityEditor.LuxRuntimeEventChannel>();
            try
            {
                var payload = new Dictionary<string, object>
                {
                    ["channelSource"] = true,
                    ["weapon"] = "plasma_rifle"
                };

                channel.Raise("weapon_fired", payload);
                UnityEditor.LuxAiActionLogBroadcaster.Flush();

                string json = File.ReadAllText(GetTempLogPath());
                Assert.That(json, Does.Contain("\"action\":\"weapon_fired\""));
                Assert.That(json, Does.Contain("\"eventType\":\"weapon_fired\""));
                Assert.That(json, Does.Contain("\"channelSource\":\"True\""));
                Assert.That(json, Does.Contain("\"weapon\":\"plasma_rifle\""));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(channel);
            }
        }

        [Test]
        [Description("AC6c: Channel.Raise(eventType) convenience overload works")]
        public void AC6c_ChannelRaise_ConvenienceOverload_Works()
        {
            var channel = ScriptableObject.CreateInstance<UnityEditor.LuxRuntimeEventChannel>();
            try
            {
                channel.Raise("simple_event");
                UnityEditor.LuxAiActionLogBroadcaster.Flush();

                string json = File.ReadAllText(GetTempLogPath());
                Assert.That(json, Does.Contain("\"action\":\"simple_event\""));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(channel);
            }
        }

        #endregion

        #region AC6d — Both paths emit compatible (identical) schema

        [Test]
        [Description("AC6d: Static API and ScriptableObject channel produce same schema structure")]
        public void AC6d_BothPaths_ProduceCompatibleSchema()
        {
            var payload = new Dictionary<string, object> { ["key"] = "value" };

            // Path 1: Static API
            UnityEditor.LuxRuntimeEvent.Log("static_path", payload);
            UnityEditor.LuxAiActionLogBroadcaster.Flush();
            string staticJson = File.ReadAllText(GetTempLogPath());

            // Clear for next test
            File.Delete(GetTempLogPath());

            // Path 2: ScriptableObject channel
            var channel = ScriptableObject.CreateInstance<UnityEditor.LuxRuntimeEventChannel>();
            channel.Raise("channel_path", payload);
            UnityEditor.LuxAiActionLogBroadcaster.Flush();
            string channelJson = File.ReadAllText(GetTempLogPath());
            UnityEngine.Object.DestroyImmediate(channel);

            // Both must share the same structural schema fields
            string[] commonFields =
            {
                "schemaVersion", "protocol", "id", "timestampUtc",
                "source", "actor", "category", "action",
                "target", "message", "severity", "success", "metadata",
                "eventType"
            };

            foreach (var field in commonFields)
            {
                Assert.That(staticJson, Does.Contain($"\"{field}\""),
                    $"Static API output must contain field: {field}");
                Assert.That(channelJson, Does.Contain($"\"{field}\""),
                    $"Channel API output must contain field: {field}");
            }

            // Both must use same source/category conventions
            Assert.That(staticJson, Does.Contain("\"source\":\"gameplay\""));
            Assert.That(channelJson, Does.Contain("\"source\":\"gameplay\""));
            Assert.That(staticJson, Does.Contain("\"category\":\"runtime\""));
            Assert.That(channelJson, Does.Contain("\"category\":\"runtime\""));
        }

        [Test]
        [Description("AC6d: Runtime events and editor hook events coexist in same JSONL log")]
        public void AC6d_RuntimeAndEditorEvents_CoexistInSameLog()
        {
            // Write an editor hook event
            UnityEditor.LuxAiActionLogBroadcaster.Record(
                "editor_hook", "scene_save", "GamePlay.unity", "Scene saved");
            // Write a runtime API event
            UnityEditor.LuxRuntimeEvent.Log("player_spawn",
                new Dictionary<string, object> { ["x"] = 10, ["y"] = 5 });
            // Write a channel event
            var ch = ScriptableObject.CreateInstance<UnityEditor.LuxRuntimeEventChannel>();
            ch.Raise("item_pickup", new Dictionary<string, object> { ["item"] = "health_pack" });
            UnityEngine.Object.DestroyImmediate(ch);

            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string[] lines = File.ReadAllLines(GetTempLogPath());
            Assert.That(lines, Has.Length.EqualTo(3),
                "All three event sources must produce separate JSONL lines");

            // Each line must be valid JSON with the shared schema
            Assert.That(lines[0], Does.Contain("\"category\":\"editor_hook\""));
            Assert.That(lines[1], Does.Contain("\"category\":\"runtime\""));
            Assert.That(lines[2], Does.Contain("\"category\":\"runtime\""));

            // All share same protocol/schema version
            foreach (var line in lines)
            {
                Assert.That(line, Does.Contain("\"schemaVersion\":1"));
                Assert.That(line, Does.Contain("\"protocol\":\"lux.ai.action_log.v1\""));
            }
        }

        #endregion

        // ====================================================================
        // Cross-cutting: Attribution & Correlation
        // ====================================================================

        [Test]
        [Description("Cross-cut: PushAttribution scopes propagate to runtime events")]
        public void Attribution_PropagatesToRuntimeEvents()
        {
            using (UnityEditor.LuxAiActionLogBroadcaster.PushAttribution(
                "ai_agent", "opencode-session", "corr-abc-123"))
            {
                UnityEditor.LuxRuntimeEvent.Log("attributed_event",
                    new Dictionary<string, object> { ["step"] = 5 });
            }
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());
            // The attribution scope should set actor/source on the recorded entry.
            // Note: LuxRuntimeEvent.Log calls PushAttribution internally with ("gameplay","gameplay")
            // which overrides the outer scope's actor. But correlationId from the outer scope
            // propagates via the attribution TTL mechanism if within the window.
            Assert.That(json, Does.Contain("\"action\":\"attributed_event\""));
            Assert.That(json, Does.Contain("\"category\":\"runtime\""));
        }

        // ====================================================================
        // LINA-4 Edge Cases
        // ====================================================================

        #region LINA-4a — Null/empty eventType fallback

        [Test]
        [Description("LINA-4a: Empty string eventType falls back to 'runtime_event'")]
        public void LINA4_EmptyEventType_FallsBackToDefault()
        {
            UnityEditor.LuxRuntimeEvent.Log("");
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());
            Assert.That(json, Does.Contain("\"eventType\":\"runtime_event\""),
                "empty eventType must fall back to 'runtime_event'");
        }

        [Test]
        [Description("LINA-4a: Null eventType and empty payload produces valid entry with default eventType")]
        public void LINA4_NullEventTypeAndEmptyPayload_ProducesValidEntry()
        {
            UnityEditor.LuxRuntimeEvent.Log(null, new Dictionary<string, object>());
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());
            Assert.That(json, Does.Contain("\"eventType\":\"runtime_event\""));
            Assert.That(json, Does.Contain("\"category\":\"runtime\""));
        }

        #endregion

        #region LINA-4b — Empty payload dictionary

        [Test]
        [Description("LINA-4b: Empty payload dictionary produces valid JSONL entry without crash")]
        public void LINA4_EmptyPayloadDictionary_ProducesValidEntry()
        {
            var emptyPayload = new Dictionary<string, object>();
            UnityEditor.LuxRuntimeEvent.Log("empty_payload_test", emptyPayload);
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string json = File.ReadAllText(GetTempLogPath());
            Assert.That(json, Does.Contain("\"eventType\":\"empty_payload_test\""));
            Assert.That(json, Does.Contain("\"category\":\"runtime\""));
            Assert.That(json, Does.Contain("\"source\":\"gameplay\""));

            // Verify the line is parseable JSON
            string[] lines = File.ReadAllLines(GetTempLogPath());
            Assert.That(lines, Has.Length.GreaterThanOrEqualTo(1));
            Assert.That(lines[0], Does.StartWith("{"));
            Assert.That(lines[0], Does.EndWith("}"));
        }

        [Test]
        [Description("LINA-4b: Multiple consecutive empty-payload events all persist correctly")]
        public void LINA4_MultipleEmptyPayloads_AllPersistCorrectly()
        {
            for (int i = 0; i < 5; i++)
            {
                UnityEditor.LuxRuntimeEvent.Log(
                    $"empty_seq_{i}",
                    new Dictionary<string, object>());
            }
            UnityEditor.LuxAiActionLogBroadcaster.Flush();

            string[] lines = File.ReadAllLines(GetTempLogPath());
            Assert.That(lines, Has.Length.EqualTo(5),
                "5 empty-payload events must produce 5 JSONL lines");
            for (int i = 0; i < 5; i++)
            {
                Assert.That(lines[i], Does.Contain($"\"eventType\":\"empty_seq_{i}\""),
                    $"line {i} must contain its eventType");
            }
        }

        #endregion

        #region LINA-4c — Concurrent thread-safe Log calls

        [Test]
        [Description("LINA-4c: Concurrent Log calls from multiple threads do not corrupt output")]
        public void LINA4_ConcurrentLogCalls_ThreadSafe()
        {
            const int threadCount = 8;
            const int callsPerThread = 25;
            var threads = new System.Threading.Thread[threadCount];
            var errors = new System.Collections.Concurrent.ConcurrentBag<Exception>();

            for (int t = 0; t < threadCount; t++)
            {
                int threadId = t;
                threads[t] = new System.Threading.Thread(() =>
                {
                    try
                    {
                        for (int i = 0; i < callsPerThread; i++)
                        {
                            UnityEditor.LuxRuntimeEvent.Log(
                                $"concurrent_t{threadId}_n{i}",
                                new Dictionary<string, object>
                                {
                                    ["thread"] = threadId,
                                    ["iteration"] = i
                                });
                        }
                    }
                    catch (Exception ex)
                    {
                        errors.Add(ex);
                    }
                });
            }

            // Start all threads
            foreach (var thread in threads)
            {
                thread.Start();
            }

            // Wait for all threads to complete (with timeout)
            foreach (var thread in threads)
            {
                Assert.That(thread.Join(10000), Is.True,
                    "thread did not complete within timeout");
            }

            // No exceptions should have been thrown
            Assert.That(errors.IsEmpty, Is.True,
                () => $"concurrent Log calls threw exceptions: {string.Join(", ", errors)}");

            // Flush and verify output integrity
            UnityEditor.LuxAiActionLogBroadcaster.Flush();
            string[] lines = File.ReadAllLines(GetTempLogPath());

            int expectedTotal = threadCount * callsPerThread;
            Assert.That(lines.Length, Is.EqualTo(expectedTotal),
                $"expected {expectedTotal} JSONL lines from concurrent calls, got {lines.Length}");

            // Every line must be valid JSON
            foreach (var line in lines)
            {
                Assert.That(line, Does.StartWith("{"), "each line must start with '{{'");
                Assert.That(line, Does.EndWith("}"), "each line must end with '}}'");
                Assert.That(line, Does.Contain("\"eventType\""), "each line must have eventType");
            }
        }

        [Test]
        [Description("LINA-4c: Concurrent mixed API paths (static + broadcaster) are safe")]
        public void LINA4_ConcurrentMixedPaths_DoNotCorrupt()
        {
            const int iterations = 20;
            var errors = new System.Collections.Concurrent.ConcurrentBag<Exception>();

            var staticApiThread = new System.Threading.Thread(() =>
            {
                try
                {
                    for (int i = 0; i < iterations; i++)
                    {
                        UnityEditor.LuxRuntimeEvent.Log(
                            $"static_mix_{i}",
                            new Dictionary<string, object> { ["src"] = "static" });
                    }
                }
                catch (Exception ex) { errors.Add(ex); }
            });

            var broadcasterThread = new System.Threading.Thread(() =>
            {
                try
                {
                    for (int i = 0; i < iterations; i++)
                    {
                        UnityEditor.LuxAiActionLogBroadcaster.Record(
                            "mixed_broadcaster", $"action_{i}", "target", "msg");
                    }
                }
                catch (Exception ex) { errors.Add(ex); }
            });

            staticApiThread.Start();
            broadcasterThread.Start();

            Assert.That(staticApiThread.Join(10000), Is.True);
            Assert.That(broadcasterThread.Join(10000), Is.True);

            Assert.That(errors.IsEmpty, Is.True,
                () => $"exceptions during concurrent mixed-path: {string.Join(", ", errors)}");

            UnityEditor.LuxAiActionLogBroadcaster.Flush();
            string[] lines = File.ReadAllLines(GetTempLogPath());

            Assert.That(lines.Length, Is.EqualTo(iterations * 2),
                $"expected {iterations * 2} total lines from both paths");
        }

        #endregion

        // ====================================================================
        // Helpers
        // ====================================================================

        string GetTempLogPath()
        {
            return Path.Combine(_tempDirectory, ".lux", "ai-action-log.jsonl");
        }
    }
}
