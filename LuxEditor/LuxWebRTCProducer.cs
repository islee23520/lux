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
                    webRtc.OnDataChannelMessage(channel, inputReceiver.ReceiveJson);
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

    internal interface IWebRTCBackend
    {
        void Initialize();
        object CreatePeerConnection(IReadOnlyList<LuxIceServer> iceServers);
        object CaptureEditorCamera(int width, int height, int frameRate);
        void AddTrack(object peerConnection, object videoTrack);
        void OnDataChannel(object peerConnection, Action<object> onDataChannel);
        string ReadDataChannelLabel(object dataChannel);
        void OnDataChannelMessage(object dataChannel, Action<string> onMessage);
        void OnIceCandidate(object peerConnection, Action<string, string, int> onIceCandidate);
        void SetRemoteDescription(object peerConnection, string type, string sdp);
        Task<string> CreateAnswerAsync(object peerConnection, CancellationToken cancellationToken);
        void SetLocalDescription(object peerConnection, string type, string sdp);
        void AddIceCandidate(object peerConnection, string candidate, string sdpMid, int sdpMLineIndex);
        void DisposeObject(object instance);
    }

    internal sealed class ReflectionWebRTCBackend : IWebRTCBackend
    {
        private Type webRtcType;

        public void Initialize()
        {
            webRtcType = FindType("Unity.WebRTC.WebRTC");
            if (webRtcType == null)
            {
                throw new InvalidOperationException("com.unity.webrtc 3.0.0 is required for Lux WebRTC streaming. Install it with Package Manager before starting remote streaming.");
            }

            webRtcType.GetMethod("Initialize", BindingFlags.Public | BindingFlags.Static)?.Invoke(null, null);
        }

        public object CreatePeerConnection(IReadOnlyList<LuxIceServer> iceServers)
        {
            var peerType = FindType("Unity.WebRTC.RTCPeerConnection");
            if (peerType == null)
            {
                throw new InvalidOperationException("Unity.WebRTC.RTCPeerConnection was not found.");
            }

            return Activator.CreateInstance(peerType);
        }

        public object CaptureEditorCamera(int width, int height, int frameRate)
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

            var method = typeof(Camera).GetMethod("CaptureStream", new[] { typeof(int), typeof(int), typeof(int) })
                ?? typeof(Camera).GetMethod("CaptureStream", new[] { typeof(int), typeof(int) });
            if (method == null)
            {
                throw new InvalidOperationException("Camera.CaptureStream is unavailable. Verify com.unity.webrtc 3.0.0 is installed.");
            }

            return method.GetParameters().Length == 3
                ? method.Invoke(camera, new object[] { width, height, frameRate })
                : method.Invoke(camera, new object[] { width, height });
        }

        public void AddTrack(object peerConnection, object videoTrack)
        {
            InvokeBestMatch(peerConnection, "AddTrack", videoTrack);
        }

        public void OnDataChannel(object peerConnection, Action<object> onDataChannel)
        {
            if (peerConnection == null || onDataChannel == null)
            {
                return;
            }

            var eventInfo = peerConnection.GetType().GetEvent("OnDataChannel");
            if (eventInfo == null)
            {
                return;
            }

            Action<object> handler = channel => onDataChannel?.Invoke(channel);
            eventInfo.AddEventHandler(peerConnection, Delegate.CreateDelegate(eventInfo.EventHandlerType, handler.Target, handler.Method));
        }

        public string ReadDataChannelLabel(object dataChannel)
        {
            return dataChannel?.GetType().GetProperty("Label")?.GetValue(dataChannel) as string ?? string.Empty;
        }

        public void OnDataChannelMessage(object dataChannel, Action<string> onMessage)
        {
            if (dataChannel == null)
            {
                return;
            }

            var eventInfo = dataChannel.GetType().GetEvent("OnMessage");
            if (eventInfo == null)
            {
                return;
            }

            Action<byte[]> bytesHandler = bytes => onMessage?.Invoke(Encoding.UTF8.GetString(bytes ?? new byte[0]));
            eventInfo.AddEventHandler(dataChannel, Delegate.CreateDelegate(eventInfo.EventHandlerType, bytesHandler.Target, bytesHandler.Method));
        }

        public void OnIceCandidate(object peerConnection, Action<string, string, int> onIceCandidate)
        {
            var eventInfo = peerConnection?.GetType().GetEvent("OnIceCandidate");
            if (eventInfo == null)
            {
                return;
            }

            Action<object> handler = candidate =>
            {
                if (candidate == null)
                {
                    return;
                }

                onIceCandidate?.Invoke(
                    ReadString(candidate, "Candidate"),
                    ReadString(candidate, "SdpMid"),
                    ReadInt(candidate, "SdpMLineIndex"));
            };
            eventInfo.AddEventHandler(peerConnection, Delegate.CreateDelegate(eventInfo.EventHandlerType, handler.Target, handler.Method));
        }

        public void SetRemoteDescription(object peerConnection, string type, string sdp)
        {
            InvokeDescription(peerConnection, "SetRemoteDescription", type, sdp);
        }

        public Task<string> CreateAnswerAsync(object peerConnection, CancellationToken cancellationToken)
        {
            var operation = InvokeBestMatch(peerConnection, "CreateAnswer");
            var desc = ReadProperty(operation, "Desc") ?? operation;
            return Task.FromResult(ReadString(desc, "sdp"));
        }

        public void SetLocalDescription(object peerConnection, string type, string sdp)
        {
            InvokeDescription(peerConnection, "SetLocalDescription", type, sdp);
        }

        public void AddIceCandidate(object peerConnection, string candidate, string sdpMid, int sdpMLineIndex)
        {
            var candidateType = FindType("Unity.WebRTC.RTCIceCandidate") ?? FindType("Unity.WebRTC.RTCIceCandidateInit");
            if (candidateType == null)
            {
                return;
            }

            var instance = Activator.CreateInstance(candidateType);
            WritePropertyOrField(instance, "candidate", candidate);
            WritePropertyOrField(instance, "sdpMid", sdpMid);
            WritePropertyOrField(instance, "sdpMLineIndex", sdpMLineIndex);
            InvokeBestMatch(peerConnection, "AddIceCandidate", instance);
        }

        public void DisposeObject(object instance)
        {
            (instance as IDisposable)?.Dispose();
        }

        private void InvokeDescription(object peerConnection, string methodName, string type, string sdp)
        {
            var descType = FindType("Unity.WebRTC.RTCSessionDescription");
            if (descType == null)
            {
                InvokeBestMatch(peerConnection, methodName, sdp);
                return;
            }

            var desc = Activator.CreateInstance(descType);
            WritePropertyOrField(desc, "type", ParseDescriptionType(type));
            WritePropertyOrField(desc, "sdp", sdp);
            InvokeBestMatch(peerConnection, methodName, desc);
        }

        private static object ParseDescriptionType(string type)
        {
            var enumType = FindType("Unity.WebRTC.RTCSdpType");
            return enumType == null ? type : Enum.Parse(enumType, type, true);
        }

        private static object InvokeBestMatch(object target, string methodName, params object[] arguments)
        {
            if (target == null)
            {
                return null;
            }

            var methods = target.GetType().GetMethods(BindingFlags.Public | BindingFlags.Instance);
            for (var index = 0; index < methods.Length; index++)
            {
                var method = methods[index];
                if (method.Name == methodName && method.GetParameters().Length == arguments.Length)
                {
                    return method.Invoke(target, arguments);
                }
            }

            return null;
        }

        private static Type FindType(string fullName)
        {
            var type = Type.GetType(fullName);
            if (type != null)
            {
                return type;
            }

            var assemblies = AppDomain.CurrentDomain.GetAssemblies();
            for (var index = 0; index < assemblies.Length; index++)
            {
                type = assemblies[index].GetType(fullName);
                if (type != null)
                {
                    return type;
                }
            }

            return null;
        }

        private static object ReadProperty(object instance, string name)
        {
            return instance?.GetType().GetProperty(name, BindingFlags.Public | BindingFlags.Instance)?.GetValue(instance, null);
        }

        private static string ReadString(object instance, string name)
        {
            var value = ReadProperty(instance, name) ?? instance?.GetType().GetField(name, BindingFlags.Public | BindingFlags.Instance)?.GetValue(instance);
            return value == null ? string.Empty : value.ToString();
        }

        private static int ReadInt(object instance, string name)
        {
            var value = ReadProperty(instance, name) ?? instance?.GetType().GetField(name, BindingFlags.Public | BindingFlags.Instance)?.GetValue(instance);
            return value == null ? 0 : Convert.ToInt32(value);
        }

        private static void WritePropertyOrField(object instance, string name, object value)
        {
            var property = instance.GetType().GetProperty(name, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase);
            if (property != null && property.CanWrite)
            {
                property.SetValue(instance, value, null);
                return;
            }

            var field = instance.GetType().GetField(name, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase);
            if (field != null)
            {
                field.SetValue(instance, value);
            }
        }
    }

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
            _ = ReceiveLoopAsync(cancellation.Token);
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
    }

    internal sealed class RemoteInputReceiver
    {
        public event Action<RemoteInputEvent> OnInputEvent;

        public bool ReceiveJson(string json)
        {
            if (string.IsNullOrWhiteSpace(json))
            {
                return false;
            }

            var inputEvent = JsonUtility.FromJson<RemoteInputEvent>(json);
            if (inputEvent == null || string.IsNullOrWhiteSpace(inputEvent.type))
            {
                return false;
            }

            OnInputEvent?.Invoke(inputEvent);
            return true;
        }
    }

    [Serializable]
    public sealed class RemoteInputEvent
    {
        public string type;
        public float x;
        public float y;
        public int button;
        public string key;
        public int touchId;
        public float deltaX;
        public float deltaY;
    }

    internal sealed class LuxGatewayEventsClient
    {
        private readonly Func<ILuxWebSocketClient> socketFactory;
        private ILuxWebSocketClient socket;

        public LuxGatewayEventsClient()
            : this(() => new LuxClientWebSocketTransport())
        {
        }

        internal LuxGatewayEventsClient(Func<ILuxWebSocketClient> socketFactory)
        {
            this.socketFactory = socketFactory ?? throw new ArgumentNullException(nameof(socketFactory));
        }

        public async Task Connect(string url, string token, CancellationToken cancellationToken)
        {
            Disconnect();
            socket = socketFactory();
            await socket.ConnectAsync(new Uri(url), token, cancellationToken);
        }

        public Task SendEventAsync(string eventJson, CancellationToken cancellationToken)
        {
            return socket != null && socket.IsConnected ? socket.SendTextAsync(eventJson, cancellationToken) : Task.CompletedTask;
        }

        public void Disconnect()
        {
            if (socket != null)
            {
                socket.Dispose();
                socket = null;
            }
        }
    }

    internal interface ILuxWebSocketClient : IDisposable
    {
        bool IsConnected { get; }
        Task ConnectAsync(Uri uri, string token, CancellationToken cancellationToken);
        Task<string> ReceiveTextAsync(CancellationToken cancellationToken);
        Task SendTextAsync(string message, CancellationToken cancellationToken);
    }

    internal sealed class LuxClientWebSocketTransport : ILuxWebSocketClient
    {
        private readonly ClientWebSocket webSocket = new ClientWebSocket();

        public bool IsConnected => webSocket.State == WebSocketState.Open;

        public async Task ConnectAsync(Uri uri, string token, CancellationToken cancellationToken)
        {
            if (!string.IsNullOrEmpty(token))
            {
                webSocket.Options.SetRequestHeader("x-lux-token", token);
            }

            await webSocket.ConnectAsync(uri, cancellationToken);
        }

        public async Task<string> ReceiveTextAsync(CancellationToken cancellationToken)
        {
            var buffer = new byte[8192];
            using (var stream = new MemoryStream())
            {
                while (true)
                {
                    var result = await webSocket.ReceiveAsync(new ArraySegment<byte>(buffer), cancellationToken);
                    if (result.MessageType == WebSocketMessageType.Close)
                    {
                        return null;
                    }

                    stream.Write(buffer, 0, result.Count);
                    if (result.EndOfMessage)
                    {
                        return Encoding.UTF8.GetString(stream.ToArray());
                    }
                }
            }
        }

        public Task SendTextAsync(string message, CancellationToken cancellationToken)
        {
            var bytes = Encoding.UTF8.GetBytes(message ?? string.Empty);
            return webSocket.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, cancellationToken);
        }

        public void Dispose()
        {
            webSocket.Dispose();
        }
    }

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
