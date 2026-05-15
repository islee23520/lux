# Unity AI Bridge Editor

C# Editor scripts for TCP communication between Unity Editor and LUX Rust gateway. Installed into target Unity projects via `lux bridge install`.

## STRUCTURE
```
AiBridgeEditor/
├── UnityAiBridgeBootstrap.cs     # [InitializeOnLoad] auto-start, 5s ensure interval
├── UnityAiBridgeTcpServer.cs     # 1971L TCP server: discovery, connections, lifecycle
├── UnityAiBridgeProtocol.cs      # 866L request/response protocol
├── UnityAiBridgeDiscovery.cs     # Discovery file management
├── UnityAiBridgeMenu.cs          # Tools/Linalab/Lux/AI Bridge menu items
├── UnityAiBridge.cs              # Context capture + export
├── UnityAiContext.cs              # Context data model
├── UnityAiContextCollector.cs     # Selection, packages, assemblies
├── UnityAiContextExportResult.cs  # Export result model
├── UnityAiSelectionSnapshot.cs   # Selection snapshot model
├── UnityAiSelectionSnapshotCollector.cs # Selection collection
├── AiToolKind.cs                  # Enum: ClaudeCode, OpenCode, Codex
├── UnityAiBridgeAssemblyInfo.cs  # InternalsVisibleTo tests
├── LuxBatchAutomation.cs         # Batch compile: AssetDatabase.Refresh + result JSON
├── LuxSceneSmoke.cs              # Scene smoke test: load + validate root objects
├── LuxUnityContext.cs             # Context refresh: ExportContext to UserSettings/
└── Linalab.UnityAiBridge.Editor.asmdef # Assembly definition
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Fix TCP server | `UnityAiBridgeTcpServer.cs` | Shared singleton, discovery file, port |
| Fix protocol | `UnityAiBridgeProtocol.cs` | Request/response parsing |
| Fix auto-start | `UnityAiBridgeBootstrap.cs` | `[InitializeOnLoad]`, skips batch mode |
| Add menu item | `UnityAiBridgeMenu.cs` | Tools/Linalab/Lux/AI Bridge/ |
| Context export | `UnityAiBridge.cs` | `ExportContext(toolKind, outputPath)` |
| Batch compile | `LuxBatchAutomation.cs` | `Compile()` — AssetDatabase.Refresh + polling |
| Scene test | `LuxSceneSmoke.cs` | `Run()` — env vars LUX_SCENE_SMOKE_* |
| Context refresh | `LuxUnityContext.cs` | `Refresh()` — delegates to UnityAiBridge.ExportContext |

## NAMESPACES
- `Linalab.UnityAiBridge.Editor` — main bridge code (AiBridge, TcpServer, Protocol, etc.)
- `Linalab.Lux.Editor` — batch automation entry points (LuxBatchAutomation, LuxSceneSmoke, LuxUnityContext)

Both live in the same assembly (`Linalab.UnityAiBridge.Editor.asmdef`).

## CONVENTIONS
- Unity 6000.0+ (Unity 6.x) compatibility required.
- Auto-start default: `GetAutoStartEnabled()` returns `true` for new installations.
- Batch mode: Bootstrap skips `Application.isBatchMode`, batch classes write JSON results.
- Results directory: `TestResults/` (compile, smoke test). Context: `UserSettings/LuxUnityContext.json`.
