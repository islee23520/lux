using System;

namespace Linalab.UnityAiBridge.Editor
{
    [Serializable]
    public sealed class UnityAiSelectionSnapshot
    {
        public string capturedAtUtc;
        public int selectionCount;
        public UnityAiSelectedFileMetadata[] selectedFiles;
    }

    [Serializable]
    public sealed class UnityAiSelectedFileMetadata
    {
        public string assetPath;
        public string absolutePath;
        public string guid;
        public string name;
        public string extension;
        public bool isFolder;
        public bool exists;
        public string mainAssetType;
        public long fileSizeBytes;
        public string lastModifiedUtc;
        public int selectionIndex;
        public int selectionCount;
    }
}
