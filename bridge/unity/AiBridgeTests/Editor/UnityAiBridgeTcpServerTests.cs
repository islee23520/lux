using System.IO;
using System.Net;
using System.Reflection;
using NUnit.Framework;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    public sealed class UnityAiBridgeTcpServerTests
    {
        [SetUp]
        public void SetUp()
        {
            DiscoveryFileCleanup.DeleteDiscoveryFile();
            UnityAiBridgeTcpServer.StopShared();
        }

        [TearDown]
        public void TearDown()
        {
            UnityAiBridgeTcpServer.StopShared();
            DiscoveryFileCleanup.DeleteDiscoveryFile();
        }

        [Test]
        public void Send_AuthorizedPing_ReturnsOkJsonLine()
        {
            using (var server = StartServer())
            {
                var discovery = ReadDiscovery();
                var request = CreateRequestJson("req-ping", UnityAiBridgeProtocol.CommandPing, discovery.token);

                var responseText = TcpRequestHelper.Send(discovery.host, discovery.port, request + "\n");
                var response = ParseSingleResponse(responseText);

                Assert.That(response.ok, Is.True);
                Assert.That(response.requestId, Is.EqualTo("req-ping"));
                Assert.That(response.payload, Is.Not.Null);
                Assert.That(response.payload.ping, Is.Not.Null);
                Assert.That(response.payload.ping.status, Is.EqualTo("ok"));
            }
        }

        [Test]
        public void Send_GetSelectedFileContext_ReturnsCachedSelectedAssetMetadata()
        {
            var previousSelection = Selection.objects;

            using (var scope = TemporaryAssetScope.Create())
            {
                var fileContentsProbe = "tcp-selected-context-probe-91c2e2";
                var assetPath = scope.CreateTextAsset("tcp-selected-context", fileContentsProbe);
                var asset = AssetDatabase.LoadAssetAtPath<TextAsset>(assetPath);

                Assert.That(asset, Is.Not.Null);

                try
                {
                    Selection.objects = new Object[] { asset };
                    UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();

                    using (var server = StartServer())
                    {
                        var discovery = ReadDiscovery();
                        var request = CreateRequestJson("req-selected", UnityAiBridgeProtocol.CommandGetSelectedFileContext, discovery.token);

                        var responseText = TcpRequestHelper.Send(discovery.host, discovery.port, request + "\n");
                        var response = ParseSingleResponse(responseText);
                        var payload = response.payload.selectedFileContext;

                        Assert.That(response.ok, Is.True);
                        Assert.That(response.requestId, Is.EqualTo("req-selected"));
                        Assert.That(payload, Is.Not.Null);
                        Assert.That(payload.projectName, Is.EqualTo(Application.productName));
                        Assert.That(payload.projectPath, Is.EqualTo(Directory.GetCurrentDirectory()));
                        Assert.That(payload.unityVersion, Is.EqualTo(Application.unityVersion));
                        Assert.That(payload.selectionCount, Is.EqualTo(1));
                        Assert.That(payload.selectedFiles, Is.Not.Null);
                        Assert.That(payload.selectedFiles.Length, Is.EqualTo(1));
                        Assert.That(payload.selectedFiles[0].assetPath, Is.EqualTo(assetPath));
                        Assert.That(payload.selectedFiles[0].name, Is.EqualTo("tcp-selected-context"));
                        Assert.That(payload.selectedFiles[0].extension, Is.EqualTo(".txt"));
                        Assert.That(payload.selectedFiles[0].exists, Is.True);
                        Assert.That(responseText, Does.Not.Contain(fileContentsProbe));
                    }
                }
                finally
                {
                    Selection.objects = previousSelection;
                    UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();
                }
            }
        }

        [Test]
        public void Send_GetProtocolInfo_ReturnsBackendVersionInJsonLine()
        {
            using (var server = StartServer())
            {
                var discovery = ReadDiscovery();
                var request = CreateRequestJson("req-protocol-info", UnityAiBridgeProtocol.CommandGetProtocolInfo, discovery.token);

                var responseText = TcpRequestHelper.Send(discovery.host, discovery.port, request + "\n");
                var response = ParseSingleResponse(responseText);

                Assert.That(response.ok, Is.True);
                Assert.That(response.payload, Is.Not.Null);
                Assert.That(response.payload.protocolInfo, Is.Not.Null);
                Assert.That(response.payload.protocolInfo.protocolVersion, Is.EqualTo(UnityAiBridgeProtocol.ProtocolVersion));
                Assert.That(response.payload.protocolInfo.backendVersion, Is.EqualTo(UnityAiBridgeProtocol.BackendVersion));
                Assert.That(response.payload.protocolInfo.commands, Does.Contain(UnityAiBridgeProtocol.CommandPing));
            }
        }

        [Test]
        public void Send_GetSelectedFileContext_PreservesMultipleSelectionOrderAndCounts()
        {
            var previousSelection = Selection.objects;

            using (var scope = TemporaryAssetScope.Create())
            {
                var firstAssetPath = scope.CreateTextAsset("tcp-selected-first", "first selection payload");
                var secondAssetPath = scope.CreateTextAsset("tcp-selected-second", "second selection payload");
                var firstAsset = AssetDatabase.LoadAssetAtPath<TextAsset>(firstAssetPath);
                var secondAsset = AssetDatabase.LoadAssetAtPath<TextAsset>(secondAssetPath);

                Assert.That(firstAsset, Is.Not.Null);
                Assert.That(secondAsset, Is.Not.Null);

                try
                {
                    Selection.objects = new Object[] { firstAsset, secondAsset };
                    UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();

                    using (var server = StartServer())
                    {
                        var discovery = ReadDiscovery();
                        var request = CreateRequestJson("req-selected-multiple", UnityAiBridgeProtocol.CommandGetSelectedFileContext, discovery.token);

                        var responseText = TcpRequestHelper.Send(discovery.host, discovery.port, request + "\n");
                        var response = ParseSingleResponse(responseText);
                        var selectedFiles = response.payload.selectedFileContext.selectedFiles;

                        Assert.That(response.ok, Is.True);
                        Assert.That(response.requestId, Is.EqualTo("req-selected-multiple"));
                        Assert.That(response.payload.selectedFileContext.selectionCount, Is.EqualTo(2));
                        Assert.That(selectedFiles, Is.Not.Null);
                        Assert.That(selectedFiles.Length, Is.EqualTo(2));
                        Assert.That(selectedFiles[0].assetPath, Is.EqualTo(firstAssetPath));
                        Assert.That(selectedFiles[0].selectionIndex, Is.EqualTo(0));
                        Assert.That(selectedFiles[0].selectionCount, Is.EqualTo(2));
                        Assert.That(selectedFiles[1].assetPath, Is.EqualTo(secondAssetPath));
                        Assert.That(selectedFiles[1].selectionIndex, Is.EqualTo(1));
                        Assert.That(selectedFiles[1].selectionCount, Is.EqualTo(2));
                    }
                }
                finally
                {
                    Selection.objects = previousSelection;
                    UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();
                }
            }
        }

        [Test]
        public void Send_UnauthorizedToken_ReturnsErrorAndServerContinues()
        {
            using (var server = StartServer())
            {
                var discovery = ReadDiscovery();
                var unauthorizedRequest = CreateRequestJson("req-denied", UnityAiBridgeProtocol.CommandPing, "wrong-token");
                var unauthorizedResponse = ParseSingleResponse(TcpRequestHelper.Send(discovery.host, discovery.port, unauthorizedRequest + "\n"));

                Assert.That(unauthorizedResponse.ok, Is.False);
                Assert.That(unauthorizedResponse.errorCode, Is.EqualTo(UnityAiBridgeProtocol.ErrorCodeUnauthorized));

                var authorizedRequest = CreateRequestJson("req-after-denied", UnityAiBridgeProtocol.CommandPing, discovery.token);
                var authorizedResponse = ParseSingleResponse(TcpRequestHelper.Send(discovery.host, discovery.port, authorizedRequest + "\n"));

                Assert.That(authorizedResponse.ok, Is.True);
                Assert.That(authorizedResponse.requestId, Is.EqualTo("req-after-denied"));
            }
        }

        [Test]
        public void Stop_CanBeCalledMultipleTimesAndDeletesDiscoveryFile()
        {
            var server = StartServer();

            Assert.That(File.Exists(DiscoveryFileCleanup.DiscoveryFilePath), Is.True);

            server.Stop();
            server.Stop();
            server.Dispose();
            server.Dispose();

            Assert.That(File.Exists(DiscoveryFileCleanup.DiscoveryFilePath), Is.False);
            Assert.That(server.IsRunning, Is.False);
        }

        [Test]
        public void HandleRequestLine_AfterStopRejectsMissingRequestToken()
        {
            var server = StartServer();
            server.Stop();

            var request = "{\"schemaVersion\":1,\"requestId\":\"req-after-stop\",\"command\":\"ping\",\"params\":{}}";
            var response = InvokeHandleRequestLine(server, request);

            Assert.That(response.ok, Is.False);
            Assert.That(response.requestId, Is.EqualTo("req-after-stop"));
            Assert.That(response.errorCode, Is.EqualTo(UnityAiBridgeProtocol.ErrorCodeUnauthorized));
        }

        [Test]
        public void RequiresMainThreadDispatch_ConsoleCommandsReturnTrue()
        {
            Assert.That(InvokeRequiresMainThreadDispatch(UnityAiBridgeProtocol.CommandPing), Is.False);
            Assert.That(InvokeRequiresMainThreadDispatch("get_lux_console_logs"), Is.True);
            Assert.That(InvokeRequiresMainThreadDispatch("clear_lux_console"), Is.True);
            Assert.That(InvokeRequiresMainThreadDispatch("simulate_lux_mouse_ui"), Is.True);
        }

        [Test]
        public void Start_WritesLoopbackRandomPortDiscovery()
        {
            using (var server = StartServer())
            {
                var discovery = ReadDiscovery();

                Assert.That(discovery.host, Is.EqualTo(UnityAiBridgeTcpServer.DiscoveryHost));
                Assert.That(IPAddress.IsLoopback(IPAddress.Parse(discovery.host)), Is.True);
                Assert.That(discovery.port, Is.GreaterThan(0));
                Assert.That(discovery.port, Is.EqualTo(server.Port));
                Assert.That(discovery.token, Is.EqualTo(server.Token));
                Assert.That(discovery.token, Is.Not.Empty);
                Assert.That(discovery.protocolVersion, Is.EqualTo(UnityAiBridgeProtocol.ProtocolVersion));
                Assert.That(discovery.projectPath, Is.EqualTo(Directory.GetCurrentDirectory()));
                Assert.That(discovery.pid, Is.GreaterThan(0));
                Assert.That(discovery.startedAtUtc, Is.Not.Empty);
            }
        }

        [Test]
        public void EnsureSharedDiscoverable_RewritesMissingDiscoveryWithoutChangingConnectionMetadata()
        {
            var server = UnityAiBridgeTcpServer.StartShared();
            var discovery = ReadDiscovery();
            File.Delete(DiscoveryFileCleanup.DiscoveryFilePath);

            Assert.That(File.Exists(DiscoveryFileCleanup.DiscoveryFilePath), Is.False);

            var healedServer = UnityAiBridgeTcpServer.EnsureSharedDiscoverable();
            var healedDiscovery = ReadDiscovery();

            Assert.That(healedServer.Port, Is.EqualTo(server.Port));
            Assert.That(healedServer.Token, Is.EqualTo(server.Token));
            Assert.That(healedDiscovery.host, Is.EqualTo(discovery.host));
            Assert.That(healedDiscovery.port, Is.EqualTo(discovery.port));
            Assert.That(healedDiscovery.token, Is.EqualTo(discovery.token));
            Assert.That(healedDiscovery.protocolVersion, Is.EqualTo(discovery.protocolVersion));
        }

        [Test]
        public void EnsureSharedDiscoverable_RewritesStaleDiscoveryWithoutChangingConnectionMetadata()
        {
            var server = UnityAiBridgeTcpServer.StartShared();
            var discovery = ReadDiscovery();
            File.WriteAllText(
                DiscoveryFileCleanup.DiscoveryFilePath,
                JsonUtility.ToJson(
                    new UnityAiBridgeDiscovery
                    {
                        host = UnityAiBridgeTcpServer.DiscoveryHost,
                        port = discovery.port + 1,
                        token = "stale-token",
                        protocolVersion = discovery.protocolVersion,
                        projectPath = discovery.projectPath,
                        pid = discovery.pid + 1,
                        startedAtUtc = discovery.startedAtUtc
                    },
                    true));

            var healedServer = UnityAiBridgeTcpServer.EnsureSharedDiscoverable();
            var healedDiscovery = ReadDiscovery();

            Assert.That(healedServer.Port, Is.EqualTo(server.Port));
            Assert.That(healedServer.Token, Is.EqualTo(server.Token));
            Assert.That(healedDiscovery.host, Is.EqualTo(discovery.host));
            Assert.That(healedDiscovery.port, Is.EqualTo(discovery.port));
            Assert.That(healedDiscovery.token, Is.EqualTo(discovery.token));
            Assert.That(healedDiscovery.pid, Is.EqualTo(discovery.pid));
            Assert.That(healedDiscovery.protocolVersion, Is.EqualTo(discovery.protocolVersion));
            Assert.That(healedDiscovery.projectPath, Is.EqualTo(discovery.projectPath));
        }

        [Test]
        public void StartShared_RewritesStaleDiscoveryWhenServerAlreadyRunning()
        {
            var server = UnityAiBridgeTcpServer.StartShared();
            var discovery = ReadDiscovery();
            File.WriteAllText(
                DiscoveryFileCleanup.DiscoveryFilePath,
                JsonUtility.ToJson(
                    new UnityAiBridgeDiscovery
                    {
                        host = UnityAiBridgeTcpServer.DiscoveryHost,
                        port = discovery.port + 1,
                        token = "stale-token",
                        protocolVersion = discovery.protocolVersion,
                        projectPath = discovery.projectPath,
                        pid = discovery.pid + 1,
                        startedAtUtc = discovery.startedAtUtc
                    },
                    true));

            var healedServer = UnityAiBridgeTcpServer.StartShared();
            var healedDiscovery = ReadDiscovery();

            Assert.That(healedServer.Port, Is.EqualTo(server.Port));
            Assert.That(healedServer.Token, Is.EqualTo(server.Token));
            Assert.That(healedDiscovery.port, Is.EqualTo(discovery.port));
            Assert.That(healedDiscovery.token, Is.EqualTo(discovery.token));
            Assert.That(healedDiscovery.pid, Is.EqualTo(discovery.pid));
        }

        private static UnityAiBridgeTcpServer StartServer()
        {
            var server = new UnityAiBridgeTcpServer();
            server.Start();
            return server;
        }

        private static UnityAiBridgeDiscovery ReadDiscovery()
        {
            Assert.That(File.Exists(DiscoveryFileCleanup.DiscoveryFilePath), Is.True);
            return JsonUtility.FromJson<UnityAiBridgeDiscovery>(File.ReadAllText(DiscoveryFileCleanup.DiscoveryFilePath));
        }

        private static UnityAiBridgeProtocolResponse InvokeHandleRequestLine(UnityAiBridgeTcpServer server, string request)
        {
            var handleRequestLine = typeof(UnityAiBridgeTcpServer).GetMethod("HandleRequestLine", BindingFlags.Instance | BindingFlags.NonPublic);

            Assert.That(handleRequestLine, Is.Not.Null);
            return (UnityAiBridgeProtocolResponse)handleRequestLine.Invoke(server, new object[] { request });
        }

        private static bool InvokeRequiresMainThreadDispatch(string command)
        {
            var method = typeof(UnityAiBridgeTcpServer).GetMethod("RequiresMainThreadDispatch", BindingFlags.Static | BindingFlags.NonPublic);

            Assert.That(method, Is.Not.Null);
            return (bool)method.Invoke(null, new object[] { new UnityAiBridgeProtocolRequest { command = command } });
        }

        private static string CreateRequestJson(string requestId, string command, string token)
        {
            return JsonUtility.ToJson(new UnityAiBridgeProtocolRequest
            {
                schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                requestId = requestId,
                command = command,
                token = token,
                @params = new UnityAiBridgeProtocolRequestParameters()
            });
        }

        private static UnityAiBridgeProtocolResponse ParseSingleResponse(string responseText)
        {
            var lines = responseText.Split('\n');
            var responseLineCount = 0;
            var responseLine = string.Empty;

            for (var i = 0; i < lines.Length; i++)
            {
                var line = lines[i].TrimEnd('\r');
                if (line.Length == 0)
                {
                    continue;
                }

                responseLineCount++;
                responseLine = line;
            }

            Assert.That(responseLineCount, Is.EqualTo(1), responseText);
            return JsonUtility.FromJson<UnityAiBridgeProtocolResponse>(responseLine);
        }
    }
}
