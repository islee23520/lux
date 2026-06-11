using Linalab.UnityAiBridge.Editor.Ast;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    public static class UnityAiBridgeAstContextMenu
    {
        private const string MenuRoot = "Tools/Linalab/Lux/AI Bridge/";
        private const string CopySelectionAstMenu = MenuRoot + "Copy Selection AST Context";
        private const string CopyActiveSceneAstMenu = MenuRoot + "Copy Active Scene AST Context";
        private const string CopyHierarchySelectionAstMenu = "GameObject/Lux/Copy Selection AST Context";

        public static string BuildSelectionAstContextJson()
        {
            var payload = UnityAstSelectionReader.ReadSelection();
            return JsonUtility.ToJson(payload, true);
        }

        public static string BuildActiveSceneAstContextJson()
        {
            var payload = UnityAstSceneReader.ReadScene();
            return JsonUtility.ToJson(payload, true);
        }

        public static void CopySelectionAstContext()
        {
            var json = BuildSelectionAstContextJson();
            EditorGUIUtility.systemCopyBuffer = json;
            Debug.Log($"Lux selection AST context copied: {Selection.gameObjects.Length} GameObject(s).");
        }

        public static void CopyActiveSceneAstContext()
        {
            var json = BuildActiveSceneAstContextJson();
            EditorGUIUtility.systemCopyBuffer = json;
            Debug.Log("Lux active scene AST context copied.");
        }

        [MenuItem(CopySelectionAstMenu)]
        private static void CopySelectionAstMenuItem()
        {
            CopySelectionAstContext();
        }

        [MenuItem(CopySelectionAstMenu, true)]
        private static bool CopySelectionAstMenuItemValidate()
        {
            return Selection.gameObjects.Length > 0;
        }

        [MenuItem(CopyActiveSceneAstMenu)]
        private static void CopyActiveSceneAstMenuItem()
        {
            CopyActiveSceneAstContext();
        }

        [MenuItem(CopyHierarchySelectionAstMenu, false, 49)]
        private static void CopyHierarchySelectionAstMenuItem()
        {
            CopySelectionAstContext();
        }

        [MenuItem(CopyHierarchySelectionAstMenu, true)]
        private static bool CopyHierarchySelectionAstMenuItemValidate()
        {
            return Selection.gameObjects.Length > 0;
        }
    }
}
