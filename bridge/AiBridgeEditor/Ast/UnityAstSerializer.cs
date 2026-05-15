using System;
using System.Collections.Generic;
using UnityEditor;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Linalab.UnityAiBridge.Editor.Ast
{
    public static class UnityAstSerializer
    {
        internal static int nodeIdCounter;
        internal static Dictionary<int, string> objectIdToAstId = new Dictionary<int, string>();

        public static UnityAstNode FromGameObject(GameObject go, int maxDepth)
        {
            return FromGameObject(go, maxDepth, true);
        }

        public static UnityAstNode FromGameObject(GameObject go, int maxDepth, bool includeComponents)
        {
            ResetState();
            var normalizedDepth = NormalizeExplicitDepth(maxDepth);
            PreassignIds(go, normalizedDepth);
            return BuildNode(go, normalizedDepth, includeComponents);
        }

        public static UnityAstScene FromScene(Scene scene, bool rootOnly, int maxDepth)
        {
            ResetState();
            var depth = rootOnly ? 1 : UnityAstConstants.NormalizeDepth(maxDepth);
            var roots = scene.GetRootGameObjects();
            for (var i = 0; i < roots.Length; i++)
            {
                PreassignIds(roots[i], depth);
            }

            var nodes = new List<UnityAstNode>();
            var totalGameObjects = 0;
            var totalComponents = 0;
            for (var i = 0; i < roots.Length; i++)
            {
                var node = BuildNode(roots[i], depth, true);
                nodes.Add(node);
                totalGameObjects += UnityAstStats.CountNodes(node);
                totalComponents += UnityAstStats.CountComponents(node);
            }

            return new UnityAstScene
            {
                schemaVersion = UnityAstScene.SchemaVersion,
                capturedAtUtc = DateTime.UtcNow.ToString("O"),
                sceneName = scene.name,
                scenePath = scene.path,
                rootCount = roots.Length,
                totalGameObjects = totalGameObjects,
                totalComponents = totalComponents,
                roots = nodes.ToArray()
            };
        }

        public static UnityAstComponent SerializeComponent(Component comp)
        {
            if (comp == null)
            {
                return null;
            }

            var serializedObject = new SerializedObject(comp);
            var component = new UnityAstComponent
            {
                type = comp.GetType().Name,
                scriptGuid = GetScriptGuid(comp),
                enabled = GetComponentEnabled(serializedObject),
                properties = SerializeProperties(serializedObject)
            };
            serializedObject.Dispose();
            return component;
        }

        public static UnityAstProperty[] SerializeProperties(SerializedObject serializedObj)
        {
            if (serializedObj == null)
            {
                return new UnityAstProperty[0];
            }

            var properties = new List<UnityAstProperty>();
            var iterator = serializedObj.GetIterator();
            var enterChildren = true;
            while (iterator.NextVisible(enterChildren))
            {
                enterChildren = false;
                if (iterator.depth != 0 || UnityAstConstants.ShouldSkipProperty(iterator) || IsTransformDefaultValue(iterator.name, iterator))
                {
                    continue;
                }

                var property = FormatProperty(iterator);
                if (property != null)
                {
                    properties.Add(property);
                }
            }

            return properties.ToArray();
        }

        public static string CompressObjectReference(UnityEngine.Object obj)
        {
            if (obj == null)
            {
                return "null";
            }

            var gameObject = obj as GameObject;
            if (gameObject != null)
            {
                string astId;
                if (objectIdToAstId.TryGetValue(gameObject.GetInstanceID(), out astId))
                {
                    return "#" + astId;
                }
            }

            var component = obj as Component;
            if (component != null && component.gameObject != null)
            {
                string astId;
                if (objectIdToAstId.TryGetValue(component.gameObject.GetInstanceID(), out astId))
                {
                    return "#" + astId + "/" + component.GetType().Name;
                }
            }

            var path = AssetDatabase.GetAssetPath(obj);
            if (!string.IsNullOrEmpty(path))
            {
                var guid = AssetDatabase.AssetPathToGUID(path);
                if (!string.IsNullOrEmpty(guid))
                {
                    return "{guid:" + guid + "}";
                }
            }

            return string.IsNullOrEmpty(obj.name) ? obj.GetType().Name : obj.name;
        }

        public static bool IsTransformDefaultValue(string propName, SerializedProperty prop)
        {
            if (string.Equals(propName, "m_LocalPosition", StringComparison.Ordinal))
            {
                return UnityAstConstants.IsDefaultVector3(prop, Vector3.zero);
            }

            if (string.Equals(propName, "m_LocalRotation", StringComparison.Ordinal))
            {
                return UnityAstConstants.IsDefaultQuaternion(prop, Quaternion.identity);
            }

            if (string.Equals(propName, "m_LocalScale", StringComparison.Ordinal))
            {
                return UnityAstConstants.IsDefaultVector3(prop, Vector3.one);
            }

            return false;
        }

        private static void ResetState()
        {
            nodeIdCounter = 0;
            objectIdToAstId = new Dictionary<int, string>();
        }

        private static int NormalizeExplicitDepth(int maxDepth)
        {
            if (maxDepth <= 0)
            {
                return 1;
            }

            return Mathf.Min(maxDepth, UnityAstConstants.AbsoluteMaxDepth);
        }

        private static void PreassignIds(GameObject go, int remainingDepth)
        {
            if (go == null || remainingDepth <= 0 || objectIdToAstId.ContainsKey(go.GetInstanceID()))
            {
                return;
            }

            objectIdToAstId.Add(go.GetInstanceID(), "g" + nodeIdCounter.ToString());
            nodeIdCounter++;

            if (remainingDepth <= 1)
            {
                return;
            }

            var transform = go.transform;
            for (var i = 0; i < transform.childCount; i++)
            {
                PreassignIds(transform.GetChild(i).gameObject, remainingDepth - 1);
            }
        }

        private static UnityAstNode BuildNode(GameObject go, int remainingDepth, bool includeComponents)
        {
            if (go == null)
            {
                return null;
            }

            string astId;
            if (!objectIdToAstId.TryGetValue(go.GetInstanceID(), out astId))
            {
                astId = "g" + nodeIdCounter.ToString();
                objectIdToAstId.Add(go.GetInstanceID(), astId);
                nodeIdCounter++;
            }

            var node = new UnityAstNode
            {
                id = astId,
                name = go.name,
                activeSelf = go.activeSelf,
                layer = go.layer,
                tag = UnityAstConstants.GetTag(go),
                components = includeComponents ? SerializeComponents(go) : new UnityAstComponent[0],
                children = null
            };

            if (remainingDepth <= 1)
            {
                return node;
            }

            var children = new List<UnityAstNode>();
            var transform = go.transform;
            for (var i = 0; i < transform.childCount; i++)
            {
                var child = BuildNode(transform.GetChild(i).gameObject, remainingDepth - 1, includeComponents);
                if (child != null)
                {
                    children.Add(child);
                }
            }

            node.children = children.ToArray();
            return node;
        }

        private static UnityAstComponent[] SerializeComponents(GameObject go)
        {
            var unityComponents = go.GetComponents<Component>();
            var components = new List<UnityAstComponent>();
            for (var i = 0; i < unityComponents.Length; i++)
            {
                if (unityComponents[i] == null)
                {
                    continue;
                }

                var component = SerializeComponent(unityComponents[i]);
                if (component != null)
                {
                    components.Add(component);
                }
            }

            return components.ToArray();
        }

        private static UnityAstProperty FormatProperty(SerializedProperty prop)
        {
            var property = new UnityAstProperty
            {
                key = prop.propertyPath,
                valueType = "string",
                stringValue = string.Empty,
                refValue = string.Empty
            };

            switch (prop.propertyType)
            {
                case SerializedPropertyType.Integer:
                    property.valueType = "int";
                    property.intValue = prop.intValue;
                    return property;
                case SerializedPropertyType.Boolean:
                    property.valueType = "bool";
                    property.boolValue = prop.boolValue;
                    return property;
                case SerializedPropertyType.Float:
                    property.valueType = "float";
                    property.floatValue = prop.floatValue;
                    return property;
                case SerializedPropertyType.String:
                    property.valueType = "string";
                    property.stringValue = prop.stringValue ?? string.Empty;
                    return property;
                case SerializedPropertyType.Color:
                    property.valueType = "color";
                    var color = (Color32)prop.colorValue;
                    property.c_r = color.r;
                    property.c_g = color.g;
                    property.c_b = color.b;
                    property.c_a = color.a;
                    return property;
                case SerializedPropertyType.ObjectReference:
                    property.valueType = prop.objectReferenceValue == null ? "null" : "ref";
                    property.refValue = CompressObjectReference(prop.objectReferenceValue);
                    return property;
                case SerializedPropertyType.Vector3:
                    property.valueType = "vector3";
                    property.v3_x = prop.vector3Value.x;
                    property.v3_y = prop.vector3Value.y;
                    property.v3_z = prop.vector3Value.z;
                    return property;
                case SerializedPropertyType.Quaternion:
                    property.valueType = "quaternion";
                    property.q_x = prop.quaternionValue.x;
                    property.q_qy = prop.quaternionValue.y;
                    property.q_z = prop.quaternionValue.z;
                    property.q_w = prop.quaternionValue.w;
                    return property;
                case SerializedPropertyType.Enum:
                    property.valueType = "string";
                    property.stringValue = prop.enumDisplayNames != null && prop.enumValueIndex >= 0 && prop.enumValueIndex < prop.enumDisplayNames.Length
                        ? prop.enumDisplayNames[prop.enumValueIndex]
                        : prop.enumValueIndex.ToString();
                    property.intValue = prop.enumValueIndex;
                    return property;
                default:
                    if (prop.isArray)
                    {
                        property.valueType = "string";
                        property.stringValue = "array[" + prop.arraySize.ToString() + "]";
                        return property;
                    }

                    return null;
            }
        }

        private static string GetScriptGuid(Component comp)
        {
            var monoBehaviour = comp as MonoBehaviour;
            if (monoBehaviour == null)
            {
                return string.Empty;
            }

            var script = MonoScript.FromMonoBehaviour(monoBehaviour);
            if (script == null)
            {
                return string.Empty;
            }

            var path = AssetDatabase.GetAssetPath(script);
            return string.IsNullOrEmpty(path) ? string.Empty : AssetDatabase.AssetPathToGUID(path);
        }

        private static bool GetComponentEnabled(SerializedObject serializedObject)
        {
            var enabledProperty = serializedObject.FindProperty("m_Enabled");
            return enabledProperty == null || enabledProperty.boolValue;
        }
    }
}
