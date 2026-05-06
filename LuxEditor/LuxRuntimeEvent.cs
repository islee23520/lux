using System;
using System.Collections.Generic;
using System.Globalization;

namespace UnityEditor
{
    /// <summary>
    /// Static API for gameplay code to log runtime events into the LUX event system.
    /// Events are persisted to the same JSONL log as editor/AI events.
    /// Usage: LuxRuntimeEvent.Log("enemy_death", new Dictionary&lt;string, object&gt; { {"enemyId", "goblin_01"} });
    /// </summary>
    public static class LuxRuntimeEvent
    {
        const string RuntimeCategory = "runtime";
        const string GameplaySource = "gameplay";

        public static void Log(string eventType, Dictionary<string, object> payload)
        {
            string normalizedEventType = string.IsNullOrWhiteSpace(eventType) ? "runtime_event" : eventType;
            using (LuxAiActionLogBroadcaster.PushAttribution(GameplaySource, GameplaySource))
            {
                LuxAiActionLogBroadcaster.Record(
                    RuntimeCategory,
                    normalizedEventType,
                    normalizedEventType,
                    "Runtime gameplay event logged.",
                    metadata: ToMetadata(normalizedEventType, payload));
            }
        }

        public static void Log(string eventType)
        {
            Log(eventType, null);
        }

        static IReadOnlyDictionary<string, string> ToMetadata(string eventType, Dictionary<string, object> payload)
        {
            var metadata = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["eventType"] = eventType
            };

            if (payload == null)
            {
                return metadata;
            }

            foreach (var pair in payload)
            {
                if (string.IsNullOrEmpty(pair.Key))
                {
                    continue;
                }

                metadata[pair.Key] = FormatPayloadValue(pair.Value);
            }

            return metadata;
        }

        static string FormatPayloadValue(object value)
        {
            if (value == null)
            {
                return string.Empty;
            }

            if (value is IFormattable formattable)
            {
                return formattable.ToString(null, CultureInfo.InvariantCulture);
            }

            return value.ToString();
        }
    }
}
