using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using Debug = UnityEngine.Debug;
using System.IO;
using System.Net.Sockets;
using System.Net.WebSockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Linalab.UnityAiBridge.Editor;
using UnityEditor;
using UnityEngine;

namespace Linalab.LuxEditor
{
    public sealed class LuxAIToolDispatcher : IDisposable
    {
        private const string Source = "lux-ai-dispatcher";
        private static readonly TimeSpan DefaultToolTimeout = TimeSpan.FromSeconds(60);

        private readonly Func<ILuxAIToolDispatcherWebSocketClient> socketFactory;
        private readonly IToolProcessRunner processRunner;
        private readonly ILuxAiBridgeClient aiBridgeClient;
        private readonly object sessionsLock = new object();
        private readonly Dictionary<string, ToolSessionInfo> activeSessions = new Dictionary<string, ToolSessionInfo>(StringComparer.Ordinal);

        private ILuxAIToolDispatcherWebSocketClient socket;
        private CancellationTokenSource cancellation;
        private Task receiveLoop;
        private string sessionId = "unity-editor";

        public LuxAIToolDispatcher()
            : this(() => new LuxAIToolDispatcherClientWebSocketTransport(), new DefaultToolProcessRunner(), new LuxAiBridgeTcpClient())
        {
        }

        public LuxAIToolDispatcher(
            Func<ILuxAIToolDispatcherWebSocketClient> socketFactory,
            IToolProcessRunner processRunner = null,
            ILuxAiBridgeClient aiBridgeClient = null)
        {
            this.socketFactory = socketFactory ?? throw new ArgumentNullException(nameof(socketFactory));
            this.processRunner = processRunner ?? new DefaultToolProcessRunner();
            this.aiBridgeClient = aiBridgeClient ?? new LuxAiBridgeTcpClient();
        }

        public IReadOnlyDictionary<string, ToolSessionInfo> ActiveSessions
        {
            get
            {
                lock (sessionsLock)
                {
                    return new Dictionary<string, ToolSessionInfo>(activeSessions, StringComparer.Ordinal);
                }
            }
        }

        public event Action<ToolExecutionResult> OnExecutionComplete;
        public event Action<ToolExecutionProgress> OnProgress;
        public event Action<string> OnError;

        public void Connect(string gatewayUrl, string token)
        {
            if (string.IsNullOrWhiteSpace(gatewayUrl))
            {
                Warn("Lux AI tool dispatcher gateway URL is not configured.");
                return;
            }

            Disconnect();
            cancellation = new CancellationTokenSource();
            socket = socketFactory();
            receiveLoop = ConnectAndReceiveAsync(gatewayUrl, token, cancellation.Token);
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

            receiveLoop = null;
            lock (sessionsLock)
            {
                activeSessions.Clear();
            }
        }

        public void Dispose()
        {
            Disconnect();
        }

        public async Task<ToolExecutionResult> HandleEventJsonAsync(string eventJson, CancellationToken cancellationToken)
        {
            if (!TryDeserializeDispatchRequest(eventJson, out var request, out var error))
            {
                if (!string.IsNullOrEmpty(error))
                {
                    await ReportErrorAsync(string.Empty, string.Empty, string.Empty, error, cancellationToken);
                }

                return null;
            }

            sessionId = string.IsNullOrWhiteSpace(request.SessionId) ? sessionId : request.SessionId;
            if (string.Equals(request.Kind, "tool-execute", StringComparison.Ordinal))
            {
                return await ExecuteToolAsync(request, cancellationToken);
            }

            if (string.Equals(request.Kind, "skill-dispatch", StringComparison.Ordinal))
            {
                return await DispatchSkillAsync(request, cancellationToken);
            }

            return null;
        }

        public static bool TryDeserializeDispatchRequest(string eventJson, out LuxAIToolDispatchRequest request, out string error)
        {
            request = null;
            error = string.Empty;

            if (string.IsNullOrWhiteSpace(eventJson))
            {
                return false;
            }

            if (!string.Equals(ExtractString(eventJson, "category"), "tool", StringComparison.Ordinal))
            {
                return false;
            }

            var payloadJson = ExtractJsonValue(eventJson, "payload");
            if (string.IsNullOrWhiteSpace(payloadJson))
            {
                return false;
            }

            var kind = ExtractString(payloadJson, "kind");
            if (!string.Equals(kind, "tool-execute", StringComparison.Ordinal) && !string.Equals(kind, "skill-dispatch", StringComparison.Ordinal))
            {
                return false;
            }

            request = new LuxAIToolDispatchRequest
            {
                Kind = kind,
                SessionId = ExtractString(eventJson, "session_id"),
                ExecutionId = ExtractString(payloadJson, "executionId"),
                ToolType = ExtractString(payloadJson, "toolType"),
                Command = ExtractString(payloadJson, "command"),
                SkillName = ExtractString(payloadJson, "skillName"),
                SkillParamsJson = ExtractJsonValue(payloadJson, "skillParams")
            };

            if (string.IsNullOrWhiteSpace(request.ExecutionId))
            {
                request.ExecutionId = Guid.NewGuid().ToString("N");
            }

            return true;
        }

        public static bool TryResolveToolProcess(string toolType, string command, out string executable, out string arguments, out string error)
        {
            executable = string.Empty;
            arguments = string.Empty;
            error = string.Empty;

            if (string.IsNullOrWhiteSpace(command))
            {
                error = "Tool command is required.";
                return false;
            }

            if (string.Equals(toolType, "claude-code", StringComparison.Ordinal))
            {
                executable = "claude";
                arguments = command;
                return true;
            }

            if (string.Equals(toolType, "openai-codex", StringComparison.Ordinal))
            {
                executable = "codex";
                arguments = $"exec {Quote(command)} -s workspace-write --skip-git-repo-check";
                return true;
            }

            if (string.Equals(toolType, "opencode", StringComparison.Ordinal))
            {
                executable = "opencode";
                arguments = command;
                return true;
            }

            error = string.IsNullOrWhiteSpace(toolType) ? "Tool type is required." : "Unknown AI tool type: " + toolType;
            return false;
        }

        public static bool TryMapSkillToAiBridgeCommand(string skillName, out string aiBridgeCommand, out string error)
        {
            aiBridgeCommand = string.Empty;
            error = string.Empty;

            if (string.Equals(skillName, "compile", StringComparison.Ordinal))
            {
                aiBridgeCommand = UnityAiBridgeProtocol.CommandTriggerCompile;
                return true;
            }

            if (string.Equals(skillName, "test", StringComparison.Ordinal))
            {
                aiBridgeCommand = "execute_lux_shell";
                return true;
            }

            if (string.Equals(skillName, "screenshot", StringComparison.Ordinal))
            {
                aiBridgeCommand = "capture_lux_screenshot";
                return true;
            }

            if (string.Equals(skillName, "logs", StringComparison.Ordinal))
            {
                aiBridgeCommand = "get_lux_console_logs";
                return true;
            }

            if (string.Equals(skillName, "playmode", StringComparison.Ordinal))
            {
                aiBridgeCommand = "control_lux_play_mode";
                return true;
            }

            if (string.Equals(skillName, "dynamic-code", StringComparison.Ordinal))
            {
                aiBridgeCommand = "execute_lux_dynamic_code";
                return true;
            }

            error = string.IsNullOrWhiteSpace(skillName) ? "Skill name is required." : "Unknown Lux skill: " + skillName;
            return false;
        }

        private async Task ConnectAndReceiveAsync(string gatewayUrl, string token, CancellationToken cancellationToken)
        {
            try
            {
                var eventsUri = BuildEventsUri(gatewayUrl);
                await socket.ConnectAsync(eventsUri, token, cancellationToken);
                while (!cancellationToken.IsCancellationRequested && socket.IsConnected)
                {
                    var message = await socket.ReceiveTextAsync(cancellationToken);
                    if (message == null)
                    {
                        break;
                    }

                    _ = HandleEventJsonAsync(message, cancellationToken);
                }
            }
            catch (OperationCanceledException)
            {
            }
            catch (Exception exception) when (exception is WebSocketException || exception is IOException || exception is InvalidOperationException || exception is UriFormatException || exception is ObjectDisposedException)
            {
                Warn("Lux AI tool dispatcher could not connect to gateway: " + exception.Message);
                OnError?.Invoke(exception.Message);
            }
        }

        private async Task<ToolExecutionResult> ExecuteToolAsync(LuxAIToolDispatchRequest request, CancellationToken cancellationToken)
        {
            if (!TryResolveToolProcess(request.ToolType, request.Command, out var executable, out var arguments, out var routeError))
            {
                return await CompleteWithErrorAsync(request, routeError, cancellationToken);
            }

            AddSession(request.ExecutionId, request.ToolType);
            try
            {
                var progress = new Progress<string>(message =>
                {
                    var progressEvent = new ToolExecutionProgress
                    {
                        ExecutionId = request.ExecutionId,
                        ToolType = request.ToolType,
                        Message = message,
                        Progress = 0f
                    };
                    OnProgress?.Invoke(progressEvent);
                    _ = SendProgressAsync(request.SessionId, progressEvent, CancellationToken.None);
                });

                var processResult = await processRunner.RunAsync(executable, arguments, DefaultToolTimeout, cancellationToken, progress);
                var result = new ToolExecutionResult
                {
                    ExecutionId = request.ExecutionId,
                    ToolType = request.ToolType,
                    Succeeded = processResult.Succeeded,
                    Output = processResult.StandardOutput,
                    Error = processResult.StandardError
                };
                OnExecutionComplete?.Invoke(result);
                await SendResultAsync(request.SessionId, result, cancellationToken);
                return result;
            }
            catch (OperationCanceledException)
            {
                return await CompleteWithErrorAsync(request, "Tool execution was cancelled or timed out.", CancellationToken.None);
            }
            catch (Exception exception) when (exception is Win32Exception || exception is InvalidOperationException)
            {
                return await CompleteWithErrorAsync(request, $"Failed to start '{executable}'. Ensure it is installed and available on PATH. {exception.Message}", cancellationToken);
            }
            finally
            {
                RemoveSession(request.ExecutionId);
            }
        }

        private async Task<ToolExecutionResult> DispatchSkillAsync(LuxAIToolDispatchRequest request, CancellationToken cancellationToken)
        {
            if (!TryMapSkillToAiBridgeCommand(request.SkillName, out var aiBridgeCommand, out var skillError))
            {
                return await CompleteWithErrorAsync(request, skillError, cancellationToken);
            }

            AddSession(request.ExecutionId, request.ToolType);
            try
            {
                var progressEvent = new ToolExecutionProgress
                {
                    ExecutionId = request.ExecutionId,
                    ToolType = request.ToolType,
                    Message = "Dispatching Lux skill '" + request.SkillName + "' to AI Bridge command '" + aiBridgeCommand + "'.",
                    Progress = 0.25f
                };
                OnProgress?.Invoke(progressEvent);
                await SendProgressAsync(request.SessionId, progressEvent, cancellationToken);

                var response = await aiBridgeClient.SendCommandAsync(aiBridgeCommand, request.SkillParamsJson, cancellationToken);
                var result = new ToolExecutionResult
                {
                    ExecutionId = request.ExecutionId,
                    ToolType = request.ToolType,
                    Succeeded = !string.IsNullOrEmpty(response) && response.IndexOf("\"ok\":true", StringComparison.Ordinal) >= 0,
                    Output = response,
                    Error = string.Empty
                };
                if (!result.Succeeded)
                {
                    result.Error = string.IsNullOrWhiteSpace(response) ? "AI Bridge returned an empty response." : response;
                }

                OnExecutionComplete?.Invoke(result);
                await SendResultAsync(request.SessionId, result, cancellationToken);
                return result;
            }
            catch (Exception exception) when (exception is IOException || exception is SocketException || exception is InvalidOperationException || exception is ObjectDisposedException)
            {
                return await CompleteWithErrorAsync(request, "AI Bridge skill dispatch failed: " + exception.Message, cancellationToken);
            }
            finally
            {
                RemoveSession(request.ExecutionId);
            }
        }

        private async Task<ToolExecutionResult> CompleteWithErrorAsync(LuxAIToolDispatchRequest request, string message, CancellationToken cancellationToken)
        {
            var result = new ToolExecutionResult
            {
                ExecutionId = request == null ? string.Empty : request.ExecutionId,
                ToolType = request == null ? string.Empty : request.ToolType,
                Succeeded = false,
                Output = string.Empty,
                Error = message ?? string.Empty
            };
            OnError?.Invoke(result.Error);
            OnExecutionComplete?.Invoke(result);
            Debug.LogError(result.Error);
            await SendResultAsync(request == null ? string.Empty : request.SessionId, result, cancellationToken);
            return result;
        }

        private async Task SendProgressAsync(string eventSessionId, ToolExecutionProgress progress, CancellationToken cancellationToken)
        {
            if (socket == null || !socket.IsConnected)
            {
                return;
            }

            await socket.SendTextAsync(CreateProgressEnvelopeJson(ResolveSessionId(eventSessionId), progress), cancellationToken);
        }

        private async Task SendResultAsync(string eventSessionId, ToolExecutionResult result, CancellationToken cancellationToken)
        {
            if (socket == null || !socket.IsConnected)
            {
                return;
            }

            await socket.SendTextAsync(CreateResultEnvelopeJson(ResolveSessionId(eventSessionId), result), cancellationToken);
        }

        private async Task ReportErrorAsync(string eventSessionId, string executionId, string toolType, string message, CancellationToken cancellationToken)
        {
            await SendResultAsync(eventSessionId, new ToolExecutionResult
            {
                ExecutionId = executionId,
                ToolType = toolType,
                Succeeded = false,
                Output = string.Empty,
                Error = message
            }, cancellationToken);
            OnError?.Invoke(message);
        }

        private void AddSession(string executionId, string toolType)
        {
            lock (sessionsLock)
            {
                activeSessions[executionId] = new ToolSessionInfo
                {
                    SessionId = executionId,
                    ToolType = toolType,
                    ConnectedAt = DateTime.UtcNow,
                    IsConnected = true
                };
            }
        }

        private void RemoveSession(string executionId)
        {
            lock (sessionsLock)
            {
                activeSessions.Remove(executionId);
            }
        }

        private string ResolveSessionId(string eventSessionId)
        {
            return string.IsNullOrWhiteSpace(eventSessionId) ? sessionId : eventSessionId;
        }

        private static Uri BuildEventsUri(string gatewayUrl)
        {
            var builder = new UriBuilder(gatewayUrl);
            if (string.IsNullOrEmpty(builder.Path) || builder.Path == "/")
            {
                builder.Path = "/events";
            }

            if (string.IsNullOrEmpty(builder.Query))
            {
                builder.Query = "role=unity&client_id=lux-ai-dispatcher";
            }

            return builder.Uri;
        }

        private static string CreateProgressEnvelopeJson(string envelopeSessionId, ToolExecutionProgress progress)
        {
            return CreateEnvelopeJson(envelopeSessionId, "tool-execution-progress", progress.ExecutionId, progress.ToolType, false, progress.Message, string.Empty, progress.Progress);
        }

        private static string CreateResultEnvelopeJson(string envelopeSessionId, ToolExecutionResult result)
        {
            return CreateEnvelopeJson(envelopeSessionId, "tool-execution-result", result.ExecutionId, result.ToolType, result.Succeeded, result.Output, result.Error, 1f);
        }

        private static string CreateEnvelopeJson(string envelopeSessionId, string kind, string executionId, string toolType, bool succeeded, string outputOrMessage, string error, float progress)
        {
            var envelope = new LuxAIToolDispatcherEnvelope
            {
                schema_version = 1,
                event_id = Guid.NewGuid().ToString("N"),
                category = "tool",
                source = Source,
                session_id = string.IsNullOrWhiteSpace(envelopeSessionId) ? "unity-editor" : envelopeSessionId,
                captured_at_utc = DateTime.UtcNow.ToString("O"),
                payload = new LuxAIToolDispatcherPayload
                {
                    kind = kind,
                    executionId = executionId ?? string.Empty,
                    toolType = toolType ?? string.Empty,
                    succeeded = succeeded,
                    output = string.Equals(kind, "tool-execution-result", StringComparison.Ordinal) ? outputOrMessage ?? string.Empty : string.Empty,
                    error = error,
                    message = string.Equals(kind, "tool-execution-progress", StringComparison.Ordinal) ? outputOrMessage ?? string.Empty : string.Empty,
                    progress = progress
                }
            };
            return JsonUtility.ToJson(envelope, false);
        }

        private static string ExtractString(string json, string fieldName)
        {
            var value = ExtractJsonValue(json, fieldName);
            return string.IsNullOrEmpty(value) || value.Length < 2 || value[0] != '"' ? string.Empty : UnescapeJsonString(value.Substring(1, value.Length - 2));
        }

        private static string ExtractJsonValue(string json, string fieldName)
        {
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

        private static string UnescapeJsonString(string value)
        {
            return value.Replace("\\\"", "\"").Replace("\\\\", "\\");
        }

        private static string Quote(string value)
        {
            return "\"" + (value ?? string.Empty).Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";
        }

        private static void Warn(string message)
        {
            Debug.LogWarning(message);
        }

        [Serializable]
        private sealed class LuxAIToolDispatcherEnvelope
        {
            public int schema_version;
            public string event_id;
            public string category;
            public string source;
            public string session_id;
            public string captured_at_utc;
            public LuxAIToolDispatcherPayload payload;
        }

        [Serializable]
        private sealed class LuxAIToolDispatcherPayload
        {
            public string kind;
            public string executionId;
            public string toolType;
            public bool succeeded;
            public string output;
            public string error;
            public string message;
            public float progress;
        }
    }

    public sealed class ToolSessionInfo
    {
        public string SessionId { get; set; }
        public string ToolType { get; set; }
        public DateTime ConnectedAt { get; set; }
        public bool IsConnected { get; set; }
    }

    public sealed class ToolExecutionResult
    {
        public string ExecutionId { get; set; }
        public string ToolType { get; set; }
        public bool Succeeded { get; set; }
        public string Output { get; set; }
        public string Error { get; set; }
    }

    public sealed class ToolExecutionProgress
    {
        public string ExecutionId { get; set; }
        public string ToolType { get; set; }
        public string Message { get; set; }
        public float Progress { get; set; }
    }

    public sealed class LuxAIToolDispatchRequest
    {
        public string Kind { get; set; }
        public string SessionId { get; set; }
        public string ExecutionId { get; set; }
        public string ToolType { get; set; }
        public string Command { get; set; }
        public string SkillName { get; set; }
        public string SkillParamsJson { get; set; }
    }

    public interface IToolProcessRunner
    {
        Task<ToolProcessResult> RunAsync(string executable, string arguments, TimeSpan timeout, CancellationToken cancellationToken, IProgress<string> onOutput = null);
    }

    internal sealed class DefaultToolProcessRunner : IToolProcessRunner
    {
        public Task<ToolProcessResult> RunAsync(string executable, string arguments, TimeSpan timeout, CancellationToken cancellationToken, IProgress<string> onOutput = null)
        {
            return ToolProcessRunner.RunAsync(executable, arguments, cancellationToken, onOutput, timeout);
        }
    }

    internal sealed class ToolProcessRunner
    {
        public static async Task<ToolProcessResult> RunAsync(
            string executable,
            string arguments,
            CancellationToken cancellationToken,
            IProgress<string> onOutput = null)
        {
            return await RunAsync(executable, arguments, cancellationToken, onOutput, TimeSpan.FromSeconds(60));
        }

        public static async Task<ToolProcessResult> RunAsync(
            string executable,
            string arguments,
            CancellationToken cancellationToken,
            IProgress<string> onOutput,
            TimeSpan timeout)
        {
            var result = new ToolProcessResult();
            var stdout = new StringBuilder();
            var stderr = new StringBuilder();
            using (var timeoutSource = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken))
            using (var process = new Process())
            {
                timeoutSource.CancelAfter(timeout <= TimeSpan.Zero ? TimeSpan.FromSeconds(60) : timeout);
                process.StartInfo = new ProcessStartInfo
                {
                    FileName = executable,
                    Arguments = arguments ?? string.Empty,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    WorkingDirectory = Directory.GetCurrentDirectory()
                };

                process.OutputDataReceived += (sender, args) =>
                {
                    if (args.Data == null)
                    {
                        return;
                    }

                    stdout.AppendLine(args.Data);
                    onOutput?.Report(args.Data);
                };
                process.ErrorDataReceived += (sender, args) =>
                {
                    if (args.Data == null)
                    {
                        return;
                    }

                    stderr.AppendLine(args.Data);
                    onOutput?.Report(args.Data);
                };

                process.Start();
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();

                while (!process.HasExited)
                {
                    await Task.Delay(50, timeoutSource.Token);
                }

                result.ExitCode = process.ExitCode;
            }

            result.StandardOutput = stdout.ToString();
            result.StandardError = stderr.ToString();
            return result;
        }
    }

    public sealed class ToolProcessResult
    {
        public int ExitCode { get; set; }
        public string StandardOutput { get; set; }
        public string StandardError { get; set; }
        public bool Succeeded => ExitCode == 0;
    }

    public interface ILuxAiBridgeClient
    {
        Task<string> SendCommandAsync(string command, string skillParamsJson, CancellationToken cancellationToken);
    }

    internal sealed class LuxAiBridgeTcpClient : ILuxAiBridgeClient
    {
        public async Task<string> SendCommandAsync(string command, string skillParamsJson, CancellationToken cancellationToken)
        {
            var discoveryPath = UnityAiBridgeTcpServer.GetDiscoveryFilePath();
            var discovery = File.Exists(discoveryPath) ? JsonUtility.FromJson<UnityAiBridgeDiscovery>(File.ReadAllText(discoveryPath)) : null;
            if (discovery == null || discovery.port <= 0)
            {
                throw new InvalidOperationException("Unity AI Bridge TCP server is not discoverable. Start Tools > Linalab > Lux > AI Bridge first.");
            }

            using (var client = new TcpClient())
            {
                await client.ConnectAsync(discovery.host, discovery.port);
                cancellationToken.ThrowIfCancellationRequested();
                using (var stream = client.GetStream())
                using (var writer = new StreamWriter(stream, new UTF8Encoding(false)) { AutoFlush = true })
                using (var reader = new StreamReader(stream, Encoding.UTF8))
                {
                    await writer.WriteLineAsync(CreateAiBridgeRequestJson(command, discovery.token, skillParamsJson));
                    cancellationToken.ThrowIfCancellationRequested();
                    return await reader.ReadLineAsync();
                }
            }
        }

        private static string CreateAiBridgeRequestJson(string command, string token, string skillParamsJson)
        {
            var builder = new StringBuilder(256);
            builder.Append('{');
            AppendJsonProperty(builder, "schemaVersion", UnityAiBridgeProtocol.SchemaVersion, false);
            AppendJsonProperty(builder, "requestId", Guid.NewGuid().ToString("N"), true);
            AppendJsonProperty(builder, "command", command, true);
            AppendJsonProperty(builder, "token", token, true);
            builder.Append(",\"params\":");
            builder.Append(string.IsNullOrWhiteSpace(skillParamsJson) ? "{}" : skillParamsJson);
            builder.Append('}');
            return builder.ToString();
        }

        private static void AppendJsonProperty(StringBuilder builder, string name, string value, bool prefixComma)
        {
            if (prefixComma)
            {
                builder.Append(',');
            }

            builder.Append('"').Append(name).Append("\":");
            AppendJsonString(builder, value ?? string.Empty);
        }

        private static void AppendJsonProperty(StringBuilder builder, string name, int value, bool prefixComma)
        {
            if (prefixComma)
            {
                builder.Append(',');
            }

            builder.Append('"').Append(name).Append("\":").Append(value);
        }

        private static void AppendJsonString(StringBuilder builder, string value)
        {
            builder.Append('"');
            foreach (var character in value ?? string.Empty)
            {
                switch (character)
                {
                    case '\\':
                        builder.Append("\\\\");
                        break;
                    case '"':
                        builder.Append("\\\"");
                        break;
                    case '\n':
                        builder.Append("\\n");
                        break;
                    case '\r':
                        builder.Append("\\r");
                        break;
                    case '\t':
                        builder.Append("\\t");
                        break;
                    default:
                        builder.Append(character);
                        break;
                }
            }

            builder.Append('"');
        }
    }

    public interface ILuxAIToolDispatcherWebSocketClient : IDisposable
    {
        bool IsConnected { get; }
        Task ConnectAsync(Uri uri, string token, CancellationToken cancellationToken);
        Task<string> ReceiveTextAsync(CancellationToken cancellationToken);
        Task SendTextAsync(string message, CancellationToken cancellationToken);
    }

    public sealed class LuxAIToolDispatcherClientWebSocketTransport : ILuxAIToolDispatcherWebSocketClient
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

    public static class LuxAIToolDispatcherIntegration
    {
        private static LuxAIToolDispatcher dispatcher;

        [MenuItem("Tools/Linalab/Lux/AI Tool Dispatcher/Connect")]
        public static void Connect()
        {
            var gatewayUrl = Environment.GetEnvironmentVariable("LUX_GATEWAY_URL");
            var token = Environment.GetEnvironmentVariable("LUX_GATEWAY_TOKEN");
            if (string.IsNullOrWhiteSpace(gatewayUrl))
            {
                Debug.LogWarning("Set LUX_GATEWAY_URL before connecting the Lux AI tool dispatcher.");
                return;
            }

            dispatcher = dispatcher ?? new LuxAIToolDispatcher();
            dispatcher.Connect(gatewayUrl, token);
            Debug.Log("Lux AI tool dispatcher connecting to configured gateway.");
        }

        [MenuItem("Tools/Linalab/Lux/AI Tool Dispatcher/Disconnect")]
        public static void Disconnect()
        {
            dispatcher?.Disconnect();
            dispatcher = null;
            Debug.Log("Lux AI tool dispatcher disconnected.");
        }

        [MenuItem("Tools/Linalab/Lux/AI Tool Dispatcher/Status")]
        public static void ShowStatus()
        {
            var activeCount = dispatcher == null ? 0 : dispatcher.ActiveSessions.Count;
            Debug.Log("Lux AI tool dispatcher status: " + (dispatcher == null ? "disconnected" : "connected") + ", active sessions: " + activeCount);
        }
    }
}
