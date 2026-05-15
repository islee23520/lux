using System;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Ast
{
    public static class UnityAstFileReader
    {
        public static UnityAstReadResult ReadAsset(string assetPath)
        {
            if (string.IsNullOrWhiteSpace(assetPath))
            {
                throw new ArgumentException("assetPath is required.", nameof(assetPath));
            }

            var normalizedPath = assetPath.Replace('\\', '/');
            if (!File.Exists(normalizedPath))
            {
                throw new FileNotFoundException("Asset file does not exist.", normalizedPath);
            }

            var extension = Path.GetExtension(normalizedPath).ToLowerInvariant();
            if (string.Equals(extension, ".unity", StringComparison.Ordinal))
            {
                throw new InvalidOperationException("Scene assets must be read with get_scene_ast because scenes require SceneManager state.");
            }

            if (string.Equals(extension, ".prefab", StringComparison.Ordinal))
            {
                return ReadPrefab(normalizedPath);
            }

            if (string.Equals(extension, ".asset", StringComparison.Ordinal))
            {
                return ReadScriptableObjectAsset(normalizedPath);
            }

            throw new NotSupportedException("unsupported_asset_type: " + extension);
        }

        private static UnityAstReadResult ReadPrefab(string assetPath)
        {
            var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(assetPath);
            if (prefab == null)
            {
                throw new InvalidOperationException("Prefab could not be loaded: " + assetPath);
            }

            var ast = UnityAstSerializer.FromGameObject(prefab, UnityAstConstants.DefaultMaxDepth);
            return CreateResult(assetPath, "prefab", ast);
        }

        private static UnityAstReadResult ReadScriptableObjectAsset(string assetPath)
        {
            var gameObject = AssetDatabase.LoadAssetAtPath<GameObject>(assetPath);
            if (gameObject != null)
            {
                var gameObjectAst = UnityAstSerializer.FromGameObject(gameObject, UnityAstConstants.DefaultMaxDepth);
                return CreateResult(assetPath, "asset", gameObjectAst);
            }

            var asset = AssetDatabase.LoadMainAssetAtPath(assetPath);
            if (asset == null)
            {
                throw new InvalidOperationException("Asset could not be loaded: " + assetPath);
            }

            var node = new UnityAstNode
            {
                id = "a0",
                name = asset.name,
                activeSelf = true,
                layer = 0,
                tag = string.Empty,
                components = new[] { SerializeAssetObject(asset) },
                children = new UnityAstNode[0]
            };

            return CreateResult(assetPath, asset is ScriptableObject ? "scriptable_object" : "asset", node);
        }

        private static UnityAstComponent SerializeAssetObject(UnityEngine.Object asset)
        {
            var serializedObject = new SerializedObject(asset);
            var component = new UnityAstComponent
            {
                type = asset.GetType().Name,
                scriptGuid = string.Empty,
                enabled = true,
                properties = UnityAstSerializer.SerializeProperties(serializedObject)
            };
            serializedObject.Dispose();
            return component;
        }

        private static UnityAstReadResult CreateResult(string assetPath, string assetType, UnityAstNode ast)
        {
            return new UnityAstReadResult
            {
                assetPath = assetPath,
                assetType = assetType,
                ast = ast,
                fileSizeBytes = new FileInfo(assetPath).Length,
                astNodeCount = UnityAstStats.CountNodes(ast)
            };
        }
    }
}
