using System;
using System.IO;
using System.Net.Sockets;
using System.Text;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    internal static class TcpRequestHelper
    {
        public static string Send(string host, int port, string request, int timeoutMilliseconds = 2000)
        {
            using (var client = new TcpClient())
            {
                if (!client.ConnectAsync(host, port).Wait(timeoutMilliseconds))
                {
                    throw new TimeoutException($"Timed out connecting to {host}:{port}.");
                }

                using (var stream = client.GetStream())
                using (var writer = new StreamWriter(stream, new UTF8Encoding(false), 1024, true))
                using (var reader = new StreamReader(stream, Encoding.UTF8, false, 1024, true))
                {
                    writer.AutoFlush = true;
                    writer.Write(request);
                    writer.Flush();
                    client.Client.Shutdown(SocketShutdown.Send);

                    return reader.ReadToEnd();
                }
            }
        }
    }
}
