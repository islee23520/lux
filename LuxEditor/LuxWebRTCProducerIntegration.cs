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
            var sessionId = "unity-editor-" + Guid.NewGuid().ToString("N");
            var token = Environment.GetEnvironmentVariable("LUX_GATEWAY_TOKEN") ?? string.Empty;
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
    }
}
