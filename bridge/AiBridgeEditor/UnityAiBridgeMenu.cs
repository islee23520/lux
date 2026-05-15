using System.IO;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    public static class UnityAiBridgeMenu
    {
        public const string AutoStartPreferenceKey = "Linalab.UnityAiBridge.AutoStartContextServer";

        private const string MenuRoot = "Tools/Linalab/Lux/AI Bridge/";
        private const string StartContextServerMenu = MenuRoot + "Start Context Server";
        private const string StopContextServerMenu = MenuRoot + "Stop Context Server";
        private const string RestartContextServerMenu = MenuRoot + "Restart Context Server";
        private const string RevealServerDiscoveryMenu = MenuRoot + "Reveal Server Discovery";
        private const string CopyMcpHelperCommandMenu = MenuRoot + "Copy MCP Helper Command";
        private const string AutoStartContextServerMenu = MenuRoot + "Auto Start Context Server";

        public static UnityAiBridgeTcpServer StartContextServer()
        {
            return UnityAiBridgeTcpServer.StartShared();
        }

        public static void StopContextServer()
        {
            UnityAiBridgeTcpServer.StopShared();
        }

        public static UnityAiBridgeTcpServer RestartContextServer()
        {
            UnityAiBridgeTcpServer.StopShared();
            return UnityAiBridgeTcpServer.StartShared();
        }

        public static string GetServerDiscoveryPath()
        {
            return UnityAiBridgeTcpServer.GetDiscoveryFilePath();
        }

        public static void RevealServerDiscovery()
        {
            var discoveryPath = GetServerDiscoveryPath();
            if (!File.Exists(discoveryPath))
            {
                Debug.LogWarning($"Unity AI Bridge discovery file does not exist yet: {discoveryPath}");
                return;
            }

            EditorUtility.RevealInFinder(discoveryPath);
        }

        public static string BuildMcpHelperCommand()
        {
            return BuildMcpHelperCommand(Directory.GetCurrentDirectory());
        }

        private static string BuildMcpHelperCommand(string projectPath)
        {
            var helperEntryPoint = Path.Combine(projectPath, "Packages", "com.linalab.lux", "McpHelper~", "dist", "src", "index.js");
            return $"UNITY_PROJECT_PATH={QuotePosixShellArgument(projectPath)} node {QuotePosixShellArgument(helperEntryPoint)}";
        }

        public static void CopyMcpHelperCommand()
        {
            var command = BuildMcpHelperCommand();
            EditorGUIUtility.systemCopyBuffer = command;
            Debug.Log($"Unity AI Bridge MCP helper command copied: {command}");
        }

        public static bool GetAutoStartEnabled()
        {
            if (!EditorPrefs.HasKey(AutoStartPreferenceKey))
            {
                return true;
            }

            return EditorPrefs.GetBool(AutoStartPreferenceKey, true);
        }

        public static void SetAutoStartEnabled(bool enabled)
        {
            EditorPrefs.SetBool(AutoStartPreferenceKey, enabled);
        }

        [MenuItem(StartContextServerMenu)]
        private static void StartContextServerMenuItem()
        {
            StartContextServer();
        }

        [MenuItem(StopContextServerMenu)]
        private static void StopContextServerMenuItem()
        {
            StopContextServer();
        }

        [MenuItem(RestartContextServerMenu)]
        private static void RestartContextServerMenuItem()
        {
            RestartContextServer();
        }

        [MenuItem(RevealServerDiscoveryMenu)]
        private static void RevealServerDiscoveryMenuItem()
        {
            RevealServerDiscovery();
        }

        [MenuItem(CopyMcpHelperCommandMenu)]
        private static void CopyMcpHelperCommandMenuItem()
        {
            CopyMcpHelperCommand();
        }

        [MenuItem(AutoStartContextServerMenu)]
        private static void ToggleAutoStartContextServerMenuItem()
        {
            SetAutoStartEnabled(!GetAutoStartEnabled());
        }

        [MenuItem(AutoStartContextServerMenu, true)]
        private static bool ToggleAutoStartContextServerMenuItemValidate()
        {
            var enabled = GetAutoStartEnabled();
            Menu.SetChecked(AutoStartContextServerMenu, enabled);
            return true;
        }

        private static string QuotePosixShellArgument(string value)
        {
            return "'" + value.Replace("'", "'\\''") + "'";
        }
    }
}
