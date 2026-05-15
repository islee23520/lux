using System;

namespace Linalab.UnityAiBridge.Editor
{
    [Serializable]
    public sealed class UnityAiBridgeDiscovery
    {
        public string host;
        public int port;
        public string token;
        public string protocolVersion;
        public string projectPath;
        public int pid;
        public string startedAtUtc;
    }
}
