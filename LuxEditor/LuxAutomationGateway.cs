using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Collections.Generic;
using System.Threading;
using System.Text.RegularExpressions;
using Linalab.UnityAiBridge.Editor;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.SceneManagement;
using UnityEngine.UIElements;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using InputMouseButton = UnityEngine.InputSystem.LowLevel.MouseButton;

namespace Linalab.Lux.Editor
{
    public readonly struct LuxAutomationResult
    {
        public LuxAutomationResult(bool allowed, bool success, int exitCode, string output, string error, string message)
        {
            Allowed = allowed;
            Success = success;
            ExitCode = exitCode;
            Output = output ?? string.Empty;
            Error = error ?? string.Empty;
            Message = message ?? string.Empty;
        }

        public bool Allowed { get; }
        public bool Success { get; }
        public int ExitCode { get; }
        public string Output { get; }
        public string Error { get; }
        public string Message { get; }
    }

    public sealed class LuxAutomationGateway
    {
        public const string DefaultActor = "ai";

        readonly LuxAutomationPolicy _policy;
        readonly LuxAutomationAuditLog _auditLog;

        public LuxAutomationGateway(LuxAutomationPolicy policy = null, LuxAutomationAuditLog auditLog = null)
        {
            _policy = policy ?? new LuxAutomationPolicy();
            _auditLog = auditLog ?? new LuxAutomationAuditLog();
        }

        public LuxAutomationPolicy Policy => _policy;
        public LuxAutomationAuditLog AuditLog => _auditLog;

        public LuxAutomationResult ExecuteShellCommand(string command, string workingDirectory, string actor = DefaultActor, bool approvalGranted = false)
        {
            return ExecuteProcess("shell", GetShellExecutable(), GetShellArguments(command ?? string.Empty), command, workingDirectory, actor, approvalGranted);
        }

        static string GetShellExecutable()
        {
            if (Application.platform == RuntimePlatform.WindowsEditor)
            {
                return "cmd.exe";
            }

            var zsh = "/bin/zsh";
            if (File.Exists(zsh)) return zsh;

            var bash = "/bin/bash";
            if (File.Exists(bash)) return bash;

            return "/bin/sh";
        }

        static string[] GetShellArguments(string command)
        {
            if (Application.platform == RuntimePlatform.WindowsEditor)
            {
                return new[] { "/c", command };
            }

            return new[] { "-lc", command };
        }
        public LuxAutomationResult ExecuteGitCommand(string arguments, string workingDirectory, string actor = DefaultActor, bool approvalGranted = false)
        {
            var command = string.IsNullOrWhiteSpace(arguments) ? "git" : $"git {arguments}";
            return ExecuteProcess("git", "git", SplitArguments(arguments), command, workingDirectory, actor, approvalGranted);
        }

        LuxAutomationResult ExecuteProcess(
            string commandKind,
            string executable,
            string[] arguments,
            string commandForPolicy,
            string workingDirectory,
            string actor,
            bool approvalGranted)
        {
            var decision = _policy.Evaluate(commandForPolicy);
            if (decision.Kind == LuxAutomationDecisionKind.Block)
            {
                return Deny(commandKind, commandForPolicy, workingDirectory, actor, decision.Reason);
            }

            if (decision.Kind == LuxAutomationDecisionKind.RequireApproval && !approvalGranted)
            {
                return Deny(commandKind, commandForPolicy, workingDirectory, actor, decision.Reason);
            }

            var normalizedWorkingDirectory = string.IsNullOrWhiteSpace(workingDirectory)
                ? Directory.GetCurrentDirectory()
                : workingDirectory;

            try
            {
                using var process = new Process();
                process.StartInfo = new ProcessStartInfo
                {
                    FileName = executable,
                    WorkingDirectory = normalizedWorkingDirectory,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    StandardOutputEncoding = Encoding.UTF8,
                    StandardErrorEncoding = Encoding.UTF8
                };

                foreach (var argument in arguments ?? Array.Empty<string>())
                {
                    process.StartInfo.ArgumentList.Add(argument ?? string.Empty);
                }

                process.Start();
                var output = process.StandardOutput.ReadToEnd();
                var error = process.StandardError.ReadToEnd();
                process.WaitForExit();

                var success = process.ExitCode == 0;
                var message = success ? "Command completed." : "Command failed.";
                _auditLog.Record(actor, commandKind, commandForPolicy, normalizedWorkingDirectory, true, success, message);
                return new LuxAutomationResult(true, success, process.ExitCode, output, error, message);
            }
            catch (Exception exception)
            {
                _auditLog.Record(actor, commandKind, commandForPolicy, normalizedWorkingDirectory, true, false, exception.Message);
                return new LuxAutomationResult(true, false, -1, string.Empty, string.Empty, exception.Message);
            }
        }

        LuxAutomationResult Deny(string commandKind, string command, string targetContext, string actor, string reason)
        {
            _auditLog.Record(actor, commandKind, command, targetContext, false, false, reason);
            return new LuxAutomationResult(false, false, -1, string.Empty, string.Empty, reason);
        }

        static string[] SplitArguments(string arguments)
        {
            if (string.IsNullOrWhiteSpace(arguments))
            {
                return Array.Empty<string>();
            }

            return arguments.Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
        }
    }

    public static partial class LuxAiBridgeProtocolRegistration
    {
        public const string CommandGetLuxContext = "get_lux_context";
        public const string CommandExecuteLuxShell = "execute_lux_shell";
        public const string CommandExecuteLuxGit = "execute_lux_git";
        public const string CommandRunLuxSceneSmoke = "run_lux_scene_smoke";
        public const string CommandCreateLuxSceneObjects = "create_lux_scene_objects";
        public const string CommandFocusLuxWindow = "focus_lux_window";
        public const string CommandGetLuxConsoleLogs = "get_lux_console_logs";
        public const string CommandClearLuxConsole = "clear_lux_console";
        public const string CommandFindLuxGameObjects = "find_lux_game_objects";
        public const string CommandGetLuxHierarchy = "get_lux_hierarchy";
        public const string CommandControlLuxPlayMode = "control_lux_play_mode";
        public const string CommandCaptureLuxScreenshot = "capture_lux_screenshot";
        public const string CommandSimulateLuxMouseUi = "simulate_lux_mouse_ui";
        public const string CommandSimulateLuxKeyboard = "simulate_lux_keyboard";
        public const string CommandSimulateLuxMouseInput = "simulate_lux_mouse_input";
        public const string CommandRecordLuxInput = "record_lux_input";
        public const string CommandReplayLuxInput = "replay_lux_input";
        public const string CommandExecuteLuxDynamicCode = "execute_lux_dynamic_code";

        static readonly LuxAutomationGateway AutomationGateway = new LuxAutomationGateway();
        static PointerEventData ActiveMouseUiDragEvent;
        static GameObject ActiveMouseUiDragTarget;
        static bool ActiveMouseUiDragStarted;

        [InitializeOnLoadMethod]
        public static void RegisterCommands()
        {
            RebuildCommandRegistry("InitializeOnLoad");
        }

        [MenuItem("Tools/Linalab/Lux/AI Bridge/Rebuild Command Registry")]
        public static void RebuildCommandRegistryMenu()
        {
            RebuildCommandRegistry("menu rebuild");
        }

        public static void RebuildCommandRegistry(string reason)
        {
            RegisterOrReplace(CommandGetLuxContext, CreateContextResponse);
            RegisterOrReplace(CommandExecuteLuxShell, ExecuteShellCommand);
            RegisterOrReplace(CommandExecuteLuxGit, ExecuteGitCommand);
            RegisterOrReplace(CommandRunLuxSceneSmoke, RunSceneSmoke);
            RegisterOrReplace(CommandCreateLuxSceneObjects, CreateSceneObjects);
            RegisterOrReplace(CommandFocusLuxWindow, FocusWindow);
            RegisterOrReplace(CommandGetLuxConsoleLogs, GetConsoleLogs);
            RegisterOrReplace(CommandClearLuxConsole, ClearConsole);
            RegisterOrReplace(CommandFindLuxGameObjects, FindGameObjects);
            RegisterOrReplace(CommandGetLuxHierarchy, GetHierarchy);
            RegisterOrReplace(CommandControlLuxPlayMode, ControlPlayMode);
            RegisterOrReplace(CommandCaptureLuxScreenshot, CaptureScreenshot);
            RegisterOrReplace(CommandSimulateLuxMouseUi, SimulateMouseUi);
            RegisterOrReplace(CommandSimulateLuxKeyboard, SimulateKeyboard);
            RegisterOrReplace(CommandSimulateLuxMouseInput, SimulateMouseInput);
            RegisterOrReplace(CommandRecordLuxInput, RecordInput);
            RegisterOrReplace(CommandReplayLuxInput, ReplayInput);
            RegisterOrReplace(CommandExecuteLuxDynamicCode, ExecuteDynamicCode);
            UnityAiBridgeProtocol.MarkRegistryReady(reason);
            UnityAiBridgeProtocol.LogRegisteredCommands(reason);
        }

        static void RegisterOrReplace(string command, Func<UnityAiBridgeProtocolRequest, UnityAiBridgeProtocolResponse> handler)
        {
            UnityAiBridgeProtocol.UnregisterCommand(command);
            UnityAiBridgeProtocol.RegisterCommand(command, handler);
            UnityEngine.Debug.Log($"Lux Unity AI Bridge registered command: {command}");
        }

        static UnityAiBridgeProtocolResponse CreateContextResponse(UnityAiBridgeProtocolRequest request)
        {
            return UnityAiBridgeProtocol.CreateOkResponse(
                request.requestId,
                new UnityAiBridgeProtocolResponsePayload
                {
                    luxContext = new UnityAiBridgeLuxContextPayload
                    {
                        packageName = "com.linalab.lux",
                        protocolSurface = "ai-bridge-tcp",
                        projectPath = GetProjectRoot(),
                        unityVersion = Application.unityVersion ?? string.Empty,
                        platform = Application.platform.ToString(),
                        remotePhase = LuxRemoteGatewayPlan.Phase,
                        videoTransport = LuxRemoteGatewayPlan.VideoTransport,
                        signalingTransport = LuxRemoteGatewayPlan.SignalingTransport,
                        controlTransport = LuxRemoteGatewayPlan.ControlTransport,
                        permissionModel = LuxRemoteGatewayPlan.PermissionModel,
                        includesIosClientImplementation = LuxRemoteGatewayPlan.IncludesIosClientImplementation,
                        automationBlockedTokens = AutomationGateway.Policy.BlockedTokens.ToArray(),
                        automationApprovalTokens = AutomationGateway.Policy.ApprovalTokens.ToArray(),
                        auditEntryCount = AutomationGateway.AuditLog.Entries.Count
                    }
                });
        }

        static UnityAiBridgeProtocolResponse ExecuteShellCommand(UnityAiBridgeProtocolRequest request)
        {
            var parameters = request.@params ?? new UnityAiBridgeProtocolRequestParameters();
            var result = AutomationGateway.ExecuteShellCommand(
                parameters.commandText,
                GetWorkingDirectory(parameters.workingDirectory),
                GetActor(parameters.actor),
                parameters.approvalGranted);

            return CreateAutomationResponse(request.requestId, result);
        }

        static UnityAiBridgeProtocolResponse ExecuteGitCommand(UnityAiBridgeProtocolRequest request)
        {
            var parameters = request.@params ?? new UnityAiBridgeProtocolRequestParameters();
            var arguments = string.IsNullOrWhiteSpace(parameters.gitArguments) ? parameters.commandText : parameters.gitArguments;
            var result = AutomationGateway.ExecuteGitCommand(
                arguments,
                GetWorkingDirectory(parameters.workingDirectory),
                GetActor(parameters.actor),
                parameters.approvalGranted);

            return CreateAutomationResponse(request.requestId, result);
        }


        static UnityAiBridgeProtocolResponse CompileLuxProject(UnityAiBridgeProtocolRequest request)
        {
            var success = false;
            var errorCount = 0;
            var message = string.Empty;

            try
            {
                AssetDatabase.Refresh(ImportAssetOptions.ForceUpdate);
                success = !EditorUtility.scriptCompilationFailed;
                errorCount = CountCompilerErrors();
                if (errorCount > 0)
                {
                    success = false;
                }
                else if (!success)
                {
                    UnityEngine.Debug.LogWarning("LuxAutomationGateway: scriptCompilationFailed is true but no compiler errors found in console logs.");
                }
                message = success
                    ? "Compilation succeeded."
                    : $"Script compilation failed with {errorCount} error(s). Check Unity console for details.";
            }
            catch (Exception exception)
            {
                success = false;
                message = $"Compilation threw an exception: {exception.Message}";
            }

            return UnityAiBridgeProtocol.CreateOkResponse(
                request.requestId,
                new UnityAiBridgeProtocolResponsePayload
                {
                    compileResult = new UnityAiBridgeCompileResultPayload
                    {
                        ok = success,
                        error_count = errorCount,
                        message = message,
                        timestamp_utc = DateTime.UtcNow.ToString("O")
                    }
                });
        }

        static int CountCompilerErrors()
        {
            var count = 0;
            foreach (var log in LuxUnityContext.GetRecentLogsSnapshot())
            {
                if (!string.Equals(log.Type, "Error", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                if (LuxCompileEventBroadcaster.CompilerErrorPattern.Match(log.Message).Success
                    || LuxCompileEventBroadcaster.CompilerErrorPattern.Match(log.StackTrace).Success)
                {
                    count++;
                }
            }
            return count;
        }

        static UnityAiBridgeProtocolResponse RunLuxTests(UnityAiBridgeProtocolRequest request)
        {
            var parameters = request.@params ?? new UnityAiBridgeProtocolRequestParameters();
            var testPlatform = string.IsNullOrWhiteSpace(parameters.testPlatform) ? "EditMode" : parameters.testPlatform;

            try
            {
                var testId = StartLuxTestRun(testPlatform);
                return UnityAiBridgeProtocol.CreateOkResponse(
                    request.requestId,
                    new UnityAiBridgeProtocolResponsePayload
                    {
                        testRunResult = new UnityAiBridgeTestRunResultPayload
                        {
                            ok = true,
                            status = "started",
                            testId = testId,
                            testPlatform = testPlatform,
                            testResults = parameters.testResults,
                            message = "Unity Test Runner started."
                        }
                    });
            }
            catch (Exception exception)
            {
                return UnityAiBridgeProtocol.CreateErrorResponse(
                    request.requestId,
                    UnityAiBridgeProtocol.ErrorCodeInvalidParams,
                    $"Failed to start Unity Test Runner: {exception.Message}");
            }
        }

        static string StartLuxTestRun(string testPlatform)
        {
            var apiType = FindLuxEditorType("UnityEditor.TestTools.TestRunner.Api.TestRunnerApi");
            var filterType = FindLuxEditorType("UnityEditor.TestTools.TestRunner.Api.Filter");
            var settingsType = FindLuxEditorType("UnityEditor.TestTools.TestRunner.Api.ExecutionSettings");
            var testModeType = FindLuxEditorType("UnityEditor.TestTools.TestRunner.Api.TestMode");
            if (apiType == null || filterType == null || settingsType == null || testModeType == null)
            {
                throw new InvalidOperationException("Unity Test Runner API is not available in this Editor session.");
            }

            var normalizedPlatform = string.Equals(testPlatform, "PlayMode", StringComparison.OrdinalIgnoreCase)
                ? "PlayMode"
                : "EditMode";
            var filter = Activator.CreateInstance(filterType);
            var testMode = Enum.Parse(testModeType, normalizedPlatform);
            filterType.GetField("testMode")?.SetValue(filter, testMode);
            filterType.GetProperty("testMode")?.SetValue(filter, testMode, null);

            var settings = CreateLuxTestExecutionSettings(settingsType, filterType, filter);
            var api = Activator.CreateInstance(apiType);
            var executeMethod = apiType.GetMethod("Execute", new[] { settingsType });
            if (executeMethod == null)
            {
                throw new InvalidOperationException("Unity Test Runner Execute method was not found.");
            }

            var result = executeMethod.Invoke(api, new[] { settings });
            return string.IsNullOrWhiteSpace(result?.ToString()) ? Guid.NewGuid().ToString("N") : result.ToString();
        }

        static object CreateLuxTestExecutionSettings(Type settingsType, Type filterType, object filter)
        {
            foreach (var constructor in settingsType.GetConstructors())
            {
                var parameters = constructor.GetParameters();
                if (parameters.Length != 1)
                {
                    continue;
                }

                if (parameters[0].ParameterType == filterType)
                {
                    return constructor.Invoke(new[] { filter });
                }

                if (parameters[0].ParameterType.IsArray && parameters[0].ParameterType.GetElementType() == filterType)
                {
                    var filters = Array.CreateInstance(filterType, 1);
                    filters.SetValue(filter, 0);
                    return constructor.Invoke(new object[] { filters });
                }
            }

            throw new InvalidOperationException("Unity Test Runner ExecutionSettings constructor was not found.");
        }

        static Type FindLuxEditorType(string fullName)
        {
            return AppDomain.CurrentDomain
                .GetAssemblies()
                .Select(assembly => assembly.GetType(fullName, false))
                .FirstOrDefault(type => type != null);
        }
    }


}
