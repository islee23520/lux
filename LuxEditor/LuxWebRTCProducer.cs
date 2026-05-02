using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Net.WebSockets;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

[assembly: InternalsVisibleTo("Linalab.LuxEditor.Tests.Editor")]

namespace Linalab.LuxEditor
{
    public sealed class LuxWebRTCProducer : IDisposable
    {
        private const string InputChannelLabel = "lux-remote-input";
        private const string AiCommandChannelLabel = "lux-ai-commands";
        private const int DefaultBackoffMilliseconds = 500;
        private const int MaximumBackoffMilliseconds = 5000;

        private readonly IWebRTCBackend webRtc;
        private readonly Func<WebRTCSignalingClient> signalingFactory;
        private readonly Func<LuxGatewayEventsClient> eventsFactory;
        private readonly RemoteInputReceiver inputReceiver = new RemoteInputReceiver();

        private CancellationTokenSource cancellation;
        private WebRTCSignalingClient signaling;
        private LuxGatewayEventsClient eventsClient;
        private object peerConnection;
        private object videoTrack;
        private object inputDataChannel;
        private object aiDataChannel;
        private int retryBackoffMilliseconds = DefaultBackoffMilliseconds;

        public LuxWebRTCProducer()
            : this(new ReflectionWebRTCBackend(), () => new WebRTCSignalingClient(), () => new LuxGatewayEventsClient())
        {
        }

        internal LuxWebRTCProducer(
            IWebRTCBackend webRtc,
            Func<WebRTCSignalingClient> signalingFactory,
            Func<LuxGatewayEventsClient> eventsFactory)
        {
            this.webRtc = webRtc ?? throw new ArgumentNullException(nameof(webRtc));
            this.signalingFactory = signalingFactory ?? throw new ArgumentNullException(nameof(signalingFactory));
            this.eventsFactory = eventsFactory ?? throw new ArgumentNullException(nameof(eventsFactory));
            inputReceiver.OnInputEvent += input => OnRemoteInput?.Invoke(input);
        }

        public bool IsStreaming { get; private set; }
        public string SessionId { get; private set; }

        public event Action OnStreamStarted;
        public event Action OnStreamStopped;
        public event Action<RemoteInputEvent> OnRemoteInput;
        public event Action<string> OnError;

        public void Start(string sessionId, string gatewayUrl, string token)
        {
            if (string.IsNullOrWhiteSpace(sessionId))
            {
                RaiseError("Lux WebRTC session id is required.");
                return;
            }

            if (string.IsNullOrWhiteSpace(gatewayUrl))
            {
                RaiseError("Lux WebRTC gateway URL is not configured.");
                return;
            }

            Stop();
            SessionId = sessionId;
            cancellation = new CancellationTokenSource();
            _ = StartAsync(sessionId, gatewayUrl, token, cancellation.Token);
        }

        public void Stop()
        {
            var wasStreaming = IsStreaming;
            IsStreaming = false;

            if (cancellation != null)
            {
                cancellation.Cancel();
                cancellation.Dispose();
                cancellation = null;
            }

            if (signaling != null)
            {
                signaling.Disconnect();
                signaling = null;
            }

            if (eventsClient != null)
            {
                eventsClient.Disconnect();
                eventsClient = null;
            }

            DisposeWebRTCResources();

            if (wasStreaming)
            {
                OnStreamStopped?.Invoke();
            }
        }

        public void Dispose()
        {
            Stop();
            inputReceiver.OnInputEvent -= input => OnRemoteInput?.Invoke(input);
        }

        private async Task StartAsync(string sessionId, string gatewayUrl, string token, CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                try
                {
                    await StartOnceAsync(sessionId, gatewayUrl, token, cancellationToken);
                    retryBackoffMilliseconds = DefaultBackoffMilliseconds;
                    return;
                }
                catch (OperationCanceledException)
                {
                    return;
                }
                catch (Exception exception)
                {
                    RaiseError("Lux WebRTC producer failed: " + exception.Message);
                    await Task.Delay(retryBackoffMilliseconds, cancellationToken);
                    retryBackoffMilliseconds = Math.Min(retryBackoffMilliseconds * 2, MaximumBackoffMilliseconds);
                }
            }
        }

        private async Task StartOnceAsync(string sessionId, string gatewayUrl, string token, CancellationToken cancellationToken)
        {
            webRtc.Initialize();
            var iceServers = await LuxRemoteSessionConfigClient.GetIceServersAsync(gatewayUrl, sessionId, token, cancellationToken);
            peerConnection = webRtc.CreatePeerConnection(iceServers);
            webRtc.OnIceCandidate(peerConnection, (candidate, sdpMid, sdpMLineIndex) =>
            {
                if (signaling != null)
                {
                    _ = signaling.SendIceCandidate(candidate, sdpMid, sdpMLineIndex);
                }
            });

            videoTrack = webRtc.CaptureEditorCamera(LuxWebRTCSettings.Width, LuxWebRTCSettings.Height, LuxWebRTCSettings.FrameRate);
            webRtc.AddTrack(peerConnection, videoTrack);

            // As answerer, receive DataChannels created by the remote web client (offerer)
            webRtc.OnDataChannel(peerConnection, channel =>
            {
                var label = webRtc.ReadDataChannelLabel(channel);
                if (label == InputChannelLabel)
                {
                    inputDataChannel = channel;
                    webRtc.OnDataChannelMessage(channel, json => inputReceiver.ReceiveJson(json));
                }
                else if (label == AiCommandChannelLabel)
                {
                    aiDataChannel = channel;
                    webRtc.OnDataChannelMessage(channel, commandJson => _ = ForwardAiCommandAsync(commandJson, sessionId, cancellationToken));
                }
            });

            eventsClient = eventsFactory();
            await eventsClient.Connect(BuildEventsUrl(gatewayUrl), token, cancellationToken);

            signaling = signalingFactory();
            signaling.OnOfferReceived += sdp => _ = HandleOfferAsync(sdp, cancellationToken);
            signaling.OnIceCandidateReceived += (candidate, sdpMid, sdpMLineIndex) => webRtc.AddIceCandidate(peerConnection, candidate, sdpMid, sdpMLineIndex);
            await signaling.Connect(BuildSignalingUrl(gatewayUrl, sessionId), token, cancellationToken);

            IsStreaming = true;
            EditorApplication.delayCall += () => OnStreamStarted?.Invoke();
        }

        private async Task HandleOfferAsync(string sdp, CancellationToken cancellationToken)
        {
            try
            {
                webRtc.SetRemoteDescription(peerConnection, "offer", sdp);
                var answer = await webRtc.CreateAnswerAsync(peerConnection, cancellationToken);
                webRtc.SetLocalDescription(peerConnection, "answer", answer);
                if (signaling != null)
                {
                    await signaling.SendAnswer(answer);
                }
            }
            catch (Exception exception)
            {
                RaiseError("Lux WebRTC offer handling failed: " + exception.Message);
            }
        }

        private async Task ForwardAiCommandAsync(string commandJson, string sessionId, CancellationToken cancellationToken)
        {
            if (eventsClient == null || string.IsNullOrWhiteSpace(commandJson))
            {
                return;
            }

            var eventJson = LuxWebRTCJson.CreateToolExecuteEnvelope(sessionId, commandJson);
            await eventsClient.SendEventAsync(eventJson, cancellationToken);
        }

        private void DisposeWebRTCResources()
        {
            webRtc.DisposeObject(aiDataChannel);
            webRtc.DisposeObject(inputDataChannel);
            webRtc.DisposeObject(videoTrack);
            webRtc.DisposeObject(peerConnection);
            aiDataChannel = null;
            inputDataChannel = null;
            videoTrack = null;
            peerConnection = null;
        }

        private void RaiseError(string message)
        {
            Debug.LogWarning(message);
            OnError?.Invoke(message);
        }

        private static string BuildSignalingUrl(string gatewayUrl, string sessionId)
        {
            var builder = new UriBuilder(gatewayUrl);
            builder.Scheme = builder.Scheme == "https" || builder.Scheme == "wss" ? "wss" : "ws";
            builder.Path = "/remote/signaling/" + Uri.EscapeDataString(sessionId);
            builder.Query = "role=unity";
            return builder.Uri.ToString();
        }

        private static string BuildEventsUrl(string gatewayUrl)
        {
            var builder = new UriBuilder(gatewayUrl);
            builder.Scheme = builder.Scheme == "https" || builder.Scheme == "wss" ? "wss" : "ws";
            builder.Path = "/events";
            builder.Query = "role=unity&client_id=lux-webrtc-producer";
            return builder.Uri.ToString();
        }
    }


}
