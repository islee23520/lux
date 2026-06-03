using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    [InitializeOnLoad]
    internal static class UnityAiBridgeBootstrap
    {
        private static readonly bool EnableStartupContextServer = true;
        private const double EnsureIntervalSeconds = 5.0d;
        private static double nextEnsureTime;

        static UnityAiBridgeBootstrap()
        {
            EditorApplication.delayCall += StartIfEnabled;
            EditorApplication.update += EnsureRunningIfEnabled;
        }

        private static void StartIfEnabled()
        {
            if (!EnableStartupContextServer)
            {
                return;
            }

            if (Application.isBatchMode)
            {
                return;
            }

            if (!UnityAiBridgeMenu.GetAutoStartEnabled())
            {
                return;
            }

            UnityAiBridgeTcpServer.EnsureSharedDiscoverable();
        }

        private static void EnsureRunningIfEnabled()
        {
            if (EditorApplication.timeSinceStartup < nextEnsureTime)
            {
                return;
            }

            nextEnsureTime = EditorApplication.timeSinceStartup + EnsureIntervalSeconds;

            if (!EnableStartupContextServer || Application.isBatchMode || !UnityAiBridgeMenu.GetAutoStartEnabled())
            {
                return;
            }

            try
            {
                UnityAiBridgeTcpServer.EnsureSharedDiscoverable();
            }
            catch (System.Exception exception)
            {
                Debug.LogWarning($"Lux Unity AI Bridge backend auto-start failed: {exception.Message}");
            }
        }
    }
}
