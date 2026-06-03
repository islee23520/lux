using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    public sealed class UnityAiBridgeTcpServer : IDisposable
    {
        public const string DiscoveryHost = "127.0.0.1";

        private static readonly object SharedLifecycleLock = new object();
        private static readonly int MainThreadId = Thread.CurrentThread.ManagedThreadId;
        private static readonly ConcurrentQueue<Action> MainThreadActionQueue = new ConcurrentQueue<Action>();
        private static readonly ConcurrentQueue<MainThreadDispatch> MainThreadDispatchQueue = new ConcurrentQueue<MainThreadDispatch>();
        private const string DynamicCodeCommand = "execute_lux_dynamic_code";
        private static int dynamicCodeInFlight;
        private static UnityAiBridgeTcpServer sharedInstance;

        private readonly object lifecycleLock = new object();
        private readonly object clientsLock = new object();
        private readonly List<ClientConnection> connectedClients = new List<ClientConnection>();
        private TcpListener listener;
        private CancellationTokenSource cancellationTokenSource;
        private Task acceptTask;
        private string token;
        private string discoveryFilePath;
        private bool disposed;

        static UnityAiBridgeTcpServer()
        {
            AssemblyReloadEvents.beforeAssemblyReload += StopSharedInstance;
            EditorApplication.quitting += StopSharedInstance;
            EditorApplication.playModeStateChanged += HandlePlayModeStateChanged;
            EditorApplication.update += ProcessMainThreadDispatchQueue;
        }

        public int Port { get; private set; }

        public string Token => token;

        public string DiscoveryFilePath => discoveryFilePath ?? GetDefaultDiscoveryFilePath();

        public static string GetDiscoveryFilePath()
        {
            return Path.Combine(Directory.GetCurrentDirectory(), "Library", "UnityAiBridge", "server.json");
        }

        public bool IsRunning
        {
            get
            {
                lock (lifecycleLock)
                {
                    return listener != null;
                }
            }
        }

        public static UnityAiBridgeTcpServer StartShared()
        {
            lock (SharedLifecycleLock)
            {
                if (sharedInstance == null)
                {
                    sharedInstance = new UnityAiBridgeTcpServer();
                }

                sharedInstance.Start();
                return sharedInstance;
            }
        }

        public static void StopShared()
        {
            StopSharedInstance();
        }

        public static UnityAiBridgeTcpServer EnsureSharedDiscoverable()
        {
            lock (SharedLifecycleLock)
            {
                if (sharedInstance == null)
                {
                    sharedInstance = new UnityAiBridgeTcpServer();
                }

                if (!sharedInstance.IsRunning)
                {
                    sharedInstance.Start();
                    return sharedInstance;
                }

                sharedInstance.EnsureDiscoveryFile();
                return sharedInstance;
            }
        }

        public static bool IsSharedRunning
        {
            get
            {
                lock (SharedLifecycleLock)
                {
                    return sharedInstance != null && sharedInstance.IsRunning;
                }
            }
        }

        public static void BroadcastEvent(string eventType, object payload)
        {
            UnityAiBridgeTcpServer server;
            lock (SharedLifecycleLock)
            {
                server = sharedInstance;
            }

            server?.BroadcastEventInternal(eventType, payload);
        }

        public static void BroadcastCommand(string command, object parameters)
        {
            UnityAiBridgeTcpServer server;
            lock (SharedLifecycleLock)
            {
                server = sharedInstance;
            }

            server?.BroadcastCommandInternal(command, parameters);
        }

        public void Start()
        {
            lock (lifecycleLock)
            {
                ThrowIfDisposed();

                if (listener != null)
                {
                    EnsureDiscoveryFile();
                    return;
                }

                token = GenerateToken();
                discoveryFilePath = GetDefaultDiscoveryFilePath();
                cancellationTokenSource = new CancellationTokenSource();
                UnityAiBridgeProtocol.TryRefreshCommandRegistry("TCP server start");
                UnityAiBridgeProtocol.LogRegisteredCommands("before TCP accept");
                listener = new TcpListener(IPAddress.Loopback, 0);
                try
                {
                    listener.Start();
                    Port = ((IPEndPoint)listener.LocalEndpoint).Port;
                    WriteDiscoveryFile();
                    acceptTask = Task.Run(() => AcceptLoop(cancellationTokenSource.Token));
                    TryInvokeLuxCompileWatcher("StartIfBridgeRunning", null);
                }
                catch (Exception exception)
                {
                    Trace.TraceError($"Unity AI Bridge TCP server failed to start: {exception}");
                    listener.Stop();
                    cancellationTokenSource.Dispose();
                    listener = null;
                    cancellationTokenSource = null;
                    Port = 0;
                    token = null;
                    DeleteDiscoveryFile();
                    throw;
                }
            }
        }

        public void Stop()
        {
            TcpListener listenerToStop;
            CancellationTokenSource sourceToCancel;

            lock (lifecycleLock)
            {
                listenerToStop = listener;
                sourceToCancel = cancellationTokenSource;
                listener = null;
                cancellationTokenSource = null;
                acceptTask = null;
                Port = 0;
                token = null;
            }

            if (sourceToCancel != null)
            {
                sourceToCancel.Cancel();
            }

            if (listenerToStop != null)
            {
                listenerToStop.Stop();
            }

            if (sourceToCancel != null)
            {
                sourceToCancel.Dispose();
            }

            DeleteDiscoveryFile();
            ClearConnectedClients();
        }

        public void Dispose()
        {
            if (disposed)
            {
                return;
            }

            Stop();
            disposed = true;
        }

        private static void StopSharedInstance()
        {
            lock (SharedLifecycleLock)
            {
                if (sharedInstance == null)
                {
                    return;
                }

                sharedInstance.Dispose();
                sharedInstance = null;
            }
        }

        private static void HandlePlayModeStateChanged(PlayModeStateChange state)
        {
            // Keep the Lux backend alive across PlayMode transitions so CLI/backend
            // operations work even when Unity is unfocused or entering PlayMode.
            // The server is stopped only for assembly reload and Editor quitting.
        }

        private async Task AcceptLoop(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                TcpClient client = null;

                try
                {
                    TcpListener activeListener;
                    lock (lifecycleLock)
                    {
                        activeListener = listener;
                    }

                    if (activeListener == null)
                    {
                        return;
                    }

                    client = await activeListener.AcceptTcpClientAsync().ConfigureAwait(false);
                    _ = Task.Run(() => HandleClient(client, cancellationToken), cancellationToken);
                }
                catch (ObjectDisposedException)
                {
                    Trace.TraceInformation("Unity AI Bridge TCP listener stopped after disposal.");
                    return;
                }
                catch (SocketException) when (cancellationToken.IsCancellationRequested)
                {
                    Trace.TraceInformation("Unity AI Bridge TCP listener stopped after cancellation.");
                    return;
                }
                catch (SocketException exception)
                {
                    Trace.TraceWarning($"Unity AI Bridge TCP accept failed: {exception.SocketErrorCode} {exception.Message}");
                    client?.Close();
                }
                catch (InvalidOperationException) when (cancellationToken.IsCancellationRequested)
                {
                    Trace.TraceInformation("Unity AI Bridge TCP listener stopped during cancellation.");
                    return;
                }
                catch (InvalidOperationException exception)
                {
                    Trace.TraceWarning($"Unity AI Bridge TCP accept loop encountered an invalid listener state: {exception.Message}");
                    client?.Close();
                }
            }
        }

        private void HandleClient(TcpClient client, CancellationToken cancellationToken)
        {
            try
            {
                using (client)
                using (var stream = client.GetStream())
                using (var reader = new StreamReader(stream, Encoding.UTF8, false, 1024, true))
                using (var writer = new StreamWriter(stream, new UTF8Encoding(false), 1024, true))
                {
                    writer.NewLine = "\n";
                    var connection = new ClientConnection(client, writer);

                    string line;
                    while (!cancellationToken.IsCancellationRequested && (line = reader.ReadLine()) != null)
                    {
                        if (line.Length == 0)
                        {
                            continue;
                        }

                        var response = HandleRequestLine(line, connection);
                        connection.WriteLine(ToJsonLine(response));
                    }

                    RemoveClient(connection);
                }
            }
            catch (ObjectDisposedException exception)
            {
                Trace.TraceInformation($"Unity AI Bridge TCP client closed during disposal: {exception.Message}");
            }
            catch (IOException exception)
            {
                Trace.TraceWarning($"Unity AI Bridge TCP client I/O failed: {exception.Message}");
            }
            catch (SocketException exception)
            {
                Trace.TraceWarning($"Unity AI Bridge TCP client socket failed: {exception.SocketErrorCode} {exception.Message}");
            }
            catch (InvalidOperationException exception)
            {
                Trace.TraceWarning($"Unity AI Bridge TCP client handling failed: {exception.Message}");
            }
        }

        private UnityAiBridgeProtocolResponse HandleRequestLine(string line, ClientConnection connection)
        {
            var request = ParseRequestLine(line);
            if (!UnityAiBridgeProtocol.IsRegistryReady)
            {
                UnityAiBridgeProtocol.TryRefreshCommandRegistry("TCP request");
            }

            var releaseDynamicCodeFlight = false;
            if (IsAuthorized(request))
            {
                AddClient(connection);
            }

            if (string.Equals(request?.command, UnityAiBridgeProtocol.CommandSubscribeEvents, StringComparison.Ordinal))
            {
                return HandleSubscribeEvents(request, connection);
            }

            if (string.Equals(request?.command, UnityAiBridgeProtocol.CommandTriggerCompile, StringComparison.Ordinal))
            {
                return HandleTriggerCompile(request);
            }

            if (IsDynamicCodeCommand(request))
            {
                if (Interlocked.CompareExchange(ref dynamicCodeInFlight, 1, 0) != 0)
                {
                    return UnityAiBridgeProtocol.CreateErrorResponse(
                        request?.requestId,
                        UnityAiBridgeProtocol.ErrorCodeSingleFlightBusy,
                        "A Lux dynamic code execution is already running.");
                }

                releaseDynamicCodeFlight = true;
            }

            if (RequiresMainThreadDispatch(request))
            {
                return HandleRequestOnMainThread(request, releaseDynamicCodeFlight);
            }

            try
            {
                return UnityAiBridgeProtocol.Handle(request, token);
            }
            finally
            {
                if (releaseDynamicCodeFlight)
                {
                    Volatile.Write(ref dynamicCodeInFlight, 0);
                }
            }
        }

        private static bool RequiresMainThreadDispatch(UnityAiBridgeProtocolRequest request)
        {
            switch (request?.command)
            {
                case UnityAiBridgeProtocol.CommandGetSelectedFileContext:
                case "focus_lux_window":
                case "run_lux_scene_smoke":
                case "create_lux_scene_objects":
                case "get_lux_console_logs":
                case "clear_lux_console":
                case "find_lux_game_objects":
                case "get_lux_hierarchy":
                case "control_lux_play_mode":
                case "capture_lux_screenshot":
                case "simulate_lux_mouse_ui":
                case "simulate_lux_keyboard":
                case "simulate_lux_mouse_input":
                case "record_lux_input":
                case "replay_lux_input":
                case UnityAiBridgeProtocol.CommandStartLuxStream:
                case UnityAiBridgeProtocol.CommandStopLuxStream:
                case UnityAiBridgeProtocol.CommandLuxInputEvent:
                case UnityAiBridgeProtocol.CommandReadAssetAst:
                case UnityAiBridgeProtocol.CommandGetSelectionAst:
                case UnityAiBridgeProtocol.CommandGetSceneAst:
                case DynamicCodeCommand:
                    return true;
                default:
                    return false;
            }
        }

        private static bool IsDynamicCodeCommand(UnityAiBridgeProtocolRequest request)
        {
            return string.Equals(request?.command, DynamicCodeCommand, StringComparison.Ordinal);
        }

        private bool IsAuthorized(UnityAiBridgeProtocolRequest request)
        {
            return request != null
                && !string.IsNullOrEmpty(token)
                && !string.IsNullOrEmpty(request.token)
                && string.Equals(request.token, token, StringComparison.Ordinal);
        }

        private UnityAiBridgeProtocolResponse HandleSubscribeEvents(UnityAiBridgeProtocolRequest request, ClientConnection connection)
        {
            if (!IsAuthorized(request))
            {
                return UnityAiBridgeProtocol.CreateErrorResponse(
                    request?.requestId,
                    UnityAiBridgeProtocol.ErrorCodeUnauthorized,
                    "Invalid token.");
            }

            var eventTypes = SplitEventTypes(request.@params?.eventTypes);
            connection.SetSubscriptions(eventTypes);
            return UnityAiBridgeProtocol.CreateOkResponse(request.requestId, null);
        }

        private UnityAiBridgeProtocolResponse HandleTriggerCompile(UnityAiBridgeProtocolRequest request)
        {
            if (!IsAuthorized(request))
            {
                return UnityAiBridgeProtocol.CreateErrorResponse(
                    request?.requestId,
                    UnityAiBridgeProtocol.ErrorCodeUnauthorized,
                    "Invalid token.");
            }

            MainThreadActionQueue.Enqueue(() =>
            {
                TryInvokeLuxCompileWatcher("TriggerCompileRefresh", new object[] { "manual trigger_compile command" });
            });
            return UnityAiBridgeProtocol.CreateOkResponse(request.requestId, null);
        }

        private static void TryInvokeLuxCompileWatcher(string methodName, object[] args)
        {
            var watcherType = Type.GetType("Linalab.Lux.Editor.LuxCompileWatcher, Linalab.Lux.Editor");
            var method = watcherType?.GetMethod(methodName, System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static);
            if (method == null)
            {
                return;
            }

            try
            {
                method.Invoke(null, args);
            }
            catch (Exception exception)
            {
                Trace.TraceWarning($"Unity AI Bridge failed to invoke Lux compile watcher {methodName}: {exception.Message}");
            }
        }

        private static string[] SplitEventTypes(string eventTypes)
        {
            if (string.IsNullOrWhiteSpace(eventTypes))
            {
                return new string[0];
            }

            return eventTypes
                .Split(new[] { ',', ';', ' ', '\n', '\r', '\t' }, StringSplitOptions.RemoveEmptyEntries)
                .Select(eventType => eventType.Trim())
                .Where(eventType => eventType.Length > 0)
                .Distinct(StringComparer.Ordinal)
                .ToArray();
        }

        private UnityAiBridgeProtocolResponse HandleRequestOnMainThread(UnityAiBridgeProtocolRequest request, bool releaseDynamicCodeFlight = false)
        {
            if (Thread.CurrentThread.ManagedThreadId == MainThreadId)
            {
                try
                {
                    return UnityAiBridgeProtocol.Handle(request, token);
                }
                finally
                {
                    if (releaseDynamicCodeFlight)
                    {
                        Volatile.Write(ref dynamicCodeInFlight, 0);
                    }
                }
            }

            var dispatch = new MainThreadDispatch(request, releaseDynamicCodeFlight);
            var completedInTime = false;
            MainThreadDispatchQueue.Enqueue(dispatch);
            completedInTime = dispatch.Completed.Wait(TimeSpan.FromSeconds(30));
            if (!completedInTime)
            {
                dispatch.MarkTimedOutIfPending();
                return UnityAiBridgeProtocol.CreateErrorResponse(
                    request?.requestId,
                    "main_thread_dispatch_timeout",
                    "Unity AI Bridge timed out waiting for main-thread command dispatch.");
            }

            dispatch.Completed.Dispose();

            if (dispatch.Exception != null)
            {
                return UnityAiBridgeProtocol.CreateErrorResponse(
                    request?.requestId,
                    "main_thread_dispatch_failed",
                    dispatch.Exception.Message);
            }

            return dispatch.Response ?? UnityAiBridgeProtocol.CreateErrorResponse(
                request?.requestId,
                "main_thread_dispatch_failed",
                "Unity AI Bridge main-thread command dispatch returned no response.");
        }

        private static void ProcessMainThreadDispatchQueue()
        {
            UnityAiBridgeTcpServer server;
            lock (SharedLifecycleLock)
            {
                server = sharedInstance;
            }

            while (MainThreadActionQueue.TryDequeue(out var action))
            {
                try
                {
                    action();
                }
                catch (Exception exception)
                {
                    Trace.TraceWarning($"Unity AI Bridge main-thread action failed: {exception.Message}");
                }
            }

            while (MainThreadDispatchQueue.TryDequeue(out var dispatch))
            {
                if (!dispatch.TryBegin())
                {
                    continue;
                }

                if (server == null || !server.IsRunning)
                {
                    dispatch.Response = UnityAiBridgeProtocol.CreateErrorResponse(
                        dispatch.Request?.requestId,
                        "server_not_running",
                        "Unity AI Bridge server is not running.");
                    dispatch.Complete();
                    continue;
                }

                server.ProcessStartedMainThreadDispatch(dispatch.Request, dispatch);
            }
        }

        private void ProcessStartedMainThreadDispatch(UnityAiBridgeProtocolRequest request, MainThreadDispatch dispatch)
        {
            try
            {
                dispatch.Response = UnityAiBridgeProtocol.Handle(request, token);
            }
            catch (Exception exception)
            {
                dispatch.Exception = exception;
            }
            finally
            {
                dispatch.Complete();
            }
        }

        private void AddClient(ClientConnection connection)
        {
            if (connection == null)
            {
                return;
            }

            lock (clientsLock)
            {
                if (!connectedClients.Contains(connection))
                {
                    connectedClients.Add(connection);
                }

                connection.Authenticated = true;
            }
        }

        private void RemoveClient(ClientConnection connection)
        {
            if (connection == null)
            {
                return;
            }

            lock (clientsLock)
            {
                connectedClients.Remove(connection);
            }
        }

        private void ClearConnectedClients()
        {
            lock (clientsLock)
            {
                connectedClients.Clear();
            }
        }

        private void BroadcastEventInternal(string eventType, object payload)
        {
            if (string.IsNullOrWhiteSpace(eventType))
            {
                return;
            }

            var eventLine = ToEventJsonLine(eventType, payload);
            ClientConnection[] clients;
            lock (clientsLock)
            {
                clients = connectedClients.ToArray();
            }

            foreach (var client in clients)
            {
                if (!client.Authenticated || !client.IsSubscribedTo(eventType))
                {
                    continue;
                }

                try
                {
                    client.WriteLine(eventLine);
                }
                catch (IOException exception)
                {
                    Trace.TraceWarning($"Unity AI Bridge TCP event broadcast failed: {exception.Message}");
                    RemoveClient(client);
                }
                catch (ObjectDisposedException exception)
                {
                    Trace.TraceInformation($"Unity AI Bridge TCP event broadcast skipped disposed client: {exception.Message}");
                    RemoveClient(client);
                }
                catch (InvalidOperationException exception)
                {
                    Trace.TraceWarning($"Unity AI Bridge TCP event broadcast failed: {exception.Message}");
                    RemoveClient(client);
                }
            }
        }

        private void BroadcastCommandInternal(string command, object parameters)
        {
            if (string.IsNullOrWhiteSpace(command))
            {
                return;
            }

            var commandLine = ToCommandJsonLine(command, parameters);
            ClientConnection[] clients;
            lock (clientsLock)
            {
                clients = connectedClients.ToArray();
            }

            foreach (var client in clients)
            {
                if (!client.Authenticated)
                {
                    continue;
                }

                try
                {
                    client.WriteLine(commandLine);
                }
                catch (IOException exception)
                {
                    Trace.TraceWarning($"Unity AI Bridge TCP command broadcast failed: {exception.Message}");
                    RemoveClient(client);
                }
                catch (ObjectDisposedException exception)
                {
                    Trace.TraceInformation($"Unity AI Bridge TCP command broadcast skipped disposed client: {exception.Message}");
                    RemoveClient(client);
                }
                catch (InvalidOperationException exception)
                {
                    Trace.TraceWarning($"Unity AI Bridge TCP command broadcast failed: {exception.Message}");
                    RemoveClient(client);
                }
            }
        }

        private static string ToEventJsonLine(string eventType, object payload)
        {
            var builder = new StringBuilder(256);
            builder.Append('{');
            AppendJsonProperty(builder, "type", "event", false);
            AppendJsonProperty(builder, "event", eventType, true);
            AppendJsonProperty(builder, "timestamp", DateTime.UtcNow.ToString("O"), true);
            builder.Append(",\"payload\":");
            AppendJsonValue(builder, payload);
            builder.Append('}');
            return builder.ToString();
        }

        private static string ToCommandJsonLine(string command, object parameters)
        {
            var builder = new StringBuilder(256);
            builder.Append('{');
            AppendJsonProperty(builder, "schemaVersion", UnityAiBridgeProtocol.SchemaVersion, false);
            AppendJsonProperty(builder, "command", command, true);
            AppendJsonProperty(builder, "capturedAtUtc", DateTime.UtcNow.ToString("O"), true);
            builder.Append(",\"params\":");
            AppendJsonValue(builder, parameters);
            builder.Append('}');
            return builder.ToString();
        }

        private static void AppendJsonValue(StringBuilder builder, object value)
        {
            if (value == null)
            {
                builder.Append("null");
                return;
            }

            if (value is string stringValue)
            {
                AppendJsonString(builder, stringValue);
                return;
            }

            if (value is bool boolValue)
            {
                builder.Append(boolValue ? "true" : "false");
                return;
            }

            if (value is int || value is long || value is short || value is byte || value is uint || value is ulong || value is ushort || value is sbyte)
            {
                builder.Append(Convert.ToString(value, CultureInfo.InvariantCulture));
                return;
            }

            if (value is float || value is double || value is decimal)
            {
                builder.Append(Convert.ToString(value, CultureInfo.InvariantCulture));
                return;
            }

            if (value is System.Collections.IEnumerable enumerable)
            {
                builder.Append('[');
                var index = 0;
                foreach (var item in enumerable)
                {
                    if (index++ > 0)
                    {
                        builder.Append(',');
                    }

                    AppendJsonValue(builder, item);
                }

                builder.Append(']');
                return;
            }

            builder.Append('{');
            var wrote = false;
            var type = value.GetType();
            foreach (var field in type.GetFields(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public))
            {
                if (wrote)
                {
                    builder.Append(',');
                }

                AppendJsonString(builder, field.Name);
                builder.Append(':');
                AppendJsonValue(builder, field.GetValue(value));
                wrote = true;
            }

            foreach (var property in type.GetProperties(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public))
            {
                if (!property.CanRead || property.GetIndexParameters().Length > 0)
                {
                    continue;
                }

                if (wrote)
                {
                    builder.Append(',');
                }

                AppendJsonString(builder, property.Name);
                builder.Append(':');
                AppendJsonValue(builder, property.GetValue(value, null));
                wrote = true;
            }

            builder.Append('}');
        }

        private sealed class MainThreadDispatch
        {
            private const int Pending = 0;
            private const int Started = 1;
            private const int TimedOut = 2;
            private int state;
            private bool waitTimedOut;
            private readonly Action onCompleted;

            public MainThreadDispatch(UnityAiBridgeProtocolRequest request, bool releaseDynamicCodeFlight = false)
            {
                Request = request;
                if (releaseDynamicCodeFlight)
                {
                    onCompleted = () => Volatile.Write(ref dynamicCodeInFlight, 0);
                }
            }

            public UnityAiBridgeProtocolRequest Request { get; }
            public ManualResetEventSlim Completed { get; } = new ManualResetEventSlim(false);
            public UnityAiBridgeProtocolResponse Response { get; set; }
            public Exception Exception { get; set; }

            public bool TryBegin()
            {
                return Interlocked.CompareExchange(ref state, Started, Pending) == Pending;
            }

            public void MarkTimedOutIfPending()
            {
                waitTimedOut = true;
                if (Interlocked.CompareExchange(ref state, TimedOut, Pending) == Pending)
                {
                    onCompleted?.Invoke();
                    Completed.Dispose();
                }
                else if (Completed.IsSet)
                {
                    Completed.Dispose();
                }
            }

            public void Complete()
            {
                onCompleted?.Invoke();
                Completed.Set();
                if (waitTimedOut)
                {
                    Completed.Dispose();
                }
            }
        }

        private sealed class ClientConnection
        {
            private readonly object writeLock = new object();
            private readonly StreamWriter writer;
            private readonly HashSet<string> subscriptions = new HashSet<string>(StringComparer.Ordinal);
            private bool hasExplicitSubscriptions;

            public ClientConnection(TcpClient client, StreamWriter writer)
            {
                Client = client;
                this.writer = writer;
            }

            public TcpClient Client { get; }
            public bool Authenticated { get; set; }

            public void SetSubscriptions(string[] eventTypes)
            {
                lock (subscriptions)
                {
                    subscriptions.Clear();
                    hasExplicitSubscriptions = eventTypes != null && eventTypes.Length > 0;
                    if (eventTypes == null)
                    {
                        return;
                    }

                    foreach (var eventType in eventTypes)
                    {
                        subscriptions.Add(eventType);
                    }
                }
            }

            public bool IsSubscribedTo(string eventType)
            {
                lock (subscriptions)
                {
                    return !hasExplicitSubscriptions || subscriptions.Contains(eventType);
                }
            }

            public void WriteLine(string line)
            {
                lock (writeLock)
                {
                    writer.WriteLine(line);
                    writer.Flush();
                }
            }
        }

        private static UnityAiBridgeProtocolRequest ParseRequestLine(string line)
        {
            if (string.IsNullOrEmpty(line))
            {
                Trace.TraceWarning("Unity AI Bridge TCP received an empty request line.");
                return null;
            }

            var request = new UnityAiBridgeProtocolRequest
            {
                schemaVersion = ExtractInt(line, "schemaVersion"),
                requestId = ExtractString(line, "requestId"),
                command = ExtractString(line, "command"),
                token = ExtractString(line, "token"),
                @params = new UnityAiBridgeProtocolRequestParameters
                {
                    commandText = ExtractString(line, "commandText"),
                    gitArguments = ExtractString(line, "gitArguments"),
                    workingDirectory = ExtractString(line, "workingDirectory"),
                    actor = ExtractString(line, "actor"),
                    approvalGranted = ExtractBool(line, "approvalGranted"),
                    scenePath = ExtractString(line, "scenePath"),
                    sceneSmokeObjectCount = ExtractInt(line, "sceneSmokeObjectCount"),
                    findGameObjectsSearchMode = ExtractString(line, "findGameObjectsSearchMode"),
                    findGameObjectsName = ExtractString(line, "findGameObjectsName"),
                    findGameObjectsRegex = ExtractString(line, "findGameObjectsRegex"),
                    findGameObjectsPath = ExtractString(line, "findGameObjectsPath"),
                    findGameObjectsComponent = ExtractString(line, "findGameObjectsComponent"),
                    findGameObjectsTag = ExtractString(line, "findGameObjectsTag"),
                    findGameObjectsLayer = ExtractString(line, "findGameObjectsLayer"),
                    findGameObjectsActiveState = ExtractString(line, "findGameObjectsActiveState"),
                    findGameObjectsInlineLimit = ExtractInt(line, "findGameObjectsInlineLimit"),
                    hierarchyAll = ExtractBool(line, "hierarchyAll"),
                    hierarchyRootPath = ExtractString(line, "hierarchyRootPath"),
                    hierarchyUseSelection = ExtractBool(line, "hierarchyUseSelection"),
                    playModeAction = ExtractString(line, "playModeAction"),
                    screenshotCaptureMode = ExtractString(line, "screenshotCaptureMode"),
                    screenshotAnnotateElements = ExtractBool(line, "screenshotAnnotateElements"),
                    screenshotElementsOnly = ExtractBool(line, "screenshotElementsOnly"),
                    mouseUiAction = ExtractString(line, "mouseUiAction"),
                    mouseUiX = ExtractFloat(line, "mouseUiX"),
                    mouseUiY = ExtractFloat(line, "mouseUiY"),
                    mouseUiDurationMs = ExtractInt(line, "mouseUiDurationMs"),
                    inputAction = ExtractString(line, "inputAction"),
                    inputKey = ExtractString(line, "inputKey"),
                    inputButton = ExtractString(line, "inputButton"),
                    inputDurationMs = ExtractInt(line, "inputDurationMs"),
                    inputDeltaX = ExtractFloat(line, "inputDeltaX"),
                    inputDeltaY = ExtractFloat(line, "inputDeltaY"),
                    inputScrollX = ExtractFloat(line, "inputScrollX"),
                    inputScrollY = ExtractFloat(line, "inputScrollY"),
                    inputSteps = ExtractInt(line, "inputSteps"),
                    inputFilePath = ExtractString(line, "inputFilePath"),
                    dynamicCode = ExtractString(line, "dynamicCode"),
                    eventTypes = ExtractString(line, "eventTypes"),
                    testPlatform = ExtractString(line, "testPlatform"),
                    testResults = ExtractString(line, "testResults"),
                    width = ExtractInt(line, "width"),
                    height = ExtractInt(line, "height"),
                    fps = ExtractInt(line, "fps"),
                    session_id = ExtractString(line, "session_id"),
                    type = ExtractString(line, "type"),
                    x = ExtractFloat(line, "x"),
                    y = ExtractFloat(line, "y"),
                    button = ExtractInt(line, "button"),
                    key_code = ExtractString(line, "key_code")
                }
            };

            if (request.schemaVersion == 0 && request.requestId == null && request.command == null && request.token == null)
            {
                Trace.TraceWarning("Unity AI Bridge TCP received a request line with no recognized protocol fields.");
                return null;
            }

            return request;
        }

        private static string ToJsonLine(UnityAiBridgeProtocolResponse response)
        {
            var builder = new StringBuilder(256);
            builder.Append('{');
            AppendJsonProperty(builder, "schemaVersion", response.schemaVersion, false);
            AppendJsonProperty(builder, "requestId", response.requestId, true);
            AppendJsonProperty(builder, "ok", response.ok, true);
            AppendJsonProperty(builder, "errorCode", response.errorCode, true);
            AppendJsonProperty(builder, "errorMessage", response.errorMessage, true);
            AppendPayload(builder, response.payload);
            AppendJsonProperty(builder, "capturedAtUtc", response.capturedAtUtc, true);
            builder.Append('}');
            return builder.ToString();
        }

        private static void AppendPayload(StringBuilder builder, UnityAiBridgeProtocolResponsePayload payload)
        {
            builder.Append(",\"payload\":");
            if (payload == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            builder.Append("\"ping\":");
            if (payload.ping == null)
            {
                builder.Append("null");
            }
            else
            {
                builder.Append('{');
                AppendJsonProperty(builder, "status", payload.ping.status, false);
                builder.Append('}');
            }

            builder.Append(",\"protocolInfo\":");
            if (payload.protocolInfo == null)
            {
                builder.Append("null");
            }
            else
            {
                builder.Append('{');
                AppendJsonProperty(builder, "protocolVersion", payload.protocolInfo.protocolVersion, false);
                AppendJsonProperty(builder, "backendVersion", payload.protocolInfo.backendVersion, true);
                builder.Append(",\"commands\":[");
                var commands = payload.protocolInfo.commands;
                for (var i = 0; commands != null && i < commands.Length; i++)
                {
                    if (i > 0)
                    {
                        builder.Append(',');
                    }

                    AppendJsonString(builder, commands[i]);
                }

                builder.Append("]}");
            }

            builder.Append(",\"selectedFileContext\":");
            if (payload.selectedFileContext == null)
            {
                builder.Append("null");
            }
            else
            {
                builder.Append('{');
                AppendJsonProperty(builder, "projectName", payload.selectedFileContext.projectName, false);
                AppendJsonProperty(builder, "projectPath", payload.selectedFileContext.projectPath, true);
                AppendJsonProperty(builder, "unityVersion", payload.selectedFileContext.unityVersion, true);
                AppendJsonProperty(builder, "selectionCapturedAtUtc", payload.selectedFileContext.selectionCapturedAtUtc, true);
                AppendJsonProperty(builder, "selectionCount", payload.selectedFileContext.selectionCount, true);
                AppendSelectedFiles(builder, payload.selectedFileContext.selectedFiles);
                builder.Append('}');
            }

            builder.Append(",\"luxContext\":");
            AppendLuxContext(builder, payload.luxContext);

            builder.Append(",\"luxAutomationResult\":");
            AppendLuxAutomationResult(builder, payload.luxAutomationResult);

            builder.Append(",\"focusWindowResult\":");
            AppendFocusWindowResult(builder, payload.focusWindowResult);

            builder.Append(",\"consoleLogs\":");
            AppendConsoleLogs(builder, payload.consoleLogs);

            builder.Append(",\"consoleClearResult\":");
            AppendConsoleClearResult(builder, payload.consoleClearResult);

            builder.Append(",\"findGameObjectsResult\":");
            AppendFindGameObjectsResult(builder, payload.findGameObjectsResult);

            builder.Append(",\"getHierarchyResult\":");
            AppendGetHierarchyResult(builder, payload.getHierarchyResult);

            builder.Append(",\"playModeState\":");
            AppendPlayModeState(builder, payload.playModeState);

            builder.Append(",\"screenshotResult\":");
            AppendScreenshotResult(builder, payload.screenshotResult);

            builder.Append(",\"mouseUiResult\":");
            AppendMouseUiResult(builder, payload.mouseUiResult);

            builder.Append(",\"inputSimulationResult\":");
            AppendInputSimulationResult(builder, payload.inputSimulationResult);

            builder.Append(",\"inputRecordResult\":");
            AppendInputRecordResult(builder, payload.inputRecordResult);

            builder.Append(",\"inputReplayResult\":");
            AppendInputReplayResult(builder, payload.inputReplayResult);

            builder.Append(",\"dynamicCodeResult\":");
            AppendDynamicCodeResult(builder, payload.dynamicCodeResult);

            builder.Append(",\"captureStreamResult\":");
            AppendJsonValue(builder, payload.captureStreamResult);

            builder.Append(",\"inputEventResult\":");
            AppendJsonValue(builder, payload.inputEventResult);

            builder.Append('}');
        }

        private static void AppendLuxContext(StringBuilder builder, UnityAiBridgeLuxContextPayload context)
        {
            if (context == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "packageName", context.packageName, false);
            AppendJsonProperty(builder, "protocolSurface", context.protocolSurface, true);
            AppendJsonProperty(builder, "projectPath", context.projectPath, true);
            AppendJsonProperty(builder, "unityVersion", context.unityVersion, true);
            AppendJsonProperty(builder, "platform", context.platform, true);
            AppendJsonProperty(builder, "remotePhase", context.remotePhase, true);
            AppendJsonProperty(builder, "videoTransport", context.videoTransport, true);
            AppendJsonProperty(builder, "signalingTransport", context.signalingTransport, true);
            AppendJsonProperty(builder, "controlTransport", context.controlTransport, true);
            AppendJsonProperty(builder, "permissionModel", context.permissionModel, true);
            AppendJsonProperty(builder, "includesIosClientImplementation", context.includesIosClientImplementation, true);
            AppendStringArray(builder, "automationBlockedTokens", context.automationBlockedTokens);
            AppendStringArray(builder, "automationApprovalTokens", context.automationApprovalTokens);
            AppendJsonProperty(builder, "auditEntryCount", context.auditEntryCount, true);
            builder.Append('}');
        }

        private static void AppendLuxAutomationResult(StringBuilder builder, UnityAiBridgeLuxAutomationResultPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "allowed", result.allowed, false);
            AppendJsonProperty(builder, "success", result.success, true);
            AppendJsonProperty(builder, "exitCode", result.exitCode, true);
            AppendJsonProperty(builder, "output", result.output, true);
            AppendJsonProperty(builder, "error", result.error, true);
            AppendJsonProperty(builder, "message", result.message, true);
            builder.Append('}');
        }

        private static void AppendConsoleLogs(StringBuilder builder, UnityAiBridgeConsoleLogsPayload logs)
        {
            if (logs == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "totalCount", logs.totalCount, false);
            AppendJsonProperty(builder, "displayedCount", logs.displayedCount, true);
            builder.Append(",\"consoleLogs\":[");
            for (var i = 0; logs.consoleLogs != null && i < logs.consoleLogs.Length; i++)
            {
                if (i > 0)
                {
                    builder.Append(',');
                }

                AppendConsoleLogEntry(builder, logs.consoleLogs[i]);
            }

            builder.Append("]}");
        }

        private static void AppendConsoleLogEntry(StringBuilder builder, UnityAiBridgeConsoleLogEntry entry)
        {
            if (entry == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "level", entry.level, false);
            AppendJsonProperty(builder, "message", entry.message, true);
            AppendJsonProperty(builder, "stackTrace", entry.stackTrace, true);
            AppendJsonProperty(builder, "timestampUtc", entry.timestampUtc, true);
            builder.Append('}');
        }

        private static void AppendConsoleClearResult(StringBuilder builder, UnityAiBridgeConsoleClearPayload clearResult)
        {
            if (clearResult == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "beforeCount", clearResult.beforeCount, false);
            AppendJsonProperty(builder, "afterCount", clearResult.afterCount, true);
            builder.Append('}');
        }
        private static void AppendFocusWindowResult(StringBuilder builder, UnityAiBridgeFocusWindowPayload focusResult)
        {
            if (focusResult == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "focused", focusResult.focused, false);
            builder.Append('}');
        }

        private static void AppendFindGameObjectsResult(StringBuilder builder, UnityAiBridgeFindGameObjectsPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "searchMode", result.searchMode, false);
            AppendJsonProperty(builder, "totalMatchCount", result.totalMatchCount, true);
            AppendJsonProperty(builder, "returnedCount", result.returnedCount, true);
            AppendJsonProperty(builder, "truncated", result.truncated, true);
            AppendJsonProperty(builder, "outputFilePath", result.outputFilePath, true);
            AppendJsonProperty(builder, "fileSizeBytes", result.fileSizeBytes, true);
            builder.Append(",\"gameObjects\":[");
            for (var i = 0; result.gameObjects != null && i < result.gameObjects.Length; i++)
            {
                if (i > 0)
                {
                    builder.Append(',');
                }

                AppendFindGameObjectsEntry(builder, result.gameObjects[i]);
            }

            builder.Append("]}");
        }

        private static void AppendFindGameObjectsEntry(StringBuilder builder, UnityAiBridgeFindGameObjectsEntry entry)
        {
            if (entry == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "name", entry.name, false);
            AppendJsonProperty(builder, "hierarchyPath", entry.hierarchyPath, true);
            AppendJsonProperty(builder, "sceneName", entry.sceneName, true);
            AppendJsonProperty(builder, "scenePath", entry.scenePath, true);
            AppendJsonProperty(builder, "activeSelf", entry.activeSelf, true);
            AppendJsonProperty(builder, "activeInHierarchy", entry.activeInHierarchy, true);
            AppendJsonProperty(builder, "tag", entry.tag, true);
            AppendJsonProperty(builder, "layer", entry.layer, true);
            AppendJsonProperty(builder, "layerName", entry.layerName, true);
            AppendJsonProperty(builder, "instanceId", entry.instanceId, true);
            AppendStringArray(builder, "componentTypeNames", entry.componentTypeNames);
            builder.Append('}');
        }

        private static void AppendGetHierarchyResult(StringBuilder builder, UnityAiBridgeGetHierarchyPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "filePath", result.filePath, false);
            AppendJsonProperty(builder, "fileSizeBytes", result.fileSizeBytes, true);
            AppendJsonProperty(builder, "rootCount", result.rootCount, true);
            AppendJsonProperty(builder, "nodeCount", result.nodeCount, true);
            builder.Append(",\"activeScene\":");
            AppendHierarchyActiveScene(builder, result.activeScene);
            builder.Append(",\"filters\":");
            AppendHierarchyFilters(builder, result.filters);
            builder.Append('}');
        }

        private static void AppendHierarchyActiveScene(StringBuilder builder, UnityAiBridgeHierarchyActiveScene activeScene)
        {
            if (activeScene == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "name", activeScene.name, false);
            AppendJsonProperty(builder, "path", activeScene.path, true);
            builder.Append('}');
        }

        private static void AppendHierarchyFilters(StringBuilder builder, UnityAiBridgeHierarchyFilters filters)
        {
            if (filters == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "all", filters.all, false);
            AppendJsonProperty(builder, "rootPath", filters.rootPath, true);
            AppendJsonProperty(builder, "useSelection", filters.useSelection, true);
            builder.Append('}');
        }

        private static void AppendPlayModeState(StringBuilder builder, UnityAiBridgePlayModeStatePayload state)
        {
            if (state == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "action", state.action, false);
            AppendJsonProperty(builder, "isPlaying", state.isPlaying, true);
            AppendJsonProperty(builder, "isPaused", state.isPaused, true);
            AppendJsonProperty(builder, "transitionRequested", state.transitionRequested, true);
            builder.Append('}');
        }

        private static void AppendScreenshotResult(StringBuilder builder, UnityAiBridgeScreenshotPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "filePath", result.filePath, false);
            AppendJsonProperty(builder, "fileSizeBytes", result.fileSizeBytes, true);
            AppendJsonProperty(builder, "mediaType", result.mediaType, true);
            AppendJsonProperty(builder, "captureMode", result.captureMode, true);
            AppendJsonProperty(builder, "annotated", result.annotated, true);
            AppendJsonProperty(builder, "elementsOnly", result.elementsOnly, true);
            AppendJsonProperty(builder, "screenshotSaved", result.screenshotSaved, true);
            AppendJsonProperty(builder, "annotationCount", result.annotationCount, true);
            builder.Append(",\"annotatedElements\":[");
            for (var i = 0; result.annotatedElements != null && i < result.annotatedElements.Length; i++)
            {
                if (i > 0)
                {
                    builder.Append(',');
                }

                AppendScreenshotElement(builder, result.annotatedElements[i]);
            }

            builder.Append("]}");
        }

        private static void AppendScreenshotElement(StringBuilder builder, UnityAiBridgeScreenshotElement element)
        {
            if (element == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "documentName", element.documentName, false);
            AppendJsonProperty(builder, "label", element.label, true);
            AppendJsonProperty(builder, "name", element.name, true);
            AppendJsonProperty(builder, "type", element.type, true);
            AppendJsonProperty(builder, "typeName", element.typeName, true);
            AppendJsonProperty(builder, "path", element.path, true);
            AppendJsonProperty(builder, "x", element.x, true);
            AppendJsonProperty(builder, "y", element.y, true);
            AppendJsonProperty(builder, "width", element.width, true);
            AppendJsonProperty(builder, "height", element.height, true);
            AppendJsonProperty(builder, "visible", element.visible, true);
            AppendJsonProperty(builder, "enabled", element.enabled, true);
            AppendJsonProperty(builder, "pickingMode", element.pickingMode, true);
            AppendJsonProperty(builder, "interaction", element.interaction, true);
            AppendJsonProperty(builder, "simX", element.simX, true);
            AppendJsonProperty(builder, "simY", element.simY, true);
            AppendJsonProperty(builder, "boundsMinX", element.boundsMinX, true);
            AppendJsonProperty(builder, "boundsMaxX", element.boundsMaxX, true);
            AppendJsonProperty(builder, "boundsMinY", element.boundsMinY, true);
            AppendJsonProperty(builder, "boundsMaxY", element.boundsMaxY, true);
            AppendJsonProperty(builder, "sortingOrder", element.sortingOrder, true);
            AppendJsonProperty(builder, "siblingIndex", element.siblingIndex, true);
            AppendJsonProperty(builder, "coordinateSystem", element.coordinateSystem, true);
            AppendJsonProperty(builder, "resolutionScale", element.resolutionScale, true);
            AppendJsonProperty(builder, "yOffset", element.yOffset, true);
            builder.Append('}');
        }

        private static void AppendMouseUiResult(StringBuilder builder, UnityAiBridgeMouseUiPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "action", result.action, false);
            AppendJsonProperty(builder, "x", result.x, true);
            AppendJsonProperty(builder, "y", result.y, true);
            AppendJsonProperty(builder, "success", result.success, true);
            AppendJsonProperty(builder, "targetName", result.targetName, true);
            AppendJsonProperty(builder, "targetPath", result.targetPath, true);
            AppendJsonProperty(builder, "raycastCount", result.raycastCount, true);
            AppendJsonProperty(builder, "dragActive", result.dragActive, true);
            builder.Append('}');
        }

        private static void AppendInputSimulationResult(StringBuilder builder, UnityAiBridgeInputSimulationPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "device", result.device, false);
            AppendJsonProperty(builder, "action", result.action, true);
            AppendJsonProperty(builder, "key", result.key, true);
            AppendJsonProperty(builder, "button", result.button, true);
            AppendJsonProperty(builder, "deltaX", result.deltaX, true);
            AppendJsonProperty(builder, "deltaY", result.deltaY, true);
            AppendJsonProperty(builder, "scrollX", result.scrollX, true);
            AppendJsonProperty(builder, "scrollY", result.scrollY, true);
            AppendStringArray(builder, "heldKeys", result.heldKeys);
            AppendStringArray(builder, "heldButtons", result.heldButtons);
            AppendJsonProperty(builder, "queuedActions", result.queuedActions, true);
            builder.Append('}');
        }

        private static void AppendInputRecordResult(StringBuilder builder, UnityAiBridgeInputRecordPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "action", result.action, false);
            AppendJsonProperty(builder, "active", result.active, true);
            AppendJsonProperty(builder, "frameCount", result.frameCount, true);
            AppendJsonProperty(builder, "filePath", result.filePath, true);
            AppendJsonProperty(builder, "fileSizeBytes", result.fileSizeBytes, true);
            AppendJsonProperty(builder, "mediaType", result.mediaType, true);
            AppendJsonProperty(builder, "message", result.message, true);
            builder.Append('}');
        }

        private static void AppendInputReplayResult(StringBuilder builder, UnityAiBridgeInputReplayPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "action", result.action, false);
            AppendJsonProperty(builder, "active", result.active, true);
            AppendJsonProperty(builder, "filePath", result.filePath, true);
            AppendJsonProperty(builder, "frameCount", result.frameCount, true);
            AppendJsonProperty(builder, "replayedFrameCount", result.replayedFrameCount, true);
            AppendJsonProperty(builder, "completed", result.completed, true);
            AppendJsonProperty(builder, "message", result.message, true);
            builder.Append('}');
        }

        private static void AppendDynamicCodeResult(StringBuilder builder, UnityAiBridgeDynamicCodeResultPayload result)
        {
            if (result == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "success", result.success, false);
            AppendJsonProperty(builder, "action", result.action, true);
            AppendJsonProperty(builder, "result", result.result, true);
            AppendJsonProperty(builder, "resultType", result.resultType, true);
            AppendJsonProperty(builder, "message", result.message, true);
            builder.Append(",\"diagnostics\":[");
            for (var i = 0; result.diagnostics != null && i < result.diagnostics.Length; i++)
            {
                if (i > 0)
                {
                    builder.Append(',');
                }

                AppendDynamicCodeDiagnostic(builder, result.diagnostics[i]);
            }
            builder.Append(']');
            builder.Append(",\"logs\":[");
            for (var i = 0; result.logs != null && i < result.logs.Length; i++)
            {
                if (i > 0)
                {
                    builder.Append(',');
                }

                AppendConsoleLogEntry(builder, result.logs[i]);
            }
            builder.Append(']');
            AppendJsonProperty(builder, "elapsedTimeMs", result.elapsedTimeMs, true);
            builder.Append('}');
        }

        private static void AppendDynamicCodeDiagnostic(StringBuilder builder, UnityAiBridgeDynamicCodeDiagnostic diagnostic)
        {
            if (diagnostic == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "id", diagnostic.id, false);
            AppendJsonProperty(builder, "severity", diagnostic.severity, true);
            AppendJsonProperty(builder, "message", diagnostic.message, true);
            AppendJsonProperty(builder, "line", diagnostic.line, true);
            AppendJsonProperty(builder, "column", diagnostic.column, true);
            builder.Append('}');
        }

        private static void AppendStringArray(StringBuilder builder, string name, string[] values)
        {
            builder.Append(',');
            AppendJsonString(builder, name);
            builder.Append(":");
            builder.Append('[');
            for (var i = 0; values != null && i < values.Length; i++)
            {
                if (i > 0)
                {
                    builder.Append(',');
                }

                AppendJsonString(builder, values[i]);
            }

            builder.Append(']');
        }

        private static void AppendSelectedFiles(StringBuilder builder, UnityAiSelectedFileMetadata[] selectedFiles)
        {
            builder.Append(",\"selectedFiles\":[");
            for (var i = 0; selectedFiles != null && i < selectedFiles.Length; i++)
            {
                if (i > 0)
                {
                    builder.Append(',');
                }

                AppendSelectedFile(builder, selectedFiles[i]);
            }

            builder.Append(']');
        }

        private static void AppendSelectedFile(StringBuilder builder, UnityAiSelectedFileMetadata selectedFile)
        {
            if (selectedFile == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('{');
            AppendJsonProperty(builder, "assetPath", selectedFile.assetPath, false);
            AppendJsonProperty(builder, "absolutePath", selectedFile.absolutePath, true);
            AppendJsonProperty(builder, "guid", selectedFile.guid, true);
            AppendJsonProperty(builder, "name", selectedFile.name, true);
            AppendJsonProperty(builder, "extension", selectedFile.extension, true);
            AppendJsonProperty(builder, "isFolder", selectedFile.isFolder, true);
            AppendJsonProperty(builder, "exists", selectedFile.exists, true);
            AppendJsonProperty(builder, "mainAssetType", selectedFile.mainAssetType, true);
            AppendJsonProperty(builder, "fileSizeBytes", selectedFile.fileSizeBytes, true);
            AppendJsonProperty(builder, "lastModifiedUtc", selectedFile.lastModifiedUtc, true);
            AppendJsonProperty(builder, "selectionIndex", selectedFile.selectionIndex, true);
            AppendJsonProperty(builder, "selectionCount", selectedFile.selectionCount, true);
            builder.Append('}');
        }

        private static void AppendJsonProperty(StringBuilder builder, string name, string value, bool prependComma)
        {
            if (prependComma)
            {
                builder.Append(',');
            }

            AppendJsonString(builder, name);
            builder.Append(':');
            AppendJsonString(builder, value);
        }

        private static void AppendJsonProperty(StringBuilder builder, string name, int value, bool prependComma)
        {
            if (prependComma)
            {
                builder.Append(',');
            }

            AppendJsonString(builder, name);
            builder.Append(':');
            builder.Append(value);
        }

        private static void AppendJsonProperty(StringBuilder builder, string name, long value, bool prependComma)
        {
            if (prependComma)
            {
                builder.Append(',');
            }

            AppendJsonString(builder, name);
            builder.Append(':');
            builder.Append(value);
        }

        private static void AppendJsonProperty(StringBuilder builder, string name, float value, bool prependComma)
        {
            if (prependComma)
            {
                builder.Append(',');
            }

            AppendJsonString(builder, name);
            builder.Append(':');
            builder.Append(value.ToString(CultureInfo.InvariantCulture));
        }

        private static void AppendJsonProperty(StringBuilder builder, string name, bool value, bool prependComma)
        {
            if (prependComma)
            {
                builder.Append(',');
            }

            AppendJsonString(builder, name);
            builder.Append(':');
            builder.Append(value ? "true" : "false");
        }

        private static void AppendJsonString(StringBuilder builder, string value)
        {
            if (value == null)
            {
                builder.Append("null");
                return;
            }

            builder.Append('\"');
            for (var i = 0; i < value.Length; i++)
            {
                var character = value[i];
                switch (character)
                {
                    case '\\':
                        builder.Append("\\\\");
                        break;
                    case '\"':
                        builder.Append("\\\"");
                        break;
                    case '\b':
                        builder.Append("\\b");
                        break;
                    case '\f':
                        builder.Append("\\f");
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
                        if (character < ' ')
                        {
                            builder.Append("\\u");
                            builder.Append(((int)character).ToString("x4"));
                        }
                        else
                        {
                            builder.Append(character);
                        }

                        break;
                }
            }

            builder.Append('\"');
        }

        private static string ExtractString(string json, string fieldName)
        {
            var valueStart = FindStringValueStart(json, fieldName);
            if (valueStart < 0)
            {
                return null;
            }

            var builder = new StringBuilder();
            for (var i = valueStart; i < json.Length; i++)
            {
                var character = json[i];
                if (character == '\"')
                {
                    return builder.ToString();
                }

                if (character != '\\' || i + 1 >= json.Length)
                {
                    builder.Append(character);
                    continue;
                }

                i++;
                AppendEscapedCharacter(builder, json[i]);
            }

            Trace.TraceWarning($"Unity AI Bridge TCP request field '{fieldName}' had an unterminated string value.");
            return null;
        }

        private static int ExtractInt(string json, string fieldName)
        {
            var colonIndex = FindFieldColon(json, fieldName);
            if (colonIndex < 0)
            {
                return 0;
            }

            var index = colonIndex + 1;
            while (index < json.Length && char.IsWhiteSpace(json[index]))
            {
                index++;
            }

            var valueStart = index;
            while (index < json.Length && (char.IsDigit(json[index]) || json[index] == '-'))
            {
                index++;
            }

            if (valueStart == index)
            {
                return 0;
            }

            int value;
            return int.TryParse(json.Substring(valueStart, index - valueStart), out value) ? value : 0;
        }

        private static float ExtractFloat(string json, string fieldName)
        {
            var colonIndex = FindFieldColon(json, fieldName);
            if (colonIndex < 0)
            {
                return 0f;
            }

            var index = colonIndex + 1;
            while (index < json.Length && char.IsWhiteSpace(json[index]))
            {
                index++;
            }

            var valueStart = index;
            while (index < json.Length && (char.IsDigit(json[index]) || json[index] == '-' || json[index] == '+' || json[index] == '.'))
            {
                index++;
            }

            if (valueStart == index)
            {
                return 0f;
            }

            float value;
            return float.TryParse(json.Substring(valueStart, index - valueStart), NumberStyles.Float, CultureInfo.InvariantCulture, out value) ? value : 0f;
        }

        private static bool ExtractBool(string json, string fieldName)
        {
            var colonIndex = FindFieldColon(json, fieldName);
            if (colonIndex < 0)
            {
                return false;
            }

            var index = colonIndex + 1;
            while (index < json.Length && char.IsWhiteSpace(json[index]))
            {
                index++;
            }

            return index + 4 <= json.Length && string.Equals(json.Substring(index, 4), "true", StringComparison.Ordinal);
        }

        private static int FindStringValueStart(string json, string fieldName)
        {
            var colonIndex = FindFieldColon(json, fieldName);
            if (colonIndex < 0)
            {
                return -1;
            }

            var index = colonIndex + 1;
            while (index < json.Length && char.IsWhiteSpace(json[index]))
            {
                index++;
            }

            if (index >= json.Length || json[index] != '\"')
            {
                return -1;
            }

            return index + 1;
        }

        private static int FindFieldColon(string json, string fieldName)
        {
            var pattern = "\"" + fieldName + "\"";
            var fieldIndex = json.IndexOf(pattern, StringComparison.Ordinal);
            if (fieldIndex < 0)
            {
                return -1;
            }

            var index = fieldIndex + pattern.Length;
            while (index < json.Length && char.IsWhiteSpace(json[index]))
            {
                index++;
            }

            return index < json.Length && json[index] == ':' ? index : -1;
        }

        private static void AppendEscapedCharacter(StringBuilder builder, char escapedCharacter)
        {
            switch (escapedCharacter)
            {
                case '\"':
                    builder.Append('\"');
                    break;
                case '\\':
                    builder.Append('\\');
                    break;
                case '/':
                    builder.Append('/');
                    break;
                case 'b':
                    builder.Append('\b');
                    break;
                case 'f':
                    builder.Append('\f');
                    break;
                case 'n':
                    builder.Append('\n');
                    break;
                case 'r':
                    builder.Append('\r');
                    break;
                case 't':
                    builder.Append('\t');
                    break;
                default:
                    builder.Append(escapedCharacter);
                    break;
            }
        }

        private void WriteDiscoveryFile()
        {
            var directory = Path.GetDirectoryName(discoveryFilePath);
            if (!string.IsNullOrEmpty(directory))
            {
                Directory.CreateDirectory(directory);
            }

            var discovery = new UnityAiBridgeDiscovery
            {
                host = DiscoveryHost,
                port = Port,
                token = token,
                protocolVersion = UnityAiBridgeProtocol.ProtocolVersion,
                projectPath = Directory.GetCurrentDirectory(),
                pid = Process.GetCurrentProcess().Id,
                startedAtUtc = DateTime.UtcNow.ToString("O")
            };

            File.WriteAllText(discoveryFilePath, JsonUtility.ToJson(discovery, true));
        }

        private void EnsureDiscoveryFile()
        {
            lock (lifecycleLock)
            {
                ThrowIfDisposed();

                if (listener == null)
                {
                    return;
                }

                if (DiscoveryFileMatchesCurrentListener())
                {
                    return;
                }

                WriteDiscoveryFile();
            }
        }

        private bool DiscoveryFileMatchesCurrentListener()
        {
            var path = DiscoveryFilePath;
            if (!File.Exists(path))
            {
                return false;
            }

            try
            {
                var discovery = JsonUtility.FromJson<UnityAiBridgeDiscovery>(File.ReadAllText(path));
                return discovery != null
                    && string.Equals(discovery.host, DiscoveryHost, StringComparison.Ordinal)
                    && discovery.port == Port
                    && string.Equals(discovery.token, token, StringComparison.Ordinal)
                    && string.Equals(discovery.protocolVersion, UnityAiBridgeProtocol.ProtocolVersion, StringComparison.Ordinal)
                    && string.Equals(discovery.projectPath, Directory.GetCurrentDirectory(), StringComparison.Ordinal)
                    && discovery.pid == Process.GetCurrentProcess().Id;
            }
            catch (Exception exception)
            {
                Trace.TraceWarning($"Unity AI Bridge discovery file is stale or unreadable and will be rewritten: {exception.Message}");
                return false;
            }
        }

        private void DeleteDiscoveryFile()
        {
            var path = DiscoveryFilePath;
            if (File.Exists(path))
            {
                File.Delete(path);
            }

            var directory = Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(directory) && Directory.Exists(directory) && Directory.GetFileSystemEntries(directory).Length == 0)
            {
                Directory.Delete(directory);
            }
        }

        private static string GenerateToken()
        {
            var bytes = new byte[32];
            using (var generator = RandomNumberGenerator.Create())
            {
                generator.GetBytes(bytes);
            }

            return Convert.ToBase64String(bytes);
        }

        private static string GetDefaultDiscoveryFilePath()
        {
            return GetDiscoveryFilePath();
        }

        private void ThrowIfDisposed()
        {
            if (disposed)
            {
                throw new ObjectDisposedException(nameof(UnityAiBridgeTcpServer));
            }
        }
    }
}
