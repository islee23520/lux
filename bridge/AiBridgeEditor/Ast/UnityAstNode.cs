using System;

namespace Linalab.UnityAiBridge.Editor.Ast
{
    [Serializable]
    public sealed class UnityAstProperty
    {
        public string key;
        public string valueType;
        public string stringValue;
        public int intValue;
        public float floatValue;
        public bool boolValue;
        public float v3_x;
        public float v3_y;
        public float v3_z;
        public float q_x;
        public float q_qy;
        public float q_z;
        public float q_w;
        public byte c_r;
        public byte c_g;
        public byte c_b;
        public byte c_a;
        public string refValue;
    }

    [Serializable]
    public sealed class UnityAstComponent
    {
        public string type;
        public string scriptGuid;
        public bool enabled;
        public UnityAstProperty[] properties;
    }

    [Serializable]
    public sealed class UnityAstNode
    {
        public string id;
        public string name;
        public bool activeSelf;
        public int layer;
        public string tag;
        public UnityAstComponent[] components;
        public UnityAstNode[] children;
    }

    [Serializable]
    public sealed class UnityAstScene
    {
        public const int SchemaVersion = 1;

        public int schemaVersion;
        public string capturedAtUtc;
        public string sceneName;
        public string scenePath;
        public int rootCount;
        public int totalGameObjects;
        public int totalComponents;
        public UnityAstNode[] roots;
    }

    [Serializable]
    public sealed class UnityAstReadResult
    {
        public string assetPath;
        public string assetType;
        public UnityAstNode ast;
        public long fileSizeBytes;
        public long astNodeCount;
    }

    [Serializable]
    public sealed class UnityAstSelectionAstPayload
    {
        public int selectionCount;
        public UnityAstNode[] selections;
    }
}
