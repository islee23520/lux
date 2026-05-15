using System;
using System.Collections.Generic;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Ast
{
    public static class UnityAstConstants
    {
        public const int DefaultMaxDepth = 5;
        public const int AbsoluteMaxDepth = 20;

        public static readonly HashSet<string> AlwaysSkipPropertyNames = new HashSet<string>(StringComparer.Ordinal)
        {
            "m_ObjectHideFlags",
            "m_CorrespondingSourceObject",
            "m_CorrespondingObject",
            "m_PrefabInstance",
            "m_PrefabAsset",
            "m_GameObject",
            "m_Script",
            "m_Name"
        };

        public static int NormalizeDepth(int maxDepth)
        {
            if (maxDepth <= 0)
            {
                return DefaultMaxDepth;
            }

            return Mathf.Min(maxDepth, AbsoluteMaxDepth);
        }

        public static bool ShouldSkipProperty(SerializedProperty property)
        {
            if (property == null || AlwaysSkipPropertyNames.Contains(property.name))
            {
                return true;
            }

            return string.Equals(property.name, "m_Enabled", StringComparison.Ordinal) && property.boolValue;
        }

        public static bool IsDefaultVector3(SerializedProperty property, Vector3 defaultValue)
        {
            if (property == null || property.propertyType != SerializedPropertyType.Vector3)
            {
                return false;
            }

            return NearlyEqual(property.vector3Value.x, defaultValue.x)
                && NearlyEqual(property.vector3Value.y, defaultValue.y)
                && NearlyEqual(property.vector3Value.z, defaultValue.z);
        }

        public static bool IsDefaultQuaternion(SerializedProperty property, Quaternion defaultValue)
        {
            if (property == null || property.propertyType != SerializedPropertyType.Quaternion)
            {
                return false;
            }

            return NearlyEqual(property.quaternionValue.x, defaultValue.x)
                && NearlyEqual(property.quaternionValue.y, defaultValue.y)
                && NearlyEqual(property.quaternionValue.z, defaultValue.z)
                && NearlyEqual(property.quaternionValue.w, defaultValue.w);
        }

        public static bool NearlyEqual(float left, float right)
        {
            return Mathf.Abs(left - right) <= 0.00001f;
        }

        public static string GetTag(GameObject gameObject)
        {
            if (gameObject == null)
            {
                return string.Empty;
            }

            try
            {
                return gameObject.tag ?? string.Empty;
            }
            catch (UnityException)
            {
                return string.Empty;
            }
        }
    }
}
