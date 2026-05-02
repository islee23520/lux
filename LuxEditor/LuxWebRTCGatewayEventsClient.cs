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
}
