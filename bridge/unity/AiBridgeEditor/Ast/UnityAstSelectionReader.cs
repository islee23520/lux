using System.Collections.Generic;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Ast
{
    public static class UnityAstSelectionReader
    {
        public static UnityAstSelectionAstPayload ReadSelection(int maxDepth = UnityAstConstants.DefaultMaxDepth, bool includeComponents = true)
        {
            var objects = Selection.gameObjects;
            var nodes = new List<UnityAstNode>();
            for (var i = 0; i < objects.Length; i++)
            {
                var go = objects[i];
                if (go != null)
                {
                    nodes.Add(UnityAstSerializer.FromGameObject(go, maxDepth, includeComponents));
                }
            }

            return new UnityAstSelectionAstPayload
            {
                selectionCount = nodes.Count,
                selections = nodes.ToArray()
            };
        }
    }
}
