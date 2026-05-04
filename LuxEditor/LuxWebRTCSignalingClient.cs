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
    internal sealed class WebRTCSignalingClient
    {
        private readonly Func<ILuxWebSocketClient> socketFactory;
        private ILuxWebSocketClient socket;
        private CancellationTokenSource cancellation;

        public WebRTCSignalingClient()
            : this(() => new LuxClientWebSocketTransport())
        {
        }

        internal WebRTCSignalingClient(Func<ILuxWebSocketClient> socketFactory)
        {
            this.socketFactory = socketFactory ?? throw new ArgumentNullException(nameof(socketFactory));
        }

        public event Action<string> OnOfferReceived;
        public event Action<string> OnAnswerReceived;
        public event Action<string, string, int> OnIceCandidateReceived;

        public Task Connect(string url)
        {
            return Connect(url, string.Empty, CancellationToken.None);
        }

        internal async Task Connect(string url, string token, CancellationToken cancellationToken)
        {
            Disconnect();
            cancellation = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            socket = socketFactory();
            await socket.ConnectAsync(new Uri(url), token, cancellation.Token);
            ObserveTask(ReceiveLoopAsync(cancellation.Token), "Lux WebRTC signaling receive loop failed");
        }

        public Task SendOffer(string sdp)
        {
            return SendAsync("sdp-offer", LuxWebRTCJson.CreateSdpPayload(sdp), CancellationToken.None);
        }

        public Task SendAnswer(string sdp)
        {
            return SendAsync("sdp-answer", LuxWebRTCJson.CreateSdpPayload(sdp), CancellationToken.None);
        }

        public Task SendIceCandidate(string candidate, string sdpMid, int sdpMLineIndex)
        {
            return SendAsync("ice-candidate", LuxWebRTCJson.CreateIceCandidatePayload(candidate, sdpMid, sdpMLineIndex), CancellationToken.None);
        }

        public void Disconnect()
        {
            if (cancellation != null)
            {
                cancellation.Cancel();
                cancellation.Dispose();
                cancellation = null;
            }

            if (socket != null)
            {
                socket.Dispose();
                socket = null;
            }
        }

        private async Task SendAsync(string type, string payload, CancellationToken cancellationToken)
        {
            if (socket == null || !socket.IsConnected)
            {
                return;
            }

            await socket.SendTextAsync("{\"type\":\"" + type + "\",\"payload\":" + payload + "}", cancellationToken);
        }

        private async Task ReceiveLoopAsync(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested && socket != null && socket.IsConnected)
            {
                var json = await socket.ReceiveTextAsync(cancellationToken);
                if (json == null)
                {
                    return;
                }

                var type = LuxWebRTCJson.ExtractString(json, "type");
                var payload = LuxWebRTCJson.ExtractJsonValue(json, "payload");
                if (string.Equals(type, "sdp-offer", StringComparison.Ordinal))
                {
                    OnOfferReceived?.Invoke(LuxWebRTCJson.ExtractString(payload, "sdp"));
                }
                else if (string.Equals(type, "sdp-answer", StringComparison.Ordinal))
                {
                    OnAnswerReceived?.Invoke(LuxWebRTCJson.ExtractString(payload, "sdp"));
                }
                else if (string.Equals(type, "ice-candidate", StringComparison.Ordinal))
                {
                    OnIceCandidateReceived?.Invoke(
                        LuxWebRTCJson.ExtractString(payload, "candidate"),
                        LuxWebRTCJson.ExtractString(payload, "sdpMid"),
                        LuxWebRTCJson.ExtractInt(payload, "sdpMLineIndex"));
                }
            }
        }

        private static void ObserveTask(Task task, string errorPrefix)
        {
            if (task == null)
            {
                return;
            }

            task.ContinueWith(completedTask =>
            {
                var exception = completedTask.Exception?.GetBaseException();
                if (exception != null && !(exception is OperationCanceledException))
                {
                    Debug.LogWarning(errorPrefix + ": " + exception.Message);
                }
            }, CancellationToken.None, TaskContinuationOptions.OnlyOnFaulted, TaskScheduler.Default);
        }
    }
}
