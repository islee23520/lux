using System.IO;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    internal static class DiscoveryFileCleanup
    {
        private static readonly string DiscoveryFilePathValue = Path.Combine(Directory.GetCurrentDirectory(), "Library", "UnityAiBridge", "server.json");

        public static string DiscoveryFilePath => DiscoveryFilePathValue;

        public static void DeleteDiscoveryFile()
        {
            if (File.Exists(DiscoveryFilePathValue))
            {
                File.Delete(DiscoveryFilePathValue);
            }

            var discoveryDirectory = Path.GetDirectoryName(DiscoveryFilePathValue);
            if (string.IsNullOrEmpty(discoveryDirectory) || !Directory.Exists(discoveryDirectory))
            {
                return;
            }

            if (Directory.GetFileSystemEntries(discoveryDirectory).Length == 0)
            {
                Directory.Delete(discoveryDirectory);
            }
        }
    }
}
