using System;
using UnityEditor;
using UnityEngine;
using UnityEngine.Networking;

namespace Linalab.Lux.Editor
{
    [InitializeOnLoad]
    public sealed class LuxServerStatusIndicator : EditorWindow
    {
        const double HeartbeatIntervalSeconds = 60.0;

        static LuxServerStatus _status = LuxServerStatus.Unknown;
        static string _message = "Not checked yet";
        static long _uptimeSeconds;
        static double _nextHeartbeatAt;
        static UnityWebRequest _request;
        static double _requestStartedAt;

        static LuxServerStatusIndicator()
        {
            EditorApplication.update += UpdateHeartbeat;
            _nextHeartbeatAt = 0.0;
        }

        [MenuItem("Tools/Linalab/Lux/Server Status")]
        public static void ShowWindow()
        {
            var window = GetWindow<LuxServerStatusIndicator>();
            window.titleContent = new GUIContent("Lux Server", EditorGUIUtility.IconContent("d_UnityEditor.ConsoleWindow").image);
            window.minSize = new Vector2(320f, 150f);
            window.Show();
            RequestHeartbeat(force: true);
        }

        void OnGUI()
        {
            EditorGUILayout.LabelField("Lux Gateway Server", EditorStyles.boldLabel);
            EditorGUILayout.Space();

            using (new EditorGUILayout.HorizontalScope())
            {
                Rect rect = GUILayoutUtility.GetRect(16f, 16f, GUILayout.Width(20f));
                EditorGUI.DrawRect(new Rect(rect.x, rect.y + 2f, 14f, 14f), StatusColor(_status));
                EditorGUILayout.LabelField(StatusLabel(_status), EditorStyles.boldLabel);
            }

            EditorGUILayout.LabelField("URL", LuxBridgeSettings.GetGatewayBaseUrl());
            EditorGUILayout.LabelField("Uptime", FormatUptime(_uptimeSeconds));
            EditorGUILayout.HelpBox(_message, MessageType.None);

            using (new EditorGUILayout.HorizontalScope())
            {
                if (GUILayout.Button("Check Now"))
                {
                    RequestHeartbeat(force: true);
                }

                if (GUILayout.Button("Write Bridge Settings"))
                {
                    LuxBridgeSettings.WriteProjectSettings();
                }
            }
        }

        static void UpdateHeartbeat()
        {
            if (_request != null)
            {
                if (_request.isDone)
                {
                    CompleteHeartbeat();
                }
                else if (EditorApplication.timeSinceStartup - _requestStartedAt > 10.0)
                {
                    _request.Abort();
                    CompleteHeartbeat();
                }

                return;
            }

            if (EditorApplication.timeSinceStartup >= _nextHeartbeatAt)
            {
                RequestHeartbeat(force: false);
            }
        }

        static void RequestHeartbeat(bool force)
        {
            if (_request != null && !force)
            {
                return;
            }

            if (_request != null)
            {
                _request.Dispose();
                _request = null;
            }

            string url = LuxBridgeSettings.GetGatewayBaseUrl() + "/api/heartbeat";
            _request = new UnityWebRequest(url, UnityWebRequest.kHttpVerbPOST)
            {
                downloadHandler = new DownloadHandlerBuffer(),
                uploadHandler = new UploadHandlerRaw(new byte[0]),
                timeout = 10
            };
            _request.SetRequestHeader("Content-Type", "application/json");
            _request.SendWebRequest();
            _requestStartedAt = EditorApplication.timeSinceStartup;
            _nextHeartbeatAt = EditorApplication.timeSinceStartup + HeartbeatIntervalSeconds;
        }

        static void CompleteHeartbeat()
        {
            try
            {
                if (_request.result == UnityWebRequest.Result.Success)
                {
                    var response = JsonUtility.FromJson<HeartbeatResponse>(_request.downloadHandler.text);
                    _uptimeSeconds = response == null ? 0 : response.uptime_seconds;
                    _status = LuxServerStatus.Alive;
                    _message = "Server heartbeat OK.";
                }
                else if (_request.result == UnityWebRequest.Result.ConnectionError)
                {
                    _status = LuxServerStatus.Unreachable;
                    _message = $"Server unreachable: {_request.error}";
                }
                else
                {
                    _status = LuxServerStatus.Error;
                    _message = $"Server error: HTTP {_request.responseCode} {_request.error}";
                }
            }
            finally
            {
                _request.Dispose();
                _request = null;
                RepaintOpenWindows();
            }
        }

        static void RepaintOpenWindows()
        {
            foreach (var window in Resources.FindObjectsOfTypeAll<LuxServerStatusIndicator>())
            {
                window.Repaint();
            }
        }

        static Color StatusColor(LuxServerStatus status)
        {
            switch (status)
            {
                case LuxServerStatus.Alive:
                    return new Color(0.2f, 0.75f, 0.25f);
                case LuxServerStatus.Error:
                    return new Color(0.85f, 0.2f, 0.2f);
                case LuxServerStatus.Unreachable:
                    return new Color(0.95f, 0.7f, 0.15f);
                default:
                    return new Color(0.6f, 0.6f, 0.6f);
            }
        }

        static string StatusLabel(LuxServerStatus status)
        {
            switch (status)
            {
                case LuxServerStatus.Alive:
                    return "Alive";
                case LuxServerStatus.Unreachable:
                    return "Unreachable";
                case LuxServerStatus.Error:
                    return "Error";
                default:
                    return "Unknown";
            }
        }

        static string FormatUptime(long seconds)
        {
            if (seconds <= 0)
            {
                return "-";
            }

            TimeSpan uptime = TimeSpan.FromSeconds(seconds);
            return uptime.TotalDays >= 1.0
                ? $"{(int)uptime.TotalDays}d {uptime.Hours}h {uptime.Minutes}m"
                : $"{uptime.Hours}h {uptime.Minutes}m {uptime.Seconds}s";
        }

        enum LuxServerStatus
        {
            Unknown,
            Alive,
            Unreachable,
            Error
        }

        [Serializable]
        sealed class HeartbeatResponse
        {
            public string status;
            public long uptime_seconds;
        }
    }
}
