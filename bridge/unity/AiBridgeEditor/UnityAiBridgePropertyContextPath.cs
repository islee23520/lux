using System;
using System.Text;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    [Serializable]
    public sealed class UnityAiBridgePropertyContextPath
    {
        public int schemaVersion;
        public string targetKind;
        public string targetType;
        public string targetName;
        public string scenePath;
        public string hierarchyPath;
        public string componentType;
        public string assetPath;
        public string assetGuid;
        public string prefabAssetPath;
        public string prefabAssetGuid;
        public string propertyPath;
        public string displayName;
        public string propertyType;
        public int targetCount;

        public static UnityAiBridgePropertyContextPath FromProperty(SerializedProperty property)
        {
            if (property == null)
            {
                throw new ArgumentNullException(nameof(property));
            }

            var serializedObject = property.serializedObject;
            var target = serializedObject == null ? null : serializedObject.targetObject;
            var context = new UnityAiBridgePropertyContextPath
            {
                schemaVersion = 1,
                targetKind = ResolveTargetKind(target),
                targetType = target == null ? string.Empty : target.GetType().FullName,
                targetName = target == null ? string.Empty : target.name,
                propertyPath = property.propertyPath,
                displayName = property.displayName,
                propertyType = property.propertyType.ToString(),
                targetCount = serializedObject == null || serializedObject.targetObjects == null ? 0 : serializedObject.targetObjects.Length
            };

            PopulateObjectContext(context, target);
            PopulateAssetContext(context, target);
            return context;
        }

        public string ToPath()
        {
            var builder = new StringBuilder("lux://unity/property");
            AppendQuery(builder, "kind", targetKind, true);
            AppendQuery(builder, "asset", assetPath, false);
            AppendQuery(builder, "assetGuid", assetGuid, false);
            AppendQuery(builder, "prefab", prefabAssetPath, false);
            AppendQuery(builder, "prefabGuid", prefabAssetGuid, false);
            AppendQuery(builder, "scene", scenePath, false);
            AppendQuery(builder, "hierarchy", hierarchyPath, false);
            AppendQuery(builder, "component", componentType, false);
            AppendQuery(builder, "targetType", targetType, false);
            AppendQuery(builder, "property", propertyPath, false);
            AppendQuery(builder, "name", displayName, false);
            AppendQuery(builder, "valueType", propertyType, false);
            AppendQuery(builder, "targets", targetCount.ToString(), false);
            return builder.ToString();
        }

        private static void PopulateObjectContext(UnityAiBridgePropertyContextPath context, UnityEngine.Object target)
        {
            var gameObject = target as GameObject;
            var component = target as Component;
            if (component != null)
            {
                gameObject = component.gameObject;
                context.componentType = component.GetType().FullName;
            }
            else
            {
                context.componentType = string.Empty;
            }

            if (gameObject == null)
            {
                context.scenePath = string.Empty;
                context.hierarchyPath = string.Empty;
                return;
            }

            context.scenePath = gameObject.scene.IsValid() ? gameObject.scene.path : string.Empty;
            context.hierarchyPath = BuildHierarchyPath(gameObject.transform);
            context.prefabAssetPath = PrefabUtility.GetPrefabAssetPathOfNearestInstanceRoot(gameObject) ?? string.Empty;
            context.prefabAssetGuid = string.IsNullOrEmpty(context.prefabAssetPath) ? string.Empty : AssetDatabase.AssetPathToGUID(context.prefabAssetPath);
        }

        private static void PopulateAssetContext(UnityAiBridgePropertyContextPath context, UnityEngine.Object target)
        {
            if (target == null)
            {
                context.assetPath = string.Empty;
                context.assetGuid = string.Empty;
                return;
            }

            context.assetPath = AssetDatabase.GetAssetPath(target) ?? string.Empty;
            context.assetGuid = string.IsNullOrEmpty(context.assetPath) ? string.Empty : AssetDatabase.AssetPathToGUID(context.assetPath);

            if (string.IsNullOrEmpty(context.prefabAssetPath) && context.assetPath.EndsWith(".prefab", StringComparison.OrdinalIgnoreCase))
            {
                context.prefabAssetPath = context.assetPath;
                context.prefabAssetGuid = context.assetGuid;
            }
        }

        private static string ResolveTargetKind(UnityEngine.Object target)
        {
            if (target is Component)
            {
                return "component";
            }

            if (target is GameObject)
            {
                return "game_object";
            }

            return target == null ? "unknown" : "asset";
        }

        private static string BuildHierarchyPath(Transform transform)
        {
            if (transform == null)
            {
                return string.Empty;
            }

            var path = transform.name;
            var current = transform.parent;
            while (current != null)
            {
                path = current.name + "/" + path;
                current = current.parent;
            }

            return "/" + path;
        }

        private static void AppendQuery(StringBuilder builder, string key, string value, bool first)
        {
            builder.Append(first ? '?' : '&');
            builder.Append(Uri.EscapeDataString(key));
            builder.Append('=');
            builder.Append(Uri.EscapeDataString(value ?? string.Empty));
        }
    }
}
