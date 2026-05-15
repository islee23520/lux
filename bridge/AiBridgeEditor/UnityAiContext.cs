using System;

namespace Linalab.UnityAiBridge.Editor
{
    [Serializable]
    public sealed class UnityAiContext
    {
        public int schemaVersion;
        public string capturedAtUtc;
        public string aiTool;
        public string projectName;
        public string projectPath;
        public string unityVersion;
        public string platform;
        public string activeScenePath;
        public string activeSceneName;
        public string[] selection;
        public string[] packages;
        public string[] assemblies;
    }
}
