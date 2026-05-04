using System;
using System.Collections.Generic;
using System.Net;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Linalab.LuxEditor;
using NUnit.Framework;

namespace Linalab.LuxEditor.Tests
{
    public sealed class LuxWebRTCProducerTests
    {
        [Test]
        public async Task producer_initializes_webrtc_subsystem()
        {
            var backend = new RecordingWebRTCBackend();
            var signalingSocket = new RecordingWebSocketClient();
            var eventsSocket = new RecordingWebSocketClient();
            using (var server = await LocalConfigServer.StartAsync("{\"iceServers\":[]}"))
            using (var producer = CreateProducer(backend, signalingSocket, eventsSocket))
            {
                producer.Start("session-a", server.GatewayUrl, "token");

                await WaitUntil(() => backend.InitializeCallCount == 1 && signalingSocket.ConnectUri != null);

                Assert.That(backend.InitializeCallCount, Is.EqualTo(1));
                Assert.That(producer.IsStreaming, Is.True);
            }
        }

        [Test]
        public async Task producer_creates_peer_connection_with_ice_config()
        {
            var backend = new RecordingWebRTCBackend();
            var signalingSocket = new RecordingWebSocketClient();
            var eventsSocket = new RecordingWebSocketClient();
            var configJson = "{\"iceServers\":[{\"urls\":[\"stun:stun.example.test\"],\"username\":\"user\",\"credential\":\"pass\"}]}";
            using (var server = await LocalConfigServer.StartAsync(configJson))
            using (var producer = CreateProducer(backend, signalingSocket, eventsSocket))
            {
                producer.Start("session-b", server.GatewayUrl, "token");

                await WaitUntil(() => backend.CreatePeerConnectionCallCount == 1);

                Assert.That(backend.IceServers, Has.Count.EqualTo(1));
                Assert.That(backend.IceServers[0].urls[0], Is.EqualTo("stun:stun.example.test"));
                Assert.That(backend.IceServers[0].username, Is.EqualTo("user"));
                Assert.That(backend.IceServers[0].credential, Is.EqualTo("pass"));
            }
        }

        [Test]
        public async Task producer_signaling_connects_to_gateway()
        {
            var backend = new RecordingWebRTCBackend();
            var signalingSocket = new RecordingWebSocketClient();
            var eventsSocket = new RecordingWebSocketClient();
            using (var server = await LocalConfigServer.StartAsync("{\"iceServers\":[]}"))
            using (var producer = CreateProducer(backend, signalingSocket, eventsSocket))
            {
                producer.Start("session-c", server.GatewayUrl, "token");

                await WaitUntil(() => signalingSocket.ConnectUri != null && eventsSocket.ConnectUri != null);

                Assert.That(signalingSocket.ConnectUri.AbsolutePath, Is.EqualTo("/remote/signaling/session-c"));
                Assert.That(signalingSocket.ConnectUri.Query, Is.EqualTo("?role=unity"));
                Assert.That(eventsSocket.ConnectUri.AbsolutePath, Is.EqualTo("/events"));
                Assert.That(eventsSocket.ConnectUri.Query, Does.Contain("token=token"));
                Assert.That(eventsSocket.ConnectUri.Query, Does.Contain("role=unity"));
                Assert.That(signalingSocket.Token, Is.EqualTo("token"));
            }
        }

        [Test]
        public void producer_input_receiver_parses_mouse_events()
        {
            var receiver = new RemoteInputReceiver();
            RemoteInputEvent parsed = null;
            receiver.OnInputEvent += input => parsed = input;

            var ok = receiver.ReceiveJson("{\"type\":\"mouse-down\",\"x\":0.25,\"y\":0.75,\"button\":0}");

            Assert.That(ok, Is.True);
            Assert.That(parsed, Is.Not.Null);
            Assert.That(parsed.type, Is.EqualTo("mouse-down"));
            Assert.That(parsed.x, Is.EqualTo(0.25f).Within(0.001f));
            Assert.That(parsed.y, Is.EqualTo(0.75f).Within(0.001f));
            Assert.That(parsed.button, Is.EqualTo(0));
        }

        [Test]
        public void producer_input_receiver_parses_touch_events()
        {
            var receiver = new RemoteInputReceiver();
            RemoteInputEvent parsed = null;
            receiver.OnInputEvent += input => parsed = input;

            var ok = receiver.ReceiveJson("{\"type\":\"touch-move\",\"x\":0.5,\"y\":0.6,\"touchId\":7,\"deltaX\":0.1,\"deltaY\":-0.2}");

            Assert.That(ok, Is.True);
            Assert.That(parsed, Is.Not.Null);
            Assert.That(parsed.type, Is.EqualTo("touch-move"));
            Assert.That(parsed.touchId, Is.EqualTo(7));
            Assert.That(parsed.deltaX, Is.EqualTo(0.1f).Within(0.001f));
            Assert.That(parsed.deltaY, Is.EqualTo(-0.2f).Within(0.001f));
        }

        private static LuxWebRTCProducer CreateProducer(RecordingWebRTCBackend backend, RecordingWebSocketClient signalingSocket, RecordingWebSocketClient eventsSocket)
        {
            return new LuxWebRTCProducer(
                backend,
                () => new WebRTCSignalingClient(() => signalingSocket),
                () => new LuxGatewayEventsClient(() => eventsSocket));
        }

        private static async Task WaitUntil(Func<bool> predicate)
        {
            var deadline = DateTime.UtcNow.AddSeconds(3);
            while (DateTime.UtcNow < deadline)
            {
                if (predicate())
                {
                    return;
                }

                await Task.Delay(25);
            }

            Assert.Fail("Condition was not met before timeout.");
        }

        private sealed class RecordingWebRTCBackend : IWebRTCBackend
        {
            public int InitializeCallCount { get; private set; }
            public int CreatePeerConnectionCallCount { get; private set; }
            public IReadOnlyList<LuxIceServer> IceServers { get; private set; }

            public void Initialize()
            {
                InitializeCallCount++;
            }

            public void StartUpdatePump()
            {
            }

            public void StopUpdatePump()
            {
            }

            public object CreatePeerConnection(IReadOnlyList<LuxIceServer> iceServers)
            {
                CreatePeerConnectionCallCount++;
                IceServers = iceServers;
                return new object();
            }

            public object CaptureEditorCamera(int width, int height, int frameRate)
            {
                return new object();
            }

            public void AddTrack(object peerConnection, object videoTrack)
            {
            }

        public Action<object> LastOnDataChannelCallback { get; private set; }

        public void OnDataChannel(object peerConnection, Action<object> onDataChannel)
        {
            LastOnDataChannelCallback = onDataChannel;
        }

        public string ReadDataChannelLabel(object dataChannel)
        {
            return dataChannel as string ?? string.Empty;
        }

            public void OnDataChannelMessage(object dataChannel, Action<string> onMessage)
            {
            }

            public void OnIceCandidate(object peerConnection, Action<string, string, int> onIceCandidate)
            {
            }

            public Task SetRemoteDescriptionAsync(object peerConnection, string type, string sdp)
            {
                return Task.CompletedTask;
            }

            public Task<string> CreateAnswerAsync(object peerConnection, CancellationToken cancellationToken)
            {
                return Task.FromResult("answer-sdp");
            }

            public Task SetLocalDescriptionAsync(object peerConnection, string type, string sdp)
            {
                return Task.CompletedTask;
            }

            public void AddIceCandidate(object peerConnection, string candidate, string sdpMid, int sdpMLineIndex)
            {
            }

            public void DisposeObject(object instance)
            {
            }
        }

        private sealed class RecordingWebSocketClient : ILuxWebSocketClient
        {
            public Uri ConnectUri { get; private set; }
            public string Token { get; private set; }
            public string LastSentText { get; private set; }
            public bool IsConnected { get; private set; }

            public Task ConnectAsync(Uri uri, string token, CancellationToken cancellationToken)
            {
                ConnectUri = uri;
                Token = token;
                IsConnected = true;
                return Task.CompletedTask;
            }

            public Task<string> ReceiveTextAsync(CancellationToken cancellationToken)
            {
                IsConnected = false;
                return Task.FromResult<string>(null);
            }

            public Task SendTextAsync(string message, CancellationToken cancellationToken)
            {
                LastSentText = message;
                return Task.CompletedTask;
            }

            public void Dispose()
            {
                IsConnected = false;
            }
        }

        private sealed class LocalConfigServer : IDisposable
        {
            private readonly HttpListener listener;
            private readonly Task serverTask;

            private LocalConfigServer(HttpListener listener, Task serverTask, string gatewayUrl)
            {
                this.listener = listener;
                this.serverTask = serverTask;
                GatewayUrl = gatewayUrl;
            }

            public string GatewayUrl { get; private set; }

            public static Task<LocalConfigServer> StartAsync(string responseJson)
            {
                var port = AllocatePort();
                var prefix = "http://127.0.0.1:" + port + "/";
                var listener = new HttpListener();
                listener.Prefixes.Add(prefix);
                listener.Start();
                var serverTask = Task.Run(async () =>
                {
                    var context = await listener.GetContextAsync();
                    var bytes = Encoding.UTF8.GetBytes(responseJson);
                    context.Response.StatusCode = 200;
                    context.Response.ContentType = "application/json";
                    context.Response.ContentLength64 = bytes.Length;
                    await context.Response.OutputStream.WriteAsync(bytes, 0, bytes.Length);
                    context.Response.Close();
                });
                return Task.FromResult(new LocalConfigServer(listener, serverTask, "ws://127.0.0.1:" + port));
            }

            public void Dispose()
            {
                listener.Stop();
                listener.Close();
                try
                {
                    serverTask.Wait(250);
                }
                catch (AggregateException)
                {
                }
            }

            private static int AllocatePort()
            {
                var listener = new System.Net.Sockets.TcpListener(IPAddress.Loopback, 0);
                listener.Start();
                var port = ((IPEndPoint)listener.LocalEndpoint).Port;
                listener.Stop();
                return port;
            }
        }
    }
}
