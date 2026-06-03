using System;
using System.Collections;
using System.Collections.Generic;
using UnityEditor;
using UnityEngine;
using UnityEngine.Networking;
using UnityEngine.SceneManagement;

namespace Linalab.UnityAiBridge.Editor
{
    [InitializeOnLoad]
    public static class LuxPlayLogger
    {
        public const string DefaultServerUrl = "http://localhost:17340";
        public const int DefaultBatchSize = 10;
        public const double DefaultFlushIntervalSeconds = 2.0d;

        private const string ServerUrlPrefsKey = "Linalab.Lux.PlayLogger.ServerUrl";
        private const string BatchSizePrefsKey = "Linalab.Lux.PlayLogger.BatchSize";
        private const string FlushIntervalPrefsKey = "Linalab.Lux.PlayLogger.FlushIntervalSeconds";
        private const string SingleEventEndpointPath = "/api/lux/play/event";
        private const string BatchEventEndpointPath = "/api/lux/play/events/batch";
        private const int MinimumBatchSize = 1;
        private const double MinimumFlushIntervalSeconds = 0.25d;

        private static readonly object PendingEventsLock = new object();
        private static readonly List<PlayEventDto> PendingEvents = new List<PlayEventDto>();
        private static readonly string SessionId = Guid.NewGuid().ToString("N");
        private static readonly string SessionStartedAtUtc = DateTime.UtcNow.ToString("O");
        private static long sequence;
        private static double nextFlushTime;
        private static LuxPlayLoggerRunner runner;
        private static bool flushInProgress;
        private static string lastActiveSceneName;

        public static string ServerUrl
        {
            get
            {
                var configured = GetConfiguredServerUrl();
                if (!string.IsNullOrWhiteSpace(configured))
                {
                    return configured.TrimEnd('/');
                }

                return EditorPrefs.GetString(ServerUrlPrefsKey, DefaultServerUrl).TrimEnd('/');
            }
            set => EditorPrefs.SetString(ServerUrlPrefsKey, string.IsNullOrWhiteSpace(value) ? DefaultServerUrl : value.TrimEnd('/'));
        }

        public static int BatchSize
        {
            get => Mathf.Max(MinimumBatchSize, EditorPrefs.GetInt(BatchSizePrefsKey, DefaultBatchSize));
            set => EditorPrefs.SetInt(BatchSizePrefsKey, Mathf.Max(MinimumBatchSize, value));
        }

        public static double FlushIntervalSeconds
        {
            get => Math.Max(MinimumFlushIntervalSeconds, EditorPrefs.GetFloat(FlushIntervalPrefsKey, (float)DefaultFlushIntervalSeconds));
            set => EditorPrefs.SetFloat(FlushIntervalPrefsKey, (float)Math.Max(MinimumFlushIntervalSeconds, value));
        }

        static LuxPlayLogger()
        {
            SceneManager.sceneLoaded += HandleSceneLoaded;
            Application.logMessageReceived += HandleLogMessageReceived;
            EditorApplication.playModeStateChanged += HandlePlayModeStateChanged;
            EditorApplication.update += HandleEditorUpdate;
            AssemblyReloadEvents.beforeAssemblyReload += FlushPendingEvents;
            EditorApplication.quitting += FlushPendingEvents;
            nextFlushTime = EditorApplication.timeSinceStartup + FlushIntervalSeconds;
        }

        public static void RecordPlayerAction(string action, string target = null, string details = null)
        {
            Enqueue("Action", "PlayerAction", action, target, details, null);
        }

        public static void RecordTrigger(string triggerName, string target = null, string details = null)
        {
            Enqueue("Trigger", "SystemEvent", triggerName, target, details, null);
        }

        public static void RecordDeath(string subject = null, string cause = null, string details = null)
        {
            Enqueue("Death", "StateChange", subject ?? "player", cause, details, null);
        }

        public static void RecordLevelComplete(string levelName = null, string details = null)
        {
            Enqueue("LevelComplete", "StateChange", levelName ?? SceneManager.GetActiveScene().name, null, details, null);
        }

        public static void RecordFeedback(string feedback, string details = null)
        {
            Enqueue("Action", "FeedbackEvent", feedback, null, details, null);
        }

        public static void RecordStateChange(string stateName, string value, string details = null)
        {
            Enqueue("Decision", "StateChange", stateName, value, details, null);
        }

        public static void RecordCustom(string name, string details = null)
        {
            Enqueue("Action", "Custom", name, null, details, null);
        }

        public static void FlushPendingEvents()
        {
            FlushPendingEvents(false);
        }

        private static void HandleEditorUpdate()
        {
            if (Application.isBatchMode)
            {
                return;
            }

            CaptureLegacyInputActions();

            if (EditorApplication.timeSinceStartup >= nextFlushTime)
            {
                nextFlushTime = EditorApplication.timeSinceStartup + FlushIntervalSeconds;
                FlushPendingEvents(false);
            }
        }

        private static void CaptureLegacyInputActions()
        {
            if (!EditorApplication.isPlaying || !IsRuntimePlatformSupported())
            {
                return;
            }

            try
            {
                if (Input.anyKeyDown)
                {
                    Enqueue("Action", "PlayerAction", "input.anyKeyDown", null, "Legacy input key press detected.", null);
                }

                for (var button = 0; button < 3; button++)
                {
                    if (Input.GetMouseButtonDown(button))
                    {
                        Enqueue("Action", "PlayerAction", "input.mouseButtonDown", button.ToString(), null, null);
                    }
                }
            }
            catch (InvalidOperationException)
            {
            }
        }

        private static void HandleSceneLoaded(Scene scene, LoadSceneMode mode)
        {
            lastActiveSceneName = scene.name;
            Enqueue("LevelStart", "StateChange", "scene.loaded", scene.name, mode.ToString(), scene.path);
        }

        private static void HandlePlayModeStateChanged(PlayModeStateChange state)
        {
            Enqueue("Action", "SystemEvent", "playMode." + state, SceneManager.GetActiveScene().name, null, SceneManager.GetActiveScene().path);
        }

        private static void HandleLogMessageReceived(string condition, string stackTrace, LogType type)
        {
            if (type == LogType.Error || type == LogType.Exception || type == LogType.Assert)
            {
                Enqueue("Action", "ErrorEvent", condition, type.ToString(), stackTrace, SceneManager.GetActiveScene().path);
                return;
            }

            if (string.IsNullOrWhiteSpace(condition))
            {
                return;
            }

            if (TryRecordTaggedLog(condition, "[LuxTrigger]", "Trigger", "SystemEvent"))
            {
                return;
            }

            if (TryRecordTaggedLog(condition, "[LuxDeath]", "Death", "StateChange"))
            {
                return;
            }

            if (TryRecordTaggedLog(condition, "[LuxLevelComplete]", "LevelComplete", "StateChange"))
            {
                return;
            }

            TryRecordTaggedLog(condition, "[LuxFeedback]", "Action", "FeedbackEvent");
        }

        private static bool TryRecordTaggedLog(string condition, string prefix, string eventType, string eventCategory)
        {
            if (!condition.StartsWith(prefix, StringComparison.Ordinal))
            {
                return false;
            }

            var value = condition.Substring(prefix.Length).Trim();
            Enqueue(eventType, eventCategory, string.IsNullOrEmpty(value) ? prefix.Trim('[', ']') : value, null, null, SceneManager.GetActiveScene().path);
            return true;
        }

        private static void Enqueue(string eventType, string eventCategory, string name, string target, string details, string scenePath)
        {
            if (!IsRuntimePlatformSupported())
            {
                return;
            }

            var activeScene = SceneManager.GetActiveScene();
            var nextSequence = System.Threading.Interlocked.Increment(ref sequence);
            var playEvent = new PlayEventDto
            {
                session_id = SessionId,
                timestamp = DateTime.UtcNow.ToString("O"),
                event_type = eventType,
                payload = new PlayEventPayloadDto
                {
                    event_category = eventCategory,
                    name = name ?? string.Empty,
                    target = target ?? string.Empty,
                    details = details ?? string.Empty,
                    scene_name = activeScene.name ?? lastActiveSceneName ?? string.Empty,
                    scene_path = scenePath ?? activeScene.path ?? string.Empty,
                    session_started_at = SessionStartedAtUtc,
                    unity_version = Application.unityVersion,
                    platform = Application.platform.ToString(),
                    product_name = Application.productName,
                    runtime_platform_supported = IsRuntimePlatformSupported()
                },
                player_id = SystemInfo.deviceUniqueIdentifier,
                game_state = new PlayGameStateDto
                {
                    is_playing = EditorApplication.isPlaying,
                    is_paused = EditorApplication.isPaused,
                    frame_count = Time.frameCount,
                    realtime_since_startup = Time.realtimeSinceStartup,
                    active_scene = activeScene.name ?? string.Empty
                },
                sequence = nextSequence
            };

            var shouldFlush = false;
            lock (PendingEventsLock)
            {
                PendingEvents.Add(playEvent);
                shouldFlush = PendingEvents.Count >= BatchSize;
            }

            if (shouldFlush)
            {
                FlushPendingEvents(false);
            }
        }

        private static bool IsRuntimePlatformSupported()
        {
            var platform = Application.platform;
            return platform == RuntimePlatform.WebGLPlayer
                || platform == RuntimePlatform.OSXEditor
                || platform == RuntimePlatform.WindowsEditor
                || platform == RuntimePlatform.LinuxEditor
                || platform == RuntimePlatform.OSXPlayer
                || platform == RuntimePlatform.WindowsPlayer
                || platform == RuntimePlatform.LinuxPlayer;
        }

        private static void FlushPendingEvents(bool blocking)
        {
            if (flushInProgress)
            {
                return;
            }

            List<PlayEventDto> eventsToSend;
            lock (PendingEventsLock)
            {
                if (PendingEvents.Count == 0)
                {
                    return;
                }

                eventsToSend = new List<PlayEventDto>(PendingEvents);
                PendingEvents.Clear();
            }

            if (blocking)
            {
                SendEventsBlocking(eventsToSend);
                return;
            }

            EnsureRunner().StartCoroutine(SendEvents(eventsToSend));
        }

        private static LuxPlayLoggerRunner EnsureRunner()
        {
            if (runner != null)
            {
                return runner;
            }

            var gameObject = new GameObject("LuxPlayLoggerRunner")
            {
                hideFlags = HideFlags.HideAndDontSave
            };
            if (Application.isPlaying)
            {
                UnityEngine.Object.DontDestroyOnLoad(gameObject);
            }
            runner = gameObject.AddComponent<LuxPlayLoggerRunner>();
            return runner;
        }

        private static IEnumerator SendEvents(List<PlayEventDto> eventsToSend)
        {
            flushInProgress = true;
            var endpoint = eventsToSend.Count == 1 ? SingleEventEndpointPath : BatchEventEndpointPath;
            var json = eventsToSend.Count == 1 ? JsonUtility.ToJson(eventsToSend[0]) : ToJsonArray(eventsToSend);

            using (var request = CreateJsonPostRequest(BuildEndpointUrl(endpoint), json))
            {
                yield return request.SendWebRequest();

                if (request.result != UnityWebRequest.Result.Success)
                {
                    RequeueFailedEvents(eventsToSend);
                    Debug.LogWarning($"Lux play event submission failed: {request.error}");
                }
            }

            flushInProgress = false;
        }

        private static void SendEventsBlocking(List<PlayEventDto> eventsToSend)
        {
            flushInProgress = true;
            var endpoint = eventsToSend.Count == 1 ? SingleEventEndpointPath : BatchEventEndpointPath;
            var json = eventsToSend.Count == 1 ? JsonUtility.ToJson(eventsToSend[0]) : ToJsonArray(eventsToSend);
            using (var request = CreateJsonPostRequest(BuildEndpointUrl(endpoint), json))
            {
                var operation = request.SendWebRequest();
                while (!operation.isDone)
                {
                }

                if (request.result != UnityWebRequest.Result.Success)
                {
                    RequeueFailedEvents(eventsToSend);
                }
            }

            flushInProgress = false;
        }

        private static UnityWebRequest CreateJsonPostRequest(string url, string json)
        {
            var request = new UnityWebRequest(url, UnityWebRequest.kHttpVerbPOST);
            var body = System.Text.Encoding.UTF8.GetBytes(json);
            request.uploadHandler = new UploadHandlerRaw(body);
            request.downloadHandler = new DownloadHandlerBuffer();
            request.SetRequestHeader("Content-Type", "application/json");
            return request;
        }

        private static string BuildEndpointUrl(string path)
        {
            return ServerUrl.TrimEnd('/') + path;
        }

        private static string GetConfiguredServerUrl()
        {
            foreach (var assembly in AppDomain.CurrentDomain.GetAssemblies())
            {
                var settingsType = assembly.GetType("Linalab.Lux.Editor.LuxBridgeSettings");
                if (settingsType == null)
                {
                    continue;
                }

                var method = settingsType.GetMethod(
                    "TryGetConfiguredGatewayBaseUrl",
                    System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static);
                return method == null ? string.Empty : method.Invoke(null, null) as string ?? string.Empty;
            }

            return string.Empty;
        }

        private static string ToJsonArray(List<PlayEventDto> eventsToSend)
        {
            var parts = new string[eventsToSend.Count];
            for (var i = 0; i < eventsToSend.Count; i++)
            {
                parts[i] = JsonUtility.ToJson(eventsToSend[i]);
            }

            return "[" + string.Join(",", parts) + "]";
        }

        private static void RequeueFailedEvents(List<PlayEventDto> failedEvents)
        {
            lock (PendingEventsLock)
            {
                PendingEvents.InsertRange(0, failedEvents);
            }
        }

        [Serializable]
        private sealed class PlayEventDto
        {
            public string session_id;
            public string timestamp;
            public string event_type;
            public PlayEventPayloadDto payload;
            public string player_id;
            public PlayGameStateDto game_state;
            public long sequence;
        }

        [Serializable]
        private sealed class PlayEventPayloadDto
        {
            public string event_category;
            public string name;
            public string target;
            public string details;
            public string scene_name;
            public string scene_path;
            public string session_started_at;
            public string unity_version;
            public string platform;
            public string product_name;
            public bool runtime_platform_supported;
        }

        [Serializable]
        private sealed class PlayGameStateDto
        {
            public bool is_playing;
            public bool is_paused;
            public int frame_count;
            public float realtime_since_startup;
            public string active_scene;
        }

    }

    internal sealed class LuxPlayLoggerRunner : MonoBehaviour
    {
    }
}
