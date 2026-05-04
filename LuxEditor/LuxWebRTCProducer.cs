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
        private const int MaximumStartAttempts = 3;
        private static readonly int MainThreadId = Thread.CurrentThread.ManagedThreadId;

        private readonly IWebRTCBackend webRtc;
        private readonly Func<WebRTCSignalingClient> signalingFactory;
        private readonly Func<LuxGatewayEventsClient> eventsFactory;
        private readonly RemoteInputReceiver inputReceiver = new RemoteInputReceiver();

        private CancellationTokenSource cancellation;
        private WebRTCSignalingClient signaling;
        private LuxGatewayEventsClient eventsClient;
        private IReadOnlyList<LuxIceServer> iceServers = Array.Empty<LuxIceServer>();
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
            iceServers = Array.Empty<LuxIceServer>();
            cancellation = new CancellationTokenSource();
            ObserveTask(StartAsync(sessionId, gatewayUrl, token, cancellation.Token), "Lux WebRTC startup task failed");
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
            webRtc.StopUpdatePump();
            iceServers = Array.Empty<LuxIceServer>();

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
            for (var attempt = 1; attempt <= MaximumStartAttempts && !cancellationToken.IsCancellationRequested; attempt++)
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
                    await RunOnMainThreadAsync(() =>
                    {
                        DisposeWebRTCResources();
                        webRtc.StopUpdatePump();
                    }, CancellationToken.None);
                    if (attempt >= MaximumStartAttempts)
                    {
                        RaiseError("Lux WebRTC producer startup stopped after " + MaximumStartAttempts + " attempts.");
                        return;
                    }

                    await Task.Delay(retryBackoffMilliseconds, cancellationToken);
                    retryBackoffMilliseconds = Math.Min(retryBackoffMilliseconds * 2, MaximumBackoffMilliseconds);
                }
            }
        }

        private async Task StartOnceAsync(string sessionId, string gatewayUrl, string token, CancellationToken cancellationToken)
        {
            iceServers = await LuxRemoteSessionConfigClient.GetIceServersAsync(gatewayUrl, sessionId, token, cancellationToken);

            eventsClient = eventsFactory();
            await eventsClient.Connect(BuildEventsUrl(gatewayUrl, token), token, cancellationToken);

            signaling = signalingFactory();
            signaling.OnOfferReceived += sdp => ObserveTask(HandleOfferAsync(sdp, cancellationToken), "Lux WebRTC offer task failed");
            signaling.OnIceCandidateReceived += (candidate, sdpMid, sdpMLineIndex) => ObserveTask(
                RunOnMainThreadAsync(() => webRtc.AddIceCandidate(peerConnection, candidate, sdpMid, sdpMLineIndex), cancellationToken),
                "Lux WebRTC ICE candidate receive failed");
            await signaling.Connect(BuildSignalingUrl(gatewayUrl, sessionId, token), token, cancellationToken);

            IsStreaming = true;
            EditorApplication.delayCall += () => OnStreamStarted?.Invoke();
        }

        private async Task HandleOfferAsync(string sdp, CancellationToken cancellationToken)
        {
            try
            {
                await EnsurePeerConnectionAsync(cancellationToken);
                string answer = null;
                await RunOnMainThreadAsync(async () =>
                {
                    await webRtc.SetRemoteDescriptionAsync(peerConnection, "offer", sdp);
                    answer = await webRtc.CreateAnswerAsync(peerConnection, cancellationToken);
                    await webRtc.SetLocalDescriptionAsync(peerConnection, "answer", answer);
                }, cancellationToken);
                if (signaling != null && !string.IsNullOrEmpty(answer))
                {
                    await signaling.SendAnswer(answer);
                }
            }
            catch (Exception exception)
            {
                RaiseError("Lux WebRTC offer handling failed: " + exception.Message);
            }
        }

        private Task EnsurePeerConnectionAsync(CancellationToken cancellationToken)
        {
            if (peerConnection != null)
            {
                return Task.FromResult(true);
            }

            return RunOnMainThreadAsync(() =>
            {
                var camera = Camera.main;
                if (camera == null)
                {
                    camera = UnityEngine.Object.FindObjectOfType<Camera>();
                }

                if (camera == null)
                {
                    throw new InvalidOperationException("No Unity camera was found to capture for Lux WebRTC streaming.");
                }

                webRtc.Initialize();
                webRtc.StartUpdatePump();
                peerConnection = webRtc.CreatePeerConnection(iceServers);
                webRtc.OnIceCandidate(peerConnection, (candidate, sdpMid, sdpMLineIndex) =>
                {
                    if (signaling != null)
                    {
                        ObserveTask(signaling.SendIceCandidate(candidate, sdpMid, sdpMLineIndex), "Lux WebRTC ICE candidate send failed");
                    }
                });

                videoTrack = webRtc.CaptureEditorCamera(LuxWebRTCSettings.Width, LuxWebRTCSettings.Height, LuxWebRTCSettings.FrameRate);
                webRtc.AddTrack(peerConnection, videoTrack);

                webRtc.OnDataChannel(peerConnection, channel =>
                {
                    ObserveTask(RunOnMainThreadAsync(() =>
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
                            webRtc.OnDataChannelMessage(channel, commandJson => ObserveTask(ForwardAiCommandAsync(commandJson, SessionId, cancellationToken), "Lux WebRTC AI command forwarding failed"));
                        }
                    }, cancellationToken), "Lux WebRTC data channel setup failed");
                });
            }, cancellationToken);
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
            if (Thread.CurrentThread.ManagedThreadId == MainThreadId)
            {
                Debug.LogWarning(message);
                OnError?.Invoke(message);
                return;
            }

            EditorApplication.delayCall += () =>
            {
                Debug.LogWarning(message);
                OnError?.Invoke(message);
            };
        }

        private Task RunOnMainThreadAsync(Action action, CancellationToken cancellationToken)
        {
            if (action == null)
            {
                return Task.CompletedTask;
            }

            if (Thread.CurrentThread.ManagedThreadId == MainThreadId)
            {
                action();
                return Task.CompletedTask;
            }

            var completion = new TaskCompletionSource<object>();
            EditorApplication.delayCall += () =>
            {
                try
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    action();
                    completion.TrySetResult(null);
                }
                catch (Exception exception)
                {
                    completion.TrySetException(exception);
                }
            };

            return completion.Task;
        }

        private Task RunOnMainThreadAsync(Func<Task> asyncAction, CancellationToken cancellationToken)
        {
            if (asyncAction == null)
            {
                return Task.CompletedTask;
            }

            if (Thread.CurrentThread.ManagedThreadId == MainThreadId)
            {
                return asyncAction();
            }

            var completion = new TaskCompletionSource<object>();
            EditorApplication.delayCall += async () =>
            {
                try
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    await asyncAction();
                    completion.TrySetResult(null);
                }
                catch (Exception exception)
                {
                    completion.TrySetException(exception);
                }
            };

            return completion.Task;
        }

        private Task<T> RunOnMainThreadAsync<T>(Func<T> action, CancellationToken cancellationToken)
        {
            if (action == null)
            {
                throw new ArgumentNullException(nameof(action));
            }

            if (Thread.CurrentThread.ManagedThreadId == MainThreadId)
            {
                return Task.FromResult(action());
            }

            var completion = new TaskCompletionSource<T>();
            EditorApplication.delayCall += () =>
            {
                try
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    completion.TrySetResult(action());
                }
                catch (Exception exception)
                {
                    completion.TrySetException(exception);
                }
            };

            return completion.Task;
        }

        private void ObserveTask(Task task, string errorPrefix)
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
                    RaiseError(errorPrefix + ": " + exception.Message);
                }
            }, CancellationToken.None, TaskContinuationOptions.OnlyOnFaulted, TaskScheduler.Default);
        }

        private static string BuildSignalingUrl(string gatewayUrl, string sessionId, string token)
        {
            var builder = new UriBuilder(gatewayUrl);
            builder.Scheme = builder.Scheme == "https" || builder.Scheme == "wss" ? "wss" : "ws";
            builder.Path = "/remote/signaling/" + Uri.EscapeDataString(sessionId);
            var query = "role=unity";
            if (!string.IsNullOrEmpty(token))
            {
                query += "&token=" + Uri.EscapeDataString(token);
            }
            builder.Query = query;
            return builder.Uri.ToString();
        }

        private static string BuildEventsUrl(string gatewayUrl, string token)
        {
            var builder = new UriBuilder(gatewayUrl);
            builder.Scheme = builder.Scheme == "https" || builder.Scheme == "wss" ? "wss" : "ws";
            builder.Path = "/events";
            var query = "role=unity&client_id=lux-webrtc-producer";
            if (!string.IsNullOrEmpty(token))
            {
                query += "&token=" + Uri.EscapeDataString(token);
            }
            builder.Query = query;
            return builder.Uri.ToString();
        }

    }

}
