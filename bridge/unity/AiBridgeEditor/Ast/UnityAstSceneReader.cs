using UnityEngine.SceneManagement;

namespace Linalab.UnityAiBridge.Editor.Ast
{
    public static class UnityAstSceneReader
    {
        public static UnityAstScene ReadScene(bool rootOnly = false, int maxDepth = UnityAstConstants.DefaultMaxDepth)
        {
            var scene = SceneManager.GetActiveScene();
            return UnityAstSerializer.FromScene(scene, rootOnly, maxDepth);
        }
    }
}
