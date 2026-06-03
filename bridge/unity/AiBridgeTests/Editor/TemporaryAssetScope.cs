using System;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    internal sealed class TemporaryAssetScope : IDisposable
    {
        private readonly string rootAssetPath;
        private readonly string rootFullPath;
        private bool isDisposed;

        private TemporaryAssetScope(string rootAssetPath)
        {
            this.rootAssetPath = rootAssetPath;
            var projectRoot = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));
            rootFullPath = Path.Combine(projectRoot, rootAssetPath);
            Directory.CreateDirectory(rootFullPath);
        }

        public static TemporaryAssetScope Create(string scopeName = null)
        {
            var suffix = string.IsNullOrEmpty(scopeName) ? Guid.NewGuid().ToString("N") : scopeName;
            return new TemporaryAssetScope($"Assets/__Linalab.UnityAiBridge.Tests.{suffix}");
        }

        public string CreateTextAsset(string fileName, string contents)
        {
            EnsureNotDisposed();

            var assetPath = Path.Combine(rootAssetPath, fileName + ".txt").Replace('\\', '/');
            var filePath = Path.Combine(rootFullPath, fileName + ".txt");
            File.WriteAllText(filePath, contents);
            AssetDatabase.ImportAsset(assetPath, ImportAssetOptions.ForceSynchronousImport);
            return assetPath;
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            isDisposed = true;

            if (AssetDatabase.IsValidFolder(rootAssetPath))
            {
                AssetDatabase.DeleteAsset(rootAssetPath);
                AssetDatabase.Refresh();
                return;
            }

            if (Directory.Exists(rootFullPath))
            {
                Directory.Delete(rootFullPath, true);
            }

            AssetDatabase.Refresh();
        }

        private void EnsureNotDisposed()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(TemporaryAssetScope));
            }
        }
    }
}
