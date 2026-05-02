using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Net.WebSockets;
using System.Reflection;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace Linalab.LuxEditor
{
    internal sealed class LuxIceServer
    {
        public string[] urls;
        public string username;
        public string credential;
    }

    internal static class LuxRemoteSessionConfigClient
    {
        public static async Task<IReadOnlyList<LuxIceServer>> GetIceServersAsync(string gatewayUrl, string sessionId, string token, CancellationToken cancellationToken)
        {
            var url = BuildConfigUrl(gatewayUrl, sessionId);
            using (var request = new HttpRequestMessage(HttpMethod.Get, url))
            using (var client = new HttpClient())
            {
                if (!string.IsNullOrEmpty(token))
                {
                    request.Headers.TryAddWithoutValidation("x-lux-token", token);
                }

                try
                {
                    var response = await client.SendAsync(request, cancellationToken);
                    if (!response.IsSuccessStatusCode)
                    {
                        return new LuxIceServer[0];
                    }

                    var json = await response.Content.ReadAsStringAsync();
                    return ParseIceServers(json);
                }
                catch (Exception exception)
                {
                    Debug.LogWarning("Lux WebRTC gateway config unavailable: " + exception.Message);
                    return new LuxIceServer[0];
                }
            }
        }

        internal static IReadOnlyList<LuxIceServer> ParseIceServers(string json)
        {
            var serversJson = LuxWebRTCJson.ExtractJsonValue(json, "iceServers");
            if (string.IsNullOrWhiteSpace(serversJson))
            {
                return new LuxIceServer[0];
            }

            var servers = new List<LuxIceServer>();
            foreach (var objectJson in LuxWebRTCJson.EnumerateObjects(serversJson))
            {
                servers.Add(new LuxIceServer
                {
                    urls = LuxWebRTCJson.ExtractStringArray(objectJson, "urls"),
                    username = LuxWebRTCJson.ExtractString(objectJson, "username"),
                    credential = LuxWebRTCJson.ExtractString(objectJson, "credential")
                });
            }

            return servers;
        }

        private static string BuildConfigUrl(string gatewayUrl, string sessionId)
        {
            var builder = new UriBuilder(gatewayUrl);
            builder.Scheme = builder.Scheme == "wss" ? "https" : "http";
            builder.Path = "/api/remote/sessions/" + Uri.EscapeDataString(sessionId) + "/config";
            builder.Query = string.Empty;
            return builder.Uri.ToString();
        }
    }

    internal static class LuxWebRTCSettings
    {
        private const string Prefix = "Linalab.Lux.WebRTC.";
        public static int Width => EditorPrefs.GetInt(Prefix + "Width", 1280);
        public static int Height => EditorPrefs.GetInt(Prefix + "Height", 720);
        public static int FrameRate => EditorPrefs.GetInt(Prefix + "FrameRate", 30);
        public static string GatewayUrl => EditorPrefs.GetString(Prefix + "GatewayUrl", "ws://127.0.0.1:17340");
        public static bool AutoStart => EditorPrefs.GetBool(Prefix + "AutoStart", false);
    }

    internal static class LuxWebRTCJson
    {
        public static string CreateSdpPayload(string sdp)
        {
            return "{\"sdp\":" + Quote(sdp) + "}";
        }

        public static string CreateIceCandidatePayload(string candidate, string sdpMid, int sdpMLineIndex)
        {
            return "{\"candidate\":" + Quote(candidate) + ",\"sdpMid\":" + Quote(sdpMid) + ",\"sdpMLineIndex\":" + sdpMLineIndex + "}";
        }

        public static string CreateToolExecuteEnvelope(string sessionId, string commandJson)
        {
            return "{\"schema_version\":1,\"event_id\":" + Quote(Guid.NewGuid().ToString("N"))
                + ",\"category\":\"tool\",\"source\":\"lux-webrtc-producer\",\"session_id\":" + Quote(sessionId)
                + ",\"captured_at_utc\":" + Quote(DateTime.UtcNow.ToString("O"))
                + ",\"payload\":{" + "\"kind\":\"tool-execute\",\"executionId\":" + Quote(Guid.NewGuid().ToString("N"))
                + ",\"toolType\":" + Quote(ExtractString(commandJson, "toolType"))
                + ",\"command\":" + Quote(ExtractString(commandJson, "command")) + "}}";
        }

        public static string ExtractString(string json, string fieldName)
        {
            var value = ExtractJsonValue(json, fieldName);
            return string.IsNullOrEmpty(value) || value.Length < 2 || value[0] != '"' ? string.Empty : Unescape(value.Substring(1, value.Length - 2));
        }

        public static int ExtractInt(string json, string fieldName)
        {
            int value;
            return int.TryParse(ExtractJsonValue(json, fieldName), out value) ? value : 0;
        }

        public static string ExtractJsonValue(string json, string fieldName)
        {
            if (string.IsNullOrEmpty(json))
            {
                return string.Empty;
            }

            var key = "\"" + fieldName + "\"";
            var keyIndex = json.IndexOf(key, StringComparison.Ordinal);
            if (keyIndex < 0)
            {
                return string.Empty;
            }

            var colon = json.IndexOf(':', keyIndex + key.Length);
            if (colon < 0)
            {
                return string.Empty;
            }

            var start = colon + 1;
            while (start < json.Length && char.IsWhiteSpace(json[start]))
            {
                start++;
            }

            if (start >= json.Length)
            {
                return string.Empty;
            }

            if (json[start] == '"')
            {
                return ExtractQuoted(json, start);
            }

            if (json[start] == '{' || json[start] == '[')
            {
                return ExtractBalanced(json, start);
            }

            var end = start;
            while (end < json.Length && json[end] != ',' && json[end] != '}' && json[end] != ']')
            {
                end++;
            }

            return json.Substring(start, end - start).Trim();
        }

        public static string[] ExtractStringArray(string json, string fieldName)
        {
            var value = ExtractJsonValue(json, fieldName);
            if (string.IsNullOrWhiteSpace(value))
            {
                var single = ExtractString(json, fieldName);
                return string.IsNullOrEmpty(single) ? new string[0] : new[] { single };
            }

            if (value.Length > 1 && value[0] == '"')
            {
                return new[] { Unescape(value.Substring(1, value.Length - 2)) };
            }

            var values = new List<string>();
            var index = 0;
            while (index < value.Length)
            {
                if (value[index] == '"')
                {
                    var quoted = ExtractQuoted(value, index);
                    values.Add(Unescape(quoted.Substring(1, quoted.Length - 2)));
                    index += quoted.Length;
                }
                else
                {
                    index++;
                }
            }

            return values.ToArray();
        }

        public static IEnumerable<string> EnumerateObjects(string jsonArray)
        {
            if (string.IsNullOrEmpty(jsonArray))
            {
                yield break;
            }

            for (var index = 0; index < jsonArray.Length; index++)
            {
                if (jsonArray[index] == '{')
                {
                    var objectJson = ExtractBalanced(jsonArray, index);
                    if (!string.IsNullOrEmpty(objectJson))
                    {
                        yield return objectJson;
                        index += objectJson.Length - 1;
                    }
                }
            }
        }

        private static string Quote(string value)
        {
            var builder = new StringBuilder();
            builder.Append('"');
            foreach (var character in value ?? string.Empty)
            {
                switch (character)
                {
                    case '\\': builder.Append("\\\\"); break;
                    case '"': builder.Append("\\\""); break;
                    case '\n': builder.Append("\\n"); break;
                    case '\r': builder.Append("\\r"); break;
                    case '\t': builder.Append("\\t"); break;
                    default: builder.Append(character); break;
                }
            }
            builder.Append('"');
            return builder.ToString();
        }

        private static string ExtractQuoted(string json, int start)
        {
            var escaped = false;
            for (var index = start + 1; index < json.Length; index++)
            {
                if (escaped)
                {
                    escaped = false;
                    continue;
                }

                if (json[index] == '\\')
                {
                    escaped = true;
                    continue;
                }

                if (json[index] == '"')
                {
                    return json.Substring(start, index - start + 1);
                }
            }

            return string.Empty;
        }

        private static string ExtractBalanced(string json, int start)
        {
            var open = json[start];
            var close = open == '{' ? '}' : ']';
            var depth = 0;
            var inString = false;
            var escaped = false;
            for (var index = start; index < json.Length; index++)
            {
                var character = json[index];
                if (inString)
                {
                    if (escaped)
                    {
                        escaped = false;
                    }
                    else if (character == '\\')
                    {
                        escaped = true;
                    }
                    else if (character == '"')
                    {
                        inString = false;
                    }
                    continue;
                }

                if (character == '"')
                {
                    inString = true;
                }
                else if (character == open)
                {
                    depth++;
                }
                else if (character == close)
                {
                    depth--;
                    if (depth == 0)
                    {
                        return json.Substring(start, index - start + 1);
                    }
                }
            }
            return string.Empty;
        }

        private static string Unescape(string value)
        {
            return (value ?? string.Empty).Replace("\\\"", "\"").Replace("\\\\", "\\").Replace("\\n", "\n").Replace("\\r", "\r").Replace("\\t", "\t");
        }
    }
}
