using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Linalab.UnityAiBridge.Editor.Ast;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    public static class UnityAiBridgeProtocol
    {
        public const int SchemaVersion = 1;
        public const string ProtocolVersion = "1";
        public const string BackendVersion = "0.1.0";

        public const string CommandPing = "ping";
        public const string CommandGetProtocolInfo = "get_protocol_info";
        public const string CommandGetSelectedFileContext = "get_selected_file_context";
        public const string CommandSubscribeEvents = "subscribe_events";
        public const string CommandTriggerCompile = "trigger_compile";
        public const string CommandStartLuxStream = "start_lux_stream";
        public const string CommandStopLuxStream = "stop_lux_stream";
        public const string CommandLuxStreamFrame = "lux_stream_frame";
        public const string CommandLuxInputEvent = "lux_input_event";
        public const string CommandLuxFrame = "lux_frame";
        public const string CommandReadAssetAst = "read_asset_ast";
        public const string CommandGetSelectionAst = "get_selection_ast";
        public const string CommandGetSceneAst = "get_scene_ast";

        public const string ErrorCodeUnknownCommand = "unknown_command";
        public const string ErrorCodeUnauthorized = "unauthorized";
        public const string ErrorCodeCommandAlreadyRegistered = "command_already_registered";
        public const string ErrorCodeRegistryNotReady = "registry_not_ready";
        public const string ErrorCodeInvalidParams = "invalid_params";
        public const string ErrorCodeArtifactWriteFailed = "artifact_write_failed";
        public const string ErrorCodePlayModeRequired = "play_mode_required";
        public const string ErrorCodeSingleFlightBusy = "single_flight_busy";
        public const string ErrorCodePolicyViolation = "policy_violation";

        private static readonly object RegisteredCommandLock = new object();
        private static readonly Dictionary<string, Func<UnityAiBridgeProtocolRequest, UnityAiBridgeProtocolResponse>> RegisteredCommandHandlers =
            new Dictionary<string, Func<UnityAiBridgeProtocolRequest, UnityAiBridgeProtocolResponse>>(StringComparer.Ordinal);
        private static readonly string[] LuxRegistryCommands =
        {
            "get_lux_context",
            "execute_lux_shell",
            "execute_lux_git",
            "run_lux_scene_smoke",
            "create_lux_scene_objects",
            "focus_lux_window",
            "get_lux_console_logs",
            "clear_lux_console",
            "find_lux_game_objects",
            "get_lux_hierarchy",
            "control_lux_play_mode",
            "capture_lux_screenshot",
            "simulate_lux_mouse_ui",
            "simulate_lux_keyboard",
            "simulate_lux_mouse_input",
            "record_lux_input",
            "replay_lux_input",
            "execute_lux_dynamic_code"
        };
        private static bool registryReady;
        private static string registryReadySource = string.Empty;

        public static readonly string[] SupportedCommands =
        {
            CommandPing,
            CommandGetProtocolInfo,
            CommandGetSelectedFileContext,
            CommandSubscribeEvents,
            CommandTriggerCompile,
            CommandStartLuxStream,
            CommandStopLuxStream,
            CommandLuxInputEvent,
            CommandReadAssetAst,
            CommandGetSelectionAst,
            CommandGetSceneAst
        };

        public static void RegisterCommand(string command, Func<UnityAiBridgeProtocolRequest, UnityAiBridgeProtocolResponse> handler)
        {
            if (string.IsNullOrWhiteSpace(command))
            {
                throw new ArgumentException("Command is required.", nameof(command));
            }

            if (handler == null)
            {
                throw new ArgumentNullException(nameof(handler));
            }

            lock (RegisteredCommandLock)
            {
                if (SupportedCommands.Contains(command) || RegisteredCommandHandlers.ContainsKey(command))
                {
                    throw new InvalidOperationException($"AI Bridge command is already registered: {command}");
                }

                RegisteredCommandHandlers.Add(command, handler);
            }
        }

        public static void MarkRegistryReady(string source)
        {
            lock (RegisteredCommandLock)
            {
                registryReady = true;
                registryReadySource = source ?? string.Empty;
            }
        }

        public static bool IsRegistryReady
        {
            get
            {
                lock (RegisteredCommandLock)
                {
                    return registryReady;
                }
            }
        }

        public static string RegistryReadySource
        {
            get
            {
                lock (RegisteredCommandLock)
                {
                    return registryReadySource;
                }
            }
        }

        public static void UnregisterCommand(string command)
        {
            if (string.IsNullOrWhiteSpace(command))
            {
                return;
            }

            lock (RegisteredCommandLock)
            {
                RegisteredCommandHandlers.Remove(command);
            }
        }

        public static string[] GetSupportedCommands()
        {
            lock (RegisteredCommandLock)
            {
                return SupportedCommands.Concat(RegisteredCommandHandlers.Keys.OrderBy(command => command, StringComparer.Ordinal)).ToArray();
            }
        }

        public static string[] GetRegisteredExtensionCommands()
        {
            lock (RegisteredCommandLock)
            {
                return RegisteredCommandHandlers.Keys.OrderBy(command => command, StringComparer.Ordinal).ToArray();
            }
        }

        public static void LogRegisteredCommands(string reason)
        {
            var commands = GetSupportedCommands();
            Debug.Log($"Lux Unity AI Bridge command registry {reason}: {commands.Length} command(s): {string.Join(", ", commands)}");
        }

        public static bool TryRefreshCommandRegistry(string reason)
        {
            var registrationType = Type.GetType("Linalab.Lux.Editor.LuxAiBridgeProtocolRegistration, Linalab.Lux.Editor");
            var registerMethod = registrationType?.GetMethod(
                "RegisterCommands",
                System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static);

            if (registerMethod == null)
            {
                Debug.LogWarning($"Lux Unity AI Bridge command registry refresh skipped during {reason}: Lux registration method is not loaded.");
                return false;
            }

            try
            {
                registerMethod.Invoke(null, null);
                return IsRegistryReady;
            }
            catch (Exception exception)
            {
                Debug.LogError($"Lux Unity AI Bridge command registry refresh failed during {reason}: {exception.Message}");
                return false;
            }
        }

        public static UnityAiBridgeProtocolResponse Handle(
            UnityAiBridgeProtocolRequest request,
            string expectedToken,
            Func<UnityAiBridgeProtocolRequest, UnityAiBridgeProtocolResponse> selectedFileContextHandler = null)
        {
            if (!IsAuthorized(request, expectedToken))
            {
                return CreateUnauthorizedResponse(request?.requestId);
            }

            switch (request.command)
            {
                case CommandPing:
                    return CreatePingResponse(request.requestId);

                case CommandGetProtocolInfo:
                    return CreateProtocolInfoResponse(request.requestId);

                case CommandGetSelectedFileContext:
                    if (selectedFileContextHandler == null)
                    {
                        return CreateSelectedFileContextResponse(request.requestId);
                    }

                    return selectedFileContextHandler(request) ?? CreateSelectedFileContextResponse(request.requestId);

                case CommandSubscribeEvents:
                case CommandTriggerCompile:
                    return CreateErrorResponse(
                        request.requestId,
                        ErrorCodeInvalidParams,
                        $"Command '{request.command}' requires TCP connection state and was not dispatched by the server.");

                case CommandStartLuxStream:
                    return HandleStartLuxStream(request);

                case CommandStopLuxStream:
                    return HandleStopLuxStream(request);

                case CommandLuxInputEvent:
                    return HandleLuxInputEvent(request);

                case CommandReadAssetAst:
                    return HandleReadAssetAst(request.requestId, request.@params);

                case CommandGetSelectionAst:
                    return HandleGetSelectionAst(request.requestId, request.@params);

                case CommandGetSceneAst:
                    return HandleGetSceneAst(request.requestId, request.@params);

                default:
                    if (!IsRegistryReady && IsLuxRegistryCommand(request.command))
                    {
                        TryRefreshCommandRegistry("request dispatch");
                    }

                    var registeredResponse = HandleRegisteredCommand(request);
                    if (registeredResponse != null)
                    {
                        return registeredResponse;
                    }

                    return !IsRegistryReady && IsLuxRegistryCommand(request.command)
                        ? CreateRegistryNotReadyResponse(request.requestId, request.command)
                        : CreateUnknownCommandResponse(request.requestId, request.command);
            }
        }

        public static UnityAiBridgeProtocolResponse CreateOkResponse(string requestId, UnityAiBridgeProtocolResponsePayload payload)
        {
            return new UnityAiBridgeProtocolResponse
            {
                schemaVersion = SchemaVersion,
                requestId = requestId,
                ok = true,
                payload = payload,
                capturedAtUtc = DateTime.UtcNow.ToString("O")
            };
        }

        public static UnityAiBridgeProtocolResponse CreateErrorResponse(string requestId, string errorCode, string errorMessage)
        {
            return new UnityAiBridgeProtocolResponse
            {
                schemaVersion = SchemaVersion,
                requestId = requestId,
                ok = false,
                errorCode = errorCode,
                errorMessage = errorMessage,
                capturedAtUtc = DateTime.UtcNow.ToString("O")
            };
        }

        private static UnityAiBridgeProtocolResponse HandleRegisteredCommand(UnityAiBridgeProtocolRequest request)
        {
            Func<UnityAiBridgeProtocolRequest, UnityAiBridgeProtocolResponse> handler;
            lock (RegisteredCommandLock)
            {
                if (!RegisteredCommandHandlers.TryGetValue(request.command ?? string.Empty, out handler))
                {
                    return null;
                }
            }

            return handler(request);
        }

        private static UnityAiBridgeProtocolResponse HandleStartLuxStream(UnityAiBridgeProtocolRequest request)
        {
            var parameters = request.@params;
            if (parameters == null || string.IsNullOrWhiteSpace(parameters.session_id))
            {
                return CreateErrorResponse(request.requestId, ErrorCodeInvalidParams, "Capture stream session_id is required.");
            }

            var width = parameters.width > 0 ? parameters.width : 1280;
            var height = parameters.height > 0 ? parameters.height : 720;
            var fps = parameters.fps > 0 ? parameters.fps : 30;
            var quality = parameters.quality > 0 ? parameters.quality : 75;

            string errorMessage;
            if (!TryInvokeStaticMethod(
                    "Linalab.UnityAiBridge.Editor.CaptureAgent",
                    "StartStream",
                    new[] { typeof(int), typeof(int), typeof(int), typeof(int), typeof(string) },
                    new object[] { width, height, fps, quality, parameters.session_id },
                    out errorMessage))
            {
                return CreateErrorResponse(request.requestId, ErrorCodeInvalidParams, errorMessage);
            }

            return CreateOkResponse(
                request.requestId,
                new UnityAiBridgeProtocolResponsePayload
                {
                    captureStreamResult = new UnityAiBridgeCaptureStreamPayload
                    {
                        action = "start",
                        active = true,
                        width = width,
                        height = height,
                        fps = fps,
                        quality = quality,
                        session_id = parameters.session_id
                    }
                });
        }

        private static UnityAiBridgeProtocolResponse HandleStopLuxStream(UnityAiBridgeProtocolRequest request)
        {
            var sessionId = request.@params?.session_id;
            if (string.IsNullOrWhiteSpace(sessionId))
            {
                return CreateErrorResponse(request.requestId, ErrorCodeInvalidParams, "Capture stream session_id is required.");
            }

            string errorMessage;
            if (!TryInvokeStaticMethod(
                    "Linalab.UnityAiBridge.Editor.CaptureAgent",
                    "StopStream",
                    new[] { typeof(string) },
                    new object[] { sessionId },
                    out errorMessage))
            {
                return CreateErrorResponse(request.requestId, ErrorCodeInvalidParams, errorMessage);
            }

            return CreateOkResponse(request.requestId, null);
        }

        private static UnityAiBridgeProtocolResponse HandleLuxInputEvent(UnityAiBridgeProtocolRequest request)
        {
            var parameters = request.@params;
            var eventType = parameters?.event_type;
            if (string.IsNullOrWhiteSpace(eventType))
            {
                eventType = parameters?.type;
            }

            if (parameters == null || string.IsNullOrWhiteSpace(eventType))
            {
                return CreateErrorResponse(request.requestId, ErrorCodeInvalidParams, "Input event type is required.");
            }

            string errorMessage;
            if (!TryInvokeStaticMethod(
                    "Linalab.UnityAiBridge.Editor.InputHandler",
                    "HandleInputEvent",
                    new[] { typeof(string), typeof(float), typeof(float), typeof(int), typeof(string), typeof(float), typeof(float) },
                    new object[] { eventType, parameters.x, parameters.y, parameters.button, parameters.key_code, parameters.inputScrollX, parameters.inputScrollY },
                    out errorMessage))
            {
                return CreateErrorResponse(request.requestId, ErrorCodeInvalidParams, errorMessage);
            }

            return CreateOkResponse(
                request.requestId,
                new UnityAiBridgeProtocolResponsePayload
                {
                    inputEventResult = new UnityAiBridgeInputEventPayload
                    {
                        session_id = parameters.session_id,
                        type = eventType,
                        x = parameters.x,
                        y = parameters.y,
                        button = parameters.button,
                        key_code = parameters.key_code,
                        handled = true
                    }
                });
        }

        private static bool TryInvokeStaticMethod(string typeName, string methodName, Type[] parameterTypes, object[] arguments, out string errorMessage)
        {
            errorMessage = null;

            var type = typeof(UnityAiBridgeProtocol).Assembly.GetType(typeName);
            if (type == null)
            {
                errorMessage = $"Required bridge type is not available: {typeName}.";
                return false;
            }

            var method = type.GetMethod(
                methodName,
                System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static,
                null,
                parameterTypes,
                null);

            if (method == null)
            {
                errorMessage = $"Required bridge method is not available: {typeName}.{methodName}.";
                return false;
            }

            try
            {
                method.Invoke(null, arguments);
                return true;
            }
            catch (Exception exception)
            {
                errorMessage = exception.InnerException?.Message ?? exception.Message;
                return false;
            }
        }

        private static bool IsLuxRegistryCommand(string command)
        {
            return !string.IsNullOrEmpty(command) && LuxRegistryCommands.Contains(command);
        }

        private static bool IsAuthorized(UnityAiBridgeProtocolRequest request, string expectedToken)
        {
            return request != null
                && !string.IsNullOrEmpty(expectedToken)
                && !string.IsNullOrEmpty(request.token)
                && string.Equals(request.token, expectedToken, StringComparison.Ordinal);
        }

        private static UnityAiBridgeProtocolResponse CreatePingResponse(string requestId)
        {
            return new UnityAiBridgeProtocolResponse
            {
                schemaVersion = SchemaVersion,
                requestId = requestId,
                ok = true,
                payload = new UnityAiBridgeProtocolResponsePayload
                {
                    ping = new UnityAiBridgePingPayload
                    {
                        status = "ok"
                    }
                },
                capturedAtUtc = DateTime.UtcNow.ToString("O")
            };
        }

        private static UnityAiBridgeProtocolResponse CreateProtocolInfoResponse(string requestId)
        {
            return new UnityAiBridgeProtocolResponse
            {
                schemaVersion = SchemaVersion,
                requestId = requestId,
                ok = true,
                payload = new UnityAiBridgeProtocolResponsePayload
                {
                    protocolInfo = new UnityAiBridgeProtocolInfoPayload
                    {
                        protocolVersion = ProtocolVersion,
                        backendVersion = BackendVersion,
                        commands = GetSupportedCommands()
                    }
                },
                capturedAtUtc = DateTime.UtcNow.ToString("O")
            };
        }

        private static UnityAiBridgeProtocolResponse CreateSelectedFileContextResponse(string requestId)
        {
            return new UnityAiBridgeProtocolResponse
            {
                schemaVersion = SchemaVersion,
                requestId = requestId,
                ok = true,
                payload = new UnityAiBridgeProtocolResponsePayload
                {
                    selectedFileContext = UnityAiSelectedFileContextCache.CreatePayload()
                },
                capturedAtUtc = DateTime.UtcNow.ToString("O")
            };
        }

        private static UnityAiBridgeProtocolResponse HandleReadAssetAst(string requestId, UnityAiBridgeProtocolRequestParameters parameters)
        {
            if (parameters == null || string.IsNullOrWhiteSpace(parameters.assetPath))
            {
                return CreateErrorResponse(requestId, ErrorCodeInvalidParams, "assetPath is required for read_asset_ast.");
            }

            try
            {
                return CreateOkResponse(
                    requestId,
                    new UnityAiBridgeProtocolResponsePayload
                    {
                        assetAst = UnityAstFileReader.ReadAsset(parameters.assetPath)
                    });
            }
            catch (Exception exception)
            {
                return CreateErrorResponse(requestId, ErrorCodeInvalidParams, exception.Message);
            }
        }

        private static UnityAiBridgeProtocolResponse HandleGetSelectionAst(string requestId, UnityAiBridgeProtocolRequestParameters parameters)
        {
            var depth = UnityAstConstants.NormalizeDepth(parameters == null ? UnityAstConstants.DefaultMaxDepth : parameters.astDepth);
            var includeComponents = parameters == null || parameters.astIncludeComponents;

            try
            {
                return CreateOkResponse(
                    requestId,
                    new UnityAiBridgeProtocolResponsePayload
                    {
                        selectionAst = UnityAstSelectionReader.ReadSelection(depth, includeComponents)
                    });
            }
            catch (Exception exception)
            {
                return CreateErrorResponse(requestId, ErrorCodeInvalidParams, exception.Message);
            }
        }

        private static UnityAiBridgeProtocolResponse HandleGetSceneAst(string requestId, UnityAiBridgeProtocolRequestParameters parameters)
        {
            var depth = UnityAstConstants.NormalizeDepth(parameters == null ? UnityAstConstants.DefaultMaxDepth : parameters.astDepth);
            var rootOnly = parameters != null && parameters.astRootOnly;

            try
            {
                return CreateOkResponse(
                    requestId,
                    new UnityAiBridgeProtocolResponsePayload
                    {
                        sceneAst = UnityAstSceneReader.ReadScene(rootOnly, depth)
                    });
            }
            catch (Exception exception)
            {
                return CreateErrorResponse(requestId, ErrorCodeInvalidParams, exception.Message);
            }
        }

        private static UnityAiBridgeProtocolResponse CreateUnknownCommandResponse(string requestId, string command)
        {
            return new UnityAiBridgeProtocolResponse
            {
                schemaVersion = SchemaVersion,
                requestId = requestId,
                ok = false,
                errorCode = ErrorCodeUnknownCommand,
                errorMessage = string.IsNullOrEmpty(command)
                    ? "Unknown command."
                    : $"Unknown command: {command}",
                capturedAtUtc = DateTime.UtcNow.ToString("O")
            };
        }

        private static UnityAiBridgeProtocolResponse CreateRegistryNotReadyResponse(string requestId, string command)
        {
            return new UnityAiBridgeProtocolResponse
            {
                schemaVersion = SchemaVersion,
                requestId = requestId,
                ok = false,
                errorCode = ErrorCodeRegistryNotReady,
                errorMessage = string.IsNullOrEmpty(command)
                    ? "Command registry is not ready."
                    : $"Command registry is not ready for command: {command}",
                capturedAtUtc = DateTime.UtcNow.ToString("O")
            };
        }

        private static UnityAiBridgeProtocolResponse CreateUnauthorizedResponse(string requestId)
        {
            return new UnityAiBridgeProtocolResponse
            {
                schemaVersion = SchemaVersion,
                requestId = requestId,
                ok = false,
                errorCode = ErrorCodeUnauthorized,
                errorMessage = "Invalid token.",
                capturedAtUtc = DateTime.UtcNow.ToString("O")
            };
        }
    }

    [Serializable]
    public sealed class UnityAiBridgeProtocolRequest
    {
        public int schemaVersion;
        public string requestId;
        public string command;
        public string token;
        public UnityAiBridgeProtocolRequestParameters @params;
    }

    [Serializable]
    public sealed class UnityAiBridgeProtocolRequestParameters
    {
        public string commandText;
        public string gitArguments;
        public string workingDirectory;
        public string actor;
        public bool approvalGranted;
        public string scenePath;
        public int sceneSmokeObjectCount;
        public string findGameObjectsSearchMode;
        public string findGameObjectsName;
        public string findGameObjectsRegex;
        public string findGameObjectsPath;
        public string findGameObjectsComponent;
        public string findGameObjectsTag;
        public string findGameObjectsLayer;
        public string findGameObjectsActiveState;
        public int findGameObjectsInlineLimit;
        public string hierarchySearchMode;
        public bool hierarchyAll;
        public string hierarchyRootPath;
        public bool hierarchyUseSelection;
        public string playModeAction;
        public string screenshotCaptureMode;
        public bool screenshotAnnotateElements;
        public bool screenshotElementsOnly;
        public string mouseUiAction;
        public float mouseUiX;
        public float mouseUiY;
        public int mouseUiDurationMs;
        public string inputAction;
        public string inputKey;
        public string inputButton;
        public int inputDurationMs;
        public float inputDeltaX;
        public float inputDeltaY;
        public float inputScrollX;
        public float inputScrollY;
        public int inputSteps;
        public string inputFilePath;
        public string dynamicCode;
        public string eventTypes;
        public string testPlatform;
        public string testResults;
        public int width;
        public int height;
        public int fps;
        public int quality;
        public string session_id;
        public string event_type;
        public string type;
        public float x;
        public float y;
        public int button;
        public string key_code;
        public string assetPath;
        public int astDepth;
        public bool astIncludeComponents;
        public bool astRootOnly;
        public string astOptions;
    }

    [Serializable]
    public sealed class UnityAiBridgeProtocolResponse
    {
        public int schemaVersion;
        public string requestId;
        public bool ok;
        public string errorCode;
        public string errorMessage;
        public UnityAiBridgeProtocolResponsePayload payload;
        public string capturedAtUtc;
    }

    [Serializable]
        public sealed class UnityAiBridgeProtocolResponsePayload
        {
            public UnityAiBridgePingPayload ping;
            public UnityAiBridgeProtocolInfoPayload protocolInfo;
            public UnityAiBridgeSelectedFileContextPayload selectedFileContext;
            public UnityAiBridgeLuxContextPayload luxContext;
            public UnityAiBridgeLuxAutomationResultPayload luxAutomationResult;
            public UnityAiBridgeFocusWindowPayload focusWindowResult;
            public UnityAiBridgeConsoleLogsPayload consoleLogs;
            public UnityAiBridgeConsoleClearPayload consoleClearResult;
            public UnityAiBridgeFindGameObjectsPayload findGameObjectsResult;
            public UnityAiBridgeGetHierarchyPayload getHierarchyResult;
            public UnityAiBridgePlayModeStatePayload playModeState;
            public UnityAiBridgeScreenshotPayload screenshotResult;
            public UnityAiBridgeMouseUiPayload mouseUiResult;
            public UnityAiBridgeInputSimulationPayload inputSimulationResult;
            public UnityAiBridgeInputRecordPayload inputRecordResult;
            public UnityAiBridgeInputReplayPayload inputReplayResult;
            public UnityAiBridgeDynamicCodeResultPayload dynamicCodeResult;
            public UnityAiBridgeCompileResultPayload compileResult;
            public UnityAiBridgeTestRunResultPayload testRunResult;
            public UnityAiBridgeCaptureStreamPayload captureStreamResult;
            public UnityAiBridgeInputEventPayload inputEventResult;
            public UnityAstReadResult assetAst;
            public UnityAstSelectionAstPayload selectionAst;
            public UnityAstScene sceneAst;
        }

    [Serializable]
    public sealed class UnityAiBridgeCaptureStreamPayload
    {
        public string action;
        public bool active;
        public int width;
        public int height;
        public int fps;
        public int quality;
        public string session_id;
    }

    [Serializable]
    public sealed class UnityAiBridgeStreamFramePayload
    {
        public string session_id;
        public string frame;
        public long sequence;
        public string timestamp;
    }

    [Serializable]
    public sealed class UnityAiBridgeInputEventPayload
    {
        public string session_id;
        public string type;
        public float x;
        public float y;
        public int button;
        public string key_code;
        public bool handled;
    }

    [Serializable]
    public sealed class UnityAiBridgeHierarchyNode
    {
        public string name;
        public string path;
        public bool active;
        public string tag;
        public string layer;
        public string[] componentTypeNames;
        public UnityAiBridgeHierarchyNode[] children;
    }

    [Serializable]
    public sealed class UnityAiBridgeHierarchyActiveScene
    {
        public string name;
        public string path;
    }

    [Serializable]
    public sealed class UnityAiBridgeHierarchyFilters
    {
        public bool all;
        public string rootPath;
        public bool useSelection;
    }

    [Serializable]
    public sealed class UnityAiBridgeGetHierarchyPayload
    {
        public string filePath;
        public long fileSizeBytes;
        public int rootCount;
        public int nodeCount;
        public UnityAiBridgeHierarchyActiveScene activeScene;
        public UnityAiBridgeHierarchyFilters filters;
    }

    [Serializable]
    public sealed class UnityAiBridgePlayModeStatePayload
    {
        public string action;
        public bool isPlaying;
        public bool isPaused;
        public bool transitionRequested;
    }

    [Serializable]
    public sealed class UnityAiBridgeScreenshotElement
    {
        public string documentName;
        public string label;
        public string name;
        public string type;
        public string typeName;
        public string path;
        public float x;
        public float y;
        public float width;
        public float height;
        public bool visible;
        public bool enabled;
        public string pickingMode;
        public string interaction;
        public float simX;
        public float simY;
        public float boundsMinX;
        public float boundsMaxX;
        public float boundsMinY;
        public float boundsMaxY;
        public int sortingOrder;
        public int siblingIndex;
        public string coordinateSystem;
        public float resolutionScale;
        public float yOffset;
    }

    [Serializable]
    public sealed class UnityAiBridgeMouseUiPayload
    {
        public string action;
        public float x;
        public float y;
        public bool success;
        public string targetName;
        public string targetPath;
        public int raycastCount;
        public bool dragActive;
    }

    [Serializable]
    public sealed class UnityAiBridgeScreenshotPayload
    {
        public string filePath;
        public long fileSizeBytes;
        public string mediaType;
        public string captureMode;
        public bool annotated;
        public bool elementsOnly;
        public bool screenshotSaved;
        public int annotationCount;
        public UnityAiBridgeScreenshotElement[] annotatedElements;
    }

    [Serializable]
    public sealed class UnityAiBridgeInputSimulationPayload
    {
        public string device;
        public string action;
        public string key;
        public string button;
        public float deltaX;
        public float deltaY;
        public float scrollX;
        public float scrollY;
        public string[] heldKeys;
        public string[] heldButtons;
        public int queuedActions;
    }

    [Serializable]
    public sealed class UnityAiBridgeInputRecordPayload
    {
        public string action;
        public bool active;
        public int frameCount;
        public string filePath;
        public long fileSizeBytes;
        public string mediaType;
        public string message;
    }

    [Serializable]
    public sealed class UnityAiBridgeInputReplayPayload
    {
        public string action;
        public bool active;
        public string filePath;
        public int frameCount;
        public int replayedFrameCount;
        public bool completed;
        public string message;
    }

    [Serializable]
    public sealed class UnityAiBridgeDynamicCodeDiagnostic
    {
        public string id;
        public string severity;
        public string message;
        public int line;
        public int column;
    }

    [Serializable]
    public sealed class UnityAiBridgeDynamicCodeResultPayload
    {
        public bool success;
        public string action;
        public string result;
        public string resultType;
        public string message;
        public UnityAiBridgeDynamicCodeDiagnostic[] diagnostics;
        public UnityAiBridgeConsoleLogEntry[] logs;
        public long elapsedTimeMs;
    }

    [Serializable]
    public sealed class UnityAiBridgeCompileResultPayload
    {
        public bool ok;
        public int error_count;
        public string message;
        public string timestamp_utc;
    }

    [Serializable]
    public sealed class UnityAiBridgeTestRunResultPayload
    {
        public bool ok;
        public string status;
        public string testId;
        public string testPlatform;
        public string testResults;
        public string message;
    }

    [Serializable]
    public sealed class UnityAiBridgeHierarchyExport
    {
        public int schemaVersion;
        public string requestId;
        public int rootCount;
        public int nodeCount;
        public UnityAiBridgeHierarchyActiveScene activeScene;
        public UnityAiBridgeHierarchyFilters filters;
        public UnityAiBridgeHierarchyNode[] roots;
    }

    [Serializable]
    public sealed class UnityAiBridgePingPayload
    {
        public string status;
    }

    [Serializable]
    public sealed class UnityAiBridgeProtocolInfoPayload
    {
        public string protocolVersion;
        public string backendVersion;
        public string[] commands;
    }

    [Serializable]
    public sealed class UnityAiBridgeSelectedFileContextPayload
    {
        public string projectName;
        public string projectPath;
        public string unityVersion;
        public string selectionCapturedAtUtc;
        public int selectionCount;
        public UnityAiSelectedFileMetadata[] selectedFiles;
    }

    [Serializable]
    public sealed class UnityAiBridgeLuxContextPayload
    {
        public string packageName;
        public string protocolSurface;
        public string projectPath;
        public string unityVersion;
        public string platform;
        public string remotePhase;
        public string videoTransport;
        public string signalingTransport;
        public string controlTransport;
        public string permissionModel;
        public bool includesIosClientImplementation;
        public string[] automationBlockedTokens;
        public string[] automationApprovalTokens;
        public int auditEntryCount;
    }

    [Serializable]
    public sealed class UnityAiBridgeLuxAutomationResultPayload
    {
        public bool allowed;
        public bool success;
        public int exitCode;
        public string output;
        public string error;
        public string message;
    }

    [Serializable]
    public sealed class UnityAiBridgeFocusWindowPayload
    {
        public bool focused;
    }

    [Serializable]
    public sealed class UnityAiBridgeConsoleLogEntry
    {
        public string level;
        public string message;
        public string stackTrace;
        public string timestampUtc;
    }

    [Serializable]
    public sealed class UnityAiBridgeConsoleLogsPayload
    {
        public int totalCount;
        public int displayedCount;
        public UnityAiBridgeConsoleLogEntry[] consoleLogs;
    }

    [Serializable]
    public sealed class UnityAiBridgeConsoleClearPayload
    {
        public int beforeCount;
        public int afterCount;
    }

    [Serializable]
    public sealed class UnityAiBridgeFindGameObjectsEntry
    {
        public string name;
        public string hierarchyPath;
        public string sceneName;
        public string scenePath;
        public bool activeSelf;
        public bool activeInHierarchy;
        public string tag;
        public int layer;
        public string layerName;
        public int instanceId;
        public string[] componentTypeNames;
    }

    [Serializable]
    public sealed class UnityAiBridgeFindGameObjectsPayload
    {
        public string searchMode;
        public int totalMatchCount;
        public int returnedCount;
        public bool truncated;
        public string outputFilePath;
        public long fileSizeBytes;
        public UnityAiBridgeFindGameObjectsEntry[] gameObjects;
    }

    [Serializable]
    internal sealed class UnityAiProjectContextMetadata
    {
        public string projectName;
        public string projectPath;
        public string unityVersion;
    }

    internal static class UnityAiSelectedFileContextCache
    {
        private static readonly object CacheLock = new object();
        private static UnityAiProjectContextMetadata cachedProject = CreateEmptyProjectMetadata();

        [InitializeOnLoadMethod]
        private static void Initialize()
        {
            RefreshProjectMetadataFromMainThread();
            UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();
        }

        public static UnityAiBridgeSelectedFileContextPayload CreatePayload()
        {
            var project = GetProjectMetadataCopy();
            var snapshot = UnityAiSelectionSnapshotCollector.GetCachedSnapshot() ?? new UnityAiSelectionSnapshot
            {
                capturedAtUtc = string.Empty,
                selectionCount = 0,
                selectedFiles = new UnityAiSelectedFileMetadata[0]
            };

            return new UnityAiBridgeSelectedFileContextPayload
            {
                projectName = project.projectName,
                projectPath = project.projectPath,
                unityVersion = project.unityVersion,
                selectionCapturedAtUtc = snapshot.capturedAtUtc ?? string.Empty,
                selectionCount = snapshot.selectionCount,
                selectedFiles = snapshot.selectedFiles ?? new UnityAiSelectedFileMetadata[0]
            };
        }

        private static void RefreshProjectMetadataFromMainThread()
        {
            var project = new UnityAiProjectContextMetadata
            {
                projectName = Application.productName ?? string.Empty,
                projectPath = Directory.GetCurrentDirectory(),
                unityVersion = Application.unityVersion ?? string.Empty
            };

            lock (CacheLock)
            {
                cachedProject = project;
            }
        }

        private static UnityAiProjectContextMetadata GetProjectMetadataCopy()
        {
            lock (CacheLock)
            {
                return new UnityAiProjectContextMetadata
                {
                    projectName = cachedProject.projectName,
                    projectPath = cachedProject.projectPath,
                    unityVersion = cachedProject.unityVersion
                };
            }
        }

        private static UnityAiProjectContextMetadata CreateEmptyProjectMetadata()
        {
            return new UnityAiProjectContextMetadata
            {
                projectName = string.Empty,
                projectPath = string.Empty,
                unityVersion = string.Empty
            };
        }
    }
}
