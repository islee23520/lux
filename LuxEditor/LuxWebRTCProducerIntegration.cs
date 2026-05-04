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
    [InitializeOnLoad]
    public static class LuxWebRTCProducerIntegration
    {
        private static LuxWebRTCProducer producer;

        static LuxWebRTCProducerIntegration()
        {
            AssemblyReloadEvents.beforeAssemblyReload += StopStreaming;
            EditorApplication.quitting += StopStreaming;
        }

        [MenuItem("Tools/Linalab/Lux/WebRTC Remote/Start Streaming")]
        public static void StartStreaming()
        {
            var token = Environment.GetEnvironmentVariable("LUX_GATEWAY_TOKEN") ?? string.Empty;
            var sessionId = CreateRemoteSession(LuxWebRTCSettings.GatewayUrl, token);
            if (string.IsNullOrEmpty(sessionId))
            {
                Debug.LogWarning("Lux WebRTC remote streaming failed: could not create gateway session.");
                return;
            }

            producer = producer ?? new LuxWebRTCProducer();
            producer.Start(sessionId, LuxWebRTCSettings.GatewayUrl, token);
            Debug.Log("Lux WebRTC remote streaming starting for session " + sessionId + ".");
        }

        [MenuItem("Tools/Linalab/Lux/WebRTC Remote/Stop Streaming")]
        public static void StopStreaming()
        {
            if (producer != null)
            {
                producer.Stop();
                producer = null;
            }
        }

        [MenuItem("Tools/Linalab/Lux/WebRTC Remote/Status")]
        public static void ShowStatus()
        {
            var status = producer != null && producer.IsStreaming ? "streaming" : "stopped";
            Debug.Log("Lux WebRTC remote streaming status: " + status + ". Gateway: " + LuxWebRTCSettings.GatewayUrl + ".");
        }

        private static string CreateRemoteSession(string gatewayUrl, string token)
        {
            try
            {
                return Task.Run(() => CreateRemoteSessionAsync(gatewayUrl, token)).GetAwaiter().GetResult();
            }
            catch (Exception exception)
            {
                Debug.LogWarning("Lux WebRTC remote session creation failed: " + exception.Message);
                return null;
            }
        }

        private static async Task<string> CreateRemoteSessionAsync(string gatewayUrl, string token)
        {
            var builder = new UriBuilder(gatewayUrl);
            builder.Scheme = builder.Scheme == "wss" || builder.Scheme == "https" ? "https" : "http";
            builder.Path = "/api/remote/sessions";
            builder.Query = string.Empty;

            using (var client = new HttpClient())
            using (var request = new HttpRequestMessage(HttpMethod.Post, builder.Uri.ToString()))
            {
                request.Content = new StringContent("{}", Encoding.UTF8, "application/json");
                if (!string.IsNullOrEmpty(token))
                {
                    request.Headers.TryAddWithoutValidation("x-lux-token", token);
                }

                var response = await client.SendAsync(request);
                if (!response.IsSuccessStatusCode)
                {
                    return null;
                }

                var json = await response.Content.ReadAsStringAsync();
                return LuxWebRTCJson.ExtractString(json, "id");
            }
        }
    }
}
