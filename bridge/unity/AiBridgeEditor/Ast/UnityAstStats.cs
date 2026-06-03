namespace Linalab.UnityAiBridge.Editor.Ast
{
    public static class UnityAstStats
    {
        public static int CountNodes(UnityAstNode node)
        {
            if (node == null)
            {
                return 0;
            }

            var count = 1;
            if (node.children == null)
            {
                return count;
            }

            for (var i = 0; i < node.children.Length; i++)
            {
                count += CountNodes(node.children[i]);
            }

            return count;
        }

        public static int CountComponents(UnityAstNode node)
        {
            if (node == null)
            {
                return 0;
            }

            var count = node.components == null ? 0 : node.components.Length;
            if (node.children == null)
            {
                return count;
            }

            for (var i = 0; i < node.children.Length; i++)
            {
                count += CountComponents(node.children[i]);
            }

            return count;
        }
    }
}
