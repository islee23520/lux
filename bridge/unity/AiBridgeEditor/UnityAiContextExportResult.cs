namespace Linalab.UnityAiBridge.Editor
{
    public readonly struct UnityAiContextExportResult
    {
        public UnityAiContextExportResult(string outputPath, string json, UnityAiContext context)
        {
            OutputPath = outputPath;
            Json = json;
            Context = context;
        }

        public string OutputPath { get; }
        public string Json { get; }
        public UnityAiContext Context { get; }
    }
}
