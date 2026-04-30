using System;
using System.Collections.Concurrent;
using System.IO;
using System.Threading;
using Linalab.UnityAiBridge.Editor;
using UnityEditor;
using UnityEngine;

namespace Linalab.Lux.Editor
{
    [InitializeOnLoad]
    public static class LuxCompileWatcher
    {
        const int DebounceMilliseconds = 500;
        const string WatchRelativePath = "Assets/_Main/Scripts";

        static readonly object WatchLock = new object();
        static readonly ConcurrentQueue<Action> MainThreadActions = new ConcurrentQueue<Action>();
        static FileSystemWatcher watcher;
        static Timer debounceTimer;
        static bool isWatching;

        static LuxCompileWatcher()
        {
            EditorApplication.update += ProcessMainThreadActions;
            EditorApplication.quitting += Stop;
            AssemblyReloadEvents.beforeAssemblyReload += Stop;

            if (UnityAiBridgeTcpServer.IsSharedRunning)
            {
                Start();
            }
        }

        [MenuItem("Tools/Linalab/Lux/Toggle Auto-Compile Watch")]
        public static void ToggleAutoCompileWatch()
        {
            if (IsWatching)
            {
                Stop();
            }
            else
            {
                Start();
            }
        }

        public static bool IsWatching
        {
            get
            {
                lock (WatchLock)
                {
                    return isWatching;
                }
            }
        }

        public static void StartIfBridgeRunning()
        {
            if (UnityAiBridgeTcpServer.IsSharedRunning)
            {
                Start();
            }
        }

        public static void Start()
        {
            lock (WatchLock)
            {
                if (isWatching)
                {
                    return;
                }

                var watchPath = Path.Combine(Directory.GetCurrentDirectory(), WatchRelativePath);
                if (!Directory.Exists(watchPath))
                {
                    Debug.LogWarning($"Lux auto-compile watch skipped: {watchPath} does not exist.");
                    return;
                }

                watcher = new FileSystemWatcher(watchPath, "*.cs")
                {
                    IncludeSubdirectories = true,
                    NotifyFilter = NotifyFilters.FileName | NotifyFilters.LastWrite | NotifyFilters.CreationTime | NotifyFilters.Size
                };
                watcher.Changed += OnFileChanged;
                watcher.Created += OnFileChanged;
                watcher.Deleted += OnFileChanged;
                watcher.Renamed += OnFileRenamed;
                watcher.EnableRaisingEvents = true;
                isWatching = true;
            }
        }

        public static void Stop()
        {
            lock (WatchLock)
            {
                isWatching = false;
                if (debounceTimer != null)
                {
                    debounceTimer.Dispose();
                    debounceTimer = null;
                }

                if (watcher != null)
                {
                    watcher.EnableRaisingEvents = false;
                    watcher.Changed -= OnFileChanged;
                    watcher.Created -= OnFileChanged;
                    watcher.Deleted -= OnFileChanged;
                    watcher.Renamed -= OnFileRenamed;
                    watcher.Dispose();
                    watcher = null;
                }
            }
        }

        public static void TriggerCompileRefresh(string reason)
        {
            LuxCompileEventBroadcaster.BroadcastCompileStarted(reason);
            AssetDatabase.Refresh(ImportAssetOptions.ForceUpdate);
        }

        static void OnFileChanged(object sender, FileSystemEventArgs args)
        {
            if (!IsCSharpPath(args.FullPath))
            {
                return;
            }

            DebounceCompileRefresh($"changed:{args.ChangeType}:{args.Name}");
        }

        static void OnFileRenamed(object sender, RenamedEventArgs args)
        {
            if (!IsCSharpPath(args.FullPath) && !IsCSharpPath(args.OldFullPath))
            {
                return;
            }

            DebounceCompileRefresh($"renamed:{args.OldName}->{args.Name}");
        }

        static bool IsCSharpPath(string path)
        {
            return string.Equals(Path.GetExtension(path), ".cs", StringComparison.OrdinalIgnoreCase);
        }

        static void DebounceCompileRefresh(string reason)
        {
            lock (WatchLock)
            {
                if (!isWatching)
                {
                    return;
                }

                if (debounceTimer == null)
                {
                    debounceTimer = new Timer(_ => EnqueueCompileRefresh(reason), null, DebounceMilliseconds, Timeout.Infinite);
                }
                else
                {
                    debounceTimer.Change(DebounceMilliseconds, Timeout.Infinite);
                }
            }
        }

        static void EnqueueCompileRefresh(string reason)
        {
            MainThreadActions.Enqueue(() => TriggerCompileRefresh(reason));
        }

        static void ProcessMainThreadActions()
        {
            while (MainThreadActions.TryDequeue(out var action))
            {
                try
                {
                    action();
                }
                catch (Exception exception)
                {
                    Debug.LogWarning($"Lux auto-compile watch action failed: {exception.Message}");
                }
            }
        }
    }
}
