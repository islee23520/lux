using System;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    [InitializeOnLoad]
    internal static class UnityAiSelectionSnapshotCollector
    {
        private static readonly object CacheLock = new object();
        private static UnityAiSelectionSnapshot cachedSnapshot;

        static UnityAiSelectionSnapshotCollector()
        {
            Selection.selectionChanged += HandleSelectionChanged;
            RefreshCacheFromSelection();
        }

        public static UnityAiSelectionSnapshot GetCachedSnapshot()
        {
            lock (CacheLock)
            {
                return CloneSnapshot(cachedSnapshot);
            }
        }

        public static UnityAiSelectionSnapshot CaptureCurrentSnapshot()
        {
            var snapshot = BuildSnapshot(Selection.objects);

            lock (CacheLock)
            {
                cachedSnapshot = snapshot;
                return CloneSnapshot(cachedSnapshot);
            }
        }

        private static void HandleSelectionChanged()
        {
            RefreshCacheFromSelection();
        }

        private static void RefreshCacheFromSelection()
        {
            var snapshot = BuildSnapshot(Selection.objects);

            lock (CacheLock)
            {
                cachedSnapshot = snapshot;
            }
        }

        private static UnityAiSelectionSnapshot BuildSnapshot(UnityEngine.Object[] selectionObjects)
        {
            var selectionCount = selectionObjects == null ? 0 : selectionObjects.Length;
            var selectedFiles = new UnityAiSelectedFileMetadata[selectionCount];

            for (var i = 0; i < selectionCount; i++)
            {
                selectedFiles[i] = BuildMetadata(selectionObjects[i], i, selectionCount);
            }

            return new UnityAiSelectionSnapshot
            {
                capturedAtUtc = DateTime.UtcNow.ToString("O"),
                selectionCount = selectionCount,
                selectedFiles = selectedFiles
            };
        }

        private static UnityAiSelectedFileMetadata BuildMetadata(UnityEngine.Object selectedObject, int selectionIndex, int selectionCount)
        {
            if (selectedObject == null)
            {
                return new UnityAiSelectedFileMetadata
                {
                    assetPath = string.Empty,
                    absolutePath = string.Empty,
                    guid = string.Empty,
                    name = string.Empty,
                    extension = string.Empty,
                    isFolder = false,
                    exists = false,
                    mainAssetType = string.Empty,
                    fileSizeBytes = -1,
                    lastModifiedUtc = string.Empty,
                    selectionIndex = selectionIndex,
                    selectionCount = selectionCount
                };
            }

            var assetPath = AssetDatabase.GetAssetPath(selectedObject);
            var absolutePath = GetAbsolutePath(assetPath);
            var isFolder = !string.IsNullOrEmpty(assetPath) && AssetDatabase.IsValidFolder(assetPath);
            var exists = !string.IsNullOrEmpty(absolutePath) && (File.Exists(absolutePath) || Directory.Exists(absolutePath));
            var mainAssetType = string.Empty;

            if (!string.IsNullOrEmpty(assetPath))
            {
                var assetType = AssetDatabase.GetMainAssetTypeAtPath(assetPath);
                if (assetType != null)
                {
                    mainAssetType = assetType.Name;
                }
            }

            return new UnityAiSelectedFileMetadata
            {
                assetPath = assetPath ?? string.Empty,
                absolutePath = absolutePath,
                guid = string.IsNullOrEmpty(assetPath) ? string.Empty : AssetDatabase.AssetPathToGUID(assetPath),
                name = selectedObject.name ?? string.Empty,
                extension = GetExtension(assetPath),
                isFolder = isFolder,
                exists = exists,
                mainAssetType = mainAssetType,
                fileSizeBytes = GetFileSizeBytes(absolutePath, isFolder, exists),
                lastModifiedUtc = GetLastModifiedUtc(absolutePath, exists),
                selectionIndex = selectionIndex,
                selectionCount = selectionCount
            };
        }

        private static string GetAbsolutePath(string assetPath)
        {
            if (string.IsNullOrEmpty(assetPath))
            {
                return string.Empty;
            }

            var projectRoot = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));
            return Path.GetFullPath(Path.Combine(projectRoot, assetPath));
        }

        private static string GetExtension(string assetPath)
        {
            if (string.IsNullOrEmpty(assetPath))
            {
                return string.Empty;
            }

            var extension = Path.GetExtension(assetPath);
            return extension ?? string.Empty;
        }

        private static long GetFileSizeBytes(string absolutePath, bool isFolder, bool exists)
        {
            if (!exists || string.IsNullOrEmpty(absolutePath))
            {
                return -1;
            }

            if (isFolder)
            {
                return 0;
            }

            return new FileInfo(absolutePath).Length;
        }

        private static string GetLastModifiedUtc(string absolutePath, bool exists)
        {
            if (!exists || string.IsNullOrEmpty(absolutePath))
            {
                return string.Empty;
            }

            if (Directory.Exists(absolutePath))
            {
                return Directory.GetLastWriteTimeUtc(absolutePath).ToString("O");
            }

            if (File.Exists(absolutePath))
            {
                return File.GetLastWriteTimeUtc(absolutePath).ToString("O");
            }

            return string.Empty;
        }

        private static UnityAiSelectionSnapshot CloneSnapshot(UnityAiSelectionSnapshot source)
        {
            if (source == null)
            {
                return null;
            }

            var selectedFiles = new UnityAiSelectedFileMetadata[source.selectedFiles == null ? 0 : source.selectedFiles.Length];
            for (var i = 0; i < selectedFiles.Length; i++)
            {
                var file = source.selectedFiles[i];
                selectedFiles[i] = file == null ? null : new UnityAiSelectedFileMetadata
                {
                    assetPath = file.assetPath,
                    absolutePath = file.absolutePath,
                    guid = file.guid,
                    name = file.name,
                    extension = file.extension,
                    isFolder = file.isFolder,
                    exists = file.exists,
                    mainAssetType = file.mainAssetType,
                    fileSizeBytes = file.fileSizeBytes,
                    lastModifiedUtc = file.lastModifiedUtc,
                    selectionIndex = file.selectionIndex,
                    selectionCount = file.selectionCount
                };
            }

            return new UnityAiSelectionSnapshot
            {
                capturedAtUtc = source.capturedAtUtc,
                selectionCount = source.selectionCount,
                selectedFiles = selectedFiles
            };
        }
    }
}
