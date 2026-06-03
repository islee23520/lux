using System;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace Linalab.Lux.Editor
{
    public static class LuxBridgeSettings
    {
        public const string Protocol = "lux.unity.bridge.v1";
        public const string SettingsRelativePath = "UserSettings/LuxBridgeSettings.json";
        public const string DefaultGatewayBaseUrl = "http://127.0.0.1:17340";

        [MenuItem("Tools/Linalab/Lux/Unity Bridge/Write Lux Bridge Settings")]
        public static void WriteSettingsFromMenu()
        {
            string path = WriteProjectSettings();
            Debug.Log($"Lux bridge settings written: {path}");
        }

        public static string WriteProjectSettings()
        {
            string projectRoot = GetProjectRoot();
            string userSettings = Path.Combine(projectRoot, "UserSettings");
            Directory.CreateDirectory(userSettings);

            string settingsPath = Path.Combine(projectRoot, SettingsRelativePath);
            File.WriteAllText(settingsPath, BuildJson(projectRoot));
            return settingsPath;
        }

        public static string GetProjectRoot()
        {
            var assetsDirectory = new DirectoryInfo(Application.dataPath);
            return assetsDirectory.Parent == null ? Directory.GetCurrentDirectory() : assetsDirectory.Parent.FullName;
        }

        public static string GetGatewayBaseUrl()
        {
            string configured = TryGetConfiguredGatewayBaseUrl();
            if (!string.IsNullOrEmpty(configured))
            {
                return configured;
            }

            return DefaultGatewayBaseUrl;
        }

        public static string TryGetConfiguredGatewayBaseUrl()
        {
            string configured = ReadGatewayUrlFromSettings();
            if (!string.IsNullOrEmpty(configured))
            {
                return configured.TrimEnd('/');
            }

            string environmentUrl = Environment.GetEnvironmentVariable("LUX_GATEWAY_URL");
            if (!string.IsNullOrEmpty(environmentUrl))
            {
                return environmentUrl.TrimEnd('/');
            }

            string host = Environment.GetEnvironmentVariable("LUX_GATEWAY_HOST");
            string port = Environment.GetEnvironmentVariable("LUX_GATEWAY_PORT");
            if (!string.IsNullOrEmpty(host) || !string.IsNullOrEmpty(port))
            {
                return $"http://{(string.IsNullOrEmpty(host) ? "127.0.0.1" : host)}:{(string.IsNullOrEmpty(port) ? "17340" : port)}";
            }

            return string.Empty;
        }

        static string BuildJson(string projectRoot)
        {
            var packageInfo = UnityEditor.PackageManager.PackageInfo.FindForAssembly(typeof(LuxBridgeSettings).Assembly);
            string packageVersion = packageInfo == null ? string.Empty : packageInfo.version;
            string packageName = packageInfo == null ? "com.linalab.lux" : packageInfo.name;
            string packageRoot = packageInfo == null ? string.Empty : packageInfo.resolvedPath;
            string rustGatewayPath = string.IsNullOrEmpty(packageRoot)
                ? string.Empty
                : Path.Combine(packageRoot, "RustGateway~");

            return "{\n"
                + $"  \"schema_version\": 1,\n"
                + $"  \"protocol\": \"{Escape(Protocol)}\",\n"
                + $"  \"package_name\": \"{Escape(packageName)}\",\n"
                + $"  \"package_version\": \"{Escape(packageVersion)}\",\n"
                + $"  \"project_root\": \"{Escape(projectRoot)}\",\n"
                + $"  \"rust_gateway_path\": \"{Escape(rustGatewayPath)}\",\n"
                + $"  \"gateway_url\": \"{Escape(GetGatewayBaseUrl())}\",\n"
                + $"  \"unity_server_port\": null,\n"
                + $"  \"generated_at_utc\": \"{Escape(DateTime.UtcNow.ToString("o"))}\"\n"
                + "}\n";
        }

        static string ReadGatewayUrlFromSettings()
        {
            string settingsPath = Path.Combine(GetProjectRoot(), SettingsRelativePath);
            if (!File.Exists(settingsPath))
            {
                return string.Empty;
            }

            try
            {
                var settings = JsonUtility.FromJson<BridgeSettingsFile>(File.ReadAllText(settingsPath));
                return settings == null ? string.Empty : settings.gateway_url;
            }
            catch (Exception error)
            {
                Debug.LogWarning($"Failed to read Lux gateway URL from bridge settings: {error.Message}");
                return string.Empty;
            }
        }

        [Serializable]
        sealed class BridgeSettingsFile
        {
            public string gateway_url;
        }

        static string Escape(string value)
        {
            return (value ?? string.Empty)
                .Replace("\\", "\\\\")
                .Replace("\"", "\\\"")
                .Replace("\n", "\\n")
                .Replace("\r", "\\r");
        }
    }
}
