using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;
using Linalab.UnityAiBridge.Editor;
using UnityEditor;

namespace Linalab.Lux.Editor
{
    [InitializeOnLoad]
    public static class LuxCompileEventBroadcaster
    {
        static readonly Regex CompilerErrorPattern = new Regex(
            @"(?<file>[^\r\n]+\.cs)\((?<line>\d+)(?:,\d+)?\):\s+error\s+(?<code>[A-Z]+\d+):\s+(?<message>.+)",
            RegexOptions.Compiled);

        static LuxCompileEventBroadcaster()
        {
            AssemblyReloadEvents.afterAssemblyReload += BroadcastCompileResult;
        }

        public static void BroadcastCompileStarted(string reason)
        {
            UnityAiBridgeTcpServer.BroadcastEvent(
                "compile_started",
                new CompileStartedPayload
                {
                    reason = reason ?? string.Empty
                });
        }

        static void BroadcastCompileResult()
        {
            if (EditorUtility.scriptCompilationFailed)
            {
                UnityAiBridgeTcpServer.BroadcastEvent(
                    "compile_result",
                    new CompileResultPayload
                    {
                        success = false,
                        errors = CaptureCompileErrors()
                    });
                return;
            }

            UnityAiBridgeTcpServer.BroadcastEvent(
                "compile_result",
                new CompileResultPayload
                {
                    success = true,
                    errors = new CompileError[0]
                });
        }

        static CompileError[] CaptureCompileErrors()
        {
            var errors = new List<CompileError>();
            foreach (var log in LuxUnityContext.GetRecentLogsSnapshot())
            {
                if (!string.Equals(log.Type, "Error", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                var error = ParseCompileError(log.Message) ?? ParseCompileError(log.StackTrace);
                if (error != null)
                {
                    errors.Add(error);
                }
            }

            return errors.ToArray();
        }

        static CompileError ParseCompileError(string text)
        {
            if (string.IsNullOrEmpty(text))
            {
                return null;
            }

            var match = CompilerErrorPattern.Match(text);
            if (!match.Success)
            {
                return null;
            }

            int line;
            int.TryParse(match.Groups["line"].Value, out line);
            return new CompileError
            {
                file = match.Groups["file"].Value,
                line = line,
                code = match.Groups["code"].Value,
                message = match.Groups["message"].Value.Trim()
            };
        }

        public sealed class CompileStartedPayload
        {
            public string reason;
        }

        public sealed class CompileResultPayload
        {
            public bool success;
            public CompileError[] errors;
        }

        public sealed class CompileError
        {
            public string file;
            public int line;
            public string code;
            public string message;
        }
    }
}
