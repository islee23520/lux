using System.IO;
using NUnit.Framework;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    public sealed class UnityAiBridgeProtocolTests
    {
        [Test]
        public void ProtocolRequest_RoundTripsThroughJsonUtility()
        {
            var request = new UnityAiBridgeProtocolRequest
            {
                schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                requestId = "req-1",
                command = UnityAiBridgeProtocol.CommandPing,
                token = "token-123",
                @params = new UnityAiBridgeProtocolRequestParameters()
            };

            var json = JsonUtility.ToJson(request);
            var loaded = JsonUtility.FromJson<UnityAiBridgeProtocolRequest>(json);

            Assert.That(loaded.schemaVersion, Is.EqualTo(UnityAiBridgeProtocol.SchemaVersion));
            Assert.That(loaded.requestId, Is.EqualTo("req-1"));
            Assert.That(loaded.command, Is.EqualTo(UnityAiBridgeProtocol.CommandPing));
            Assert.That(loaded.token, Is.EqualTo("token-123"));
            Assert.That(loaded.@params, Is.Not.Null);
        }

        [Test]
        public void ProtocolResponse_RoundTripsThroughJsonUtility()
        {
            var response = UnityAiBridgeProtocol.Handle(
                new UnityAiBridgeProtocolRequest
                {
                    schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                    requestId = "req-2",
                    command = UnityAiBridgeProtocol.CommandPing,
                    token = "token-123",
                    @params = new UnityAiBridgeProtocolRequestParameters()
                },
                "token-123");

            var json = JsonUtility.ToJson(response);
            var loaded = JsonUtility.FromJson<UnityAiBridgeProtocolResponse>(json);

            Assert.That(loaded.ok, Is.True);
            Assert.That(loaded.requestId, Is.EqualTo("req-2"));
            Assert.That(loaded.payload, Is.Not.Null);
            Assert.That(loaded.payload.ping, Is.Not.Null);
            Assert.That(loaded.payload.ping.status, Is.EqualTo("ok"));
            Assert.That(loaded.capturedAtUtc, Is.Not.Empty);
        }

        [Test]
        public void Handle_GetProtocolInfo_ReturnsCoreCommands()
        {
            var response = UnityAiBridgeProtocol.Handle(
                new UnityAiBridgeProtocolRequest
                {
                    schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                    requestId = "req-3",
                    command = UnityAiBridgeProtocol.CommandGetProtocolInfo,
                    token = "token-123",
                    @params = new UnityAiBridgeProtocolRequestParameters()
                },
                "token-123");

            Assert.That(response.ok, Is.True);
            Assert.That(response.payload.protocolInfo.protocolVersion, Is.EqualTo(UnityAiBridgeProtocol.ProtocolVersion));
            Assert.That(response.payload.protocolInfo.backendVersion, Is.EqualTo(UnityAiBridgeProtocol.BackendVersion));
            Assert.That(response.payload.protocolInfo.commands, Does.Contain(UnityAiBridgeProtocol.CommandPing));
            Assert.That(response.payload.protocolInfo.commands, Does.Contain(UnityAiBridgeProtocol.CommandGetProtocolInfo));
            Assert.That(response.payload.protocolInfo.commands, Does.Contain(UnityAiBridgeProtocol.CommandGetSelectedFileContext));
            Assert.That(response.payload.protocolInfo.commands.Length, Is.GreaterThanOrEqualTo(UnityAiBridgeProtocol.SupportedCommands.Length));
        }

        [Test]
        public void RegisterCommand_AddsCommandToProtocolInfoAndHandlerDispatch()
        {
            const string command = "test_registered_command";
            UnityAiBridgeProtocol.UnregisterCommand(command);

            try
            {
                UnityAiBridgeProtocol.RegisterCommand(command, request => UnityAiBridgeProtocol.CreateOkResponse(
                    request.requestId,
                    new UnityAiBridgeProtocolResponsePayload
                    {
                        ping = new UnityAiBridgePingPayload
                        {
                            status = "registered"
                        }
                    }));

                var protocolInfo = UnityAiBridgeProtocol.Handle(
                    new UnityAiBridgeProtocolRequest
                    {
                        schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                        requestId = "req-registered-info",
                        command = UnityAiBridgeProtocol.CommandGetProtocolInfo,
                        token = "token-123",
                        @params = new UnityAiBridgeProtocolRequestParameters()
                    },
                    "token-123");

                var response = UnityAiBridgeProtocol.Handle(
                    new UnityAiBridgeProtocolRequest
                    {
                        schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                        requestId = "req-registered-dispatch",
                        command = command,
                        token = "token-123",
                        @params = new UnityAiBridgeProtocolRequestParameters()
                    },
                    "token-123");

                Assert.That(protocolInfo.payload.protocolInfo.commands, Does.Contain(command));
                Assert.That(response.ok, Is.True);
                Assert.That(response.payload.ping.status, Is.EqualTo("registered"));
            }
            finally
            {
                UnityAiBridgeProtocol.UnregisterCommand(command);
            }
        }

        [Test]
        public void Handle_GetSelectedFileContext_EmptySelectionReturnsSuccessfulPayload()
        {
            var previousSelection = Selection.objects;

            try
            {
                Selection.objects = new Object[0];
                UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();

                var response = UnityAiBridgeProtocol.Handle(
                    new UnityAiBridgeProtocolRequest
                    {
                        schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                        requestId = "req-selected-empty",
                        command = UnityAiBridgeProtocol.CommandGetSelectedFileContext,
                        token = "token-123",
                        @params = new UnityAiBridgeProtocolRequestParameters()
                    },
                    "token-123");

                var payload = response.payload.selectedFileContext;

                Assert.That(response.ok, Is.True);
                Assert.That(payload, Is.Not.Null);
                Assert.That(payload.projectName, Is.EqualTo(Application.productName));
                Assert.That(payload.projectPath, Is.EqualTo(Directory.GetCurrentDirectory()));
                Assert.That(payload.unityVersion, Is.EqualTo(Application.unityVersion));
                Assert.That(payload.selectionCount, Is.EqualTo(0));
                Assert.That(payload.selectedFiles, Is.Not.Null);
                Assert.That(payload.selectedFiles, Is.Empty);
            }
            finally
            {
                Selection.objects = previousSelection;
                UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();
            }
        }

        [Test]
        public void Handle_UnknownCommand_ReturnsUnknownCommandError()
        {
            var response = UnityAiBridgeProtocol.Handle(
                new UnityAiBridgeProtocolRequest
                {
                    schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                    requestId = "req-4",
                    command = "delete_file",
                    token = "token-123",
                    @params = new UnityAiBridgeProtocolRequestParameters()
                },
                "token-123");

            Assert.That(response.ok, Is.False);
            Assert.That(response.errorCode, Is.EqualTo(UnityAiBridgeProtocol.ErrorCodeUnknownCommand));
            Assert.That(response.errorMessage, Does.Contain("delete_file"));
        }

        [Test]
        public void Handle_InvalidToken_ReturnsUnauthorizedError()
        {
            var response = UnityAiBridgeProtocol.Handle(
                new UnityAiBridgeProtocolRequest
                {
                    schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                    requestId = "req-5",
                    command = UnityAiBridgeProtocol.CommandPing,
                    token = "wrong-token",
                    @params = new UnityAiBridgeProtocolRequestParameters()
                },
                "token-123");

            Assert.That(response.ok, Is.False);
            Assert.That(response.errorCode, Is.EqualTo(UnityAiBridgeProtocol.ErrorCodeUnauthorized));
            Assert.That(response.errorMessage, Is.EqualTo("Invalid token."));
        }

        [Test]
        public void Handle_NullOrEmptyTokens_ReturnUnauthorizedError()
        {
            AssertUnauthorized(null, null);
            AssertUnauthorized(string.Empty, null);
            AssertUnauthorized(null, "token-123");
            AssertUnauthorized(string.Empty, "token-123");
            AssertUnauthorized("token-123", null);
            AssertUnauthorized("token-123", string.Empty);
        }

        private static void AssertUnauthorized(string requestToken, string expectedToken)
        {
            var response = UnityAiBridgeProtocol.Handle(
                new UnityAiBridgeProtocolRequest
                {
                    schemaVersion = UnityAiBridgeProtocol.SchemaVersion,
                    requestId = "req-null-token",
                    command = UnityAiBridgeProtocol.CommandPing,
                    token = requestToken,
                    @params = new UnityAiBridgeProtocolRequestParameters()
                },
                expectedToken);

            Assert.That(response.ok, Is.False);
            Assert.That(response.errorCode, Is.EqualTo(UnityAiBridgeProtocol.ErrorCodeUnauthorized));
        }
    }
}
