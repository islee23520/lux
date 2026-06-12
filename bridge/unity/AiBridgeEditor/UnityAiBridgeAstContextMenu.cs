using Linalab.UnityAiBridge.Editor.Ast;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    [InitializeOnLoad]
    public static class UnityAiBridgeAstContextMenu
    {
        private const string MenuRoot = "Tools/Linalab/Lux/AI Bridge/";
        private const string CopySelectionAstMenu = MenuRoot + "Copy Selection AST Context";
        private const string CopyActiveSceneAstMenu = MenuRoot + "Copy Active Scene AST Context";
        private const string CopyHierarchySelectionAstMenu = "GameObject/Lux/Copy Selection AST Context";
        private const string CopyPropertyContextPathMenu = "Lux/Copy Property Context Path";
        private const string CopyPropertyContextJsonMenu = "Lux/Copy Property Context JSON";

        static UnityAiBridgeAstContextMenu()
        {
            EditorApplication.contextualPropertyMenu -= AddPropertyContextMenuItems;
            EditorApplication.contextualPropertyMenu += AddPropertyContextMenuItems;
        }

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

        public static string BuildPropertyContextPath(SerializedProperty property)
        {
            var context = UnityAiBridgePropertyContextPath.FromProperty(property);
            return context.ToPath();
        }

        public static string BuildPropertyContextJson(SerializedProperty property)
        {
            var context = UnityAiBridgePropertyContextPath.FromProperty(property);
            return JsonUtility.ToJson(context, true);
        }

        public static void CopyPropertyContextPath(SerializedProperty property)
        {
            var path = BuildPropertyContextPath(property);
            EditorGUIUtility.systemCopyBuffer = path;
            Debug.Log($"Lux property context path copied: {path}");
        }

        public static void CopyPropertyContextJson(SerializedProperty property)
        {
            var json = BuildPropertyContextJson(property);
            EditorGUIUtility.systemCopyBuffer = json;
            Debug.Log("Lux property context JSON copied.");
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

        private static void AddPropertyContextMenuItems(GenericMenu menu, SerializedProperty property)
        {
            if (menu == null || property == null)
            {
                return;
            }

            var path = BuildPropertyContextPath(property);
            var json = BuildPropertyContextJson(property);
            menu.AddItem(new GUIContent(CopyPropertyContextPathMenu), false, () => CopyTextToClipboard(path, "Lux property context path copied."));
            menu.AddItem(new GUIContent(CopyPropertyContextJsonMenu), false, () => CopyTextToClipboard(json, "Lux property context JSON copied."));
        }

        private static void CopyTextToClipboard(string text, string message)
        {
            EditorGUIUtility.systemCopyBuffer = text;
            Debug.Log(message);
        }
    }
}
