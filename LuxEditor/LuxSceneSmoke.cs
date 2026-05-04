using System;
using System.IO;
using System.Text;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;

namespace Linalab.Lux.Editor
{
    /// <summary>
    /// Lux-native scene smoke automation for verifying Unity control through Lux.
    /// Invoked via `lux unity scene-smoke`.
    /// </summary>
    public static class LuxSceneSmoke
    {
        public const string ResultRelativePath = "TestResults/LuxSceneSmokeResult.json";

        const string DefaultScenePath = "Assets/_Main/Scenes/GamePlay.unity";
        const string RootName = "LuxSkillTestObjects";
        const string ObjectPrefix = "LuxSkillTestObject_";
        const double TimeoutSeconds = 30.0d;
        const double PlayerFindTimeoutSeconds = 10.0d;
        const double InputHoldSeconds = 1.0d;

        static State _state;
        static double _deadline;
        static double _inputReleaseTime;
        static int _objectCount;
        static string _scenePath;
        static string _message;
        static bool _objectsCreated;
        static bool _playModeEntered;
        static double _playerFindDeadline;
        static bool _playerFound;
        static bool _keyboardAvailable;
        static bool _movementObserved;
        static bool _playerOnlySuccess;
        static string _playerObjectName;
        static Vector3 _initialPosition;
        static Vector3 _finalPosition;
        static bool _exitEditorOnComplete;

        static LuxSceneSmoke()
        {
            EditorApplication.playModeStateChanged -= OnPlayModeStateChanged;
            EditorApplication.playModeStateChanged += OnPlayModeStateChanged;
        }

        enum State
        {
            None,
            EnteringPlayMode,
            PressingInput,
            WaitingForMovement,
            ExitingPlayMode,
            Complete,
            Failed
        }

        [MenuItem("Tools/Linalab/Lux/Scene Smoke/Create 10 Objects And Test Player")]
        public static void Run()
        {
            RunInternal(exitEditorOnComplete: true);
        }

        public static string RunLive(int objectCount, string scenePath)
        {
            if (_state != State.None && _state != State.Complete && _state != State.Failed)
            {
                return "Lux scene smoke is already running.";
            }

            Environment.SetEnvironmentVariable("LUX_SCENE_SMOKE_OBJECT_COUNT", objectCount.ToString());
            Environment.SetEnvironmentVariable("LUX_SCENE_SMOKE_SCENE_PATH", scenePath ?? string.Empty);
            RunInternal(exitEditorOnComplete: false);
            return "Lux scene smoke started in live Unity Editor. Poll TestResults/LuxSceneSmokeResult.json for completion.";
        }

        public static string CreateObjectsLive(int objectCount, string scenePath)
        {
            if (string.IsNullOrWhiteSpace(scenePath))
            {
                scenePath = DefaultScenePath;
            }

            if (!EditorApplication.isPlaying)
            {
                var activeScene = EditorSceneManager.GetActiveScene();
                if (activeScene.path != scenePath)
                {
                    EditorSceneManager.OpenScene(scenePath, OpenSceneMode.Single);
                }
            }

            ResetResultState();
            _scenePath = scenePath;
            _objectCount = objectCount <= 0 ? 10 : objectCount;
            CreateTestObjects(_objectCount);
            _objectsCreated = true;
            _message = $"Created {_objectCount} GameObjects in live Unity Editor.";
            WriteResult(true);
            return _message;
        }

        public static void ReleaseSyntheticInputState()
        {
            if (Keyboard.current == null)
            {
                return;
            }

            InputSystem.QueueStateEvent(Keyboard.current, new KeyboardState());
            InputSystem.Update();
        }

        static void OnPlayModeStateChanged(PlayModeStateChange state)
        {
            if (state == PlayModeStateChange.ExitingPlayMode || state == PlayModeStateChange.EnteredEditMode)
            {
                ReleaseSyntheticInputState();
            }
        }

        static void RunInternal(bool exitEditorOnComplete)
        {
            _exitEditorOnComplete = exitEditorOnComplete;
            _state = State.None;
            _deadline = EditorApplication.timeSinceStartup + TimeoutSeconds;
            _objectCount = ReadIntEnvironment("LUX_SCENE_SMOKE_OBJECT_COUNT", 10);
            _scenePath = Environment.GetEnvironmentVariable("LUX_SCENE_SMOKE_SCENE_PATH");
            if (string.IsNullOrWhiteSpace(_scenePath))
            {
                _scenePath = DefaultScenePath;
            }

            ResetResultState();

            try
            {
                EditorSceneManager.OpenScene(_scenePath, OpenSceneMode.Single);
                CreateTestObjects(_objectCount);
                _objectsCreated = true;
                _message = $"Created {_objectCount} GameObjects in {_scenePath}.";
                SetHostProjectDebugTargetScene(_scenePath);

                EditorApplication.update -= OnUpdate;
                EditorApplication.update += OnUpdate;
                EditorApplication.EnterPlaymode();
                _state = State.EnteringPlayMode;
            }
            catch (Exception exception)
            {
                Fail($"Scene smoke setup failed: {exception.Message}");
            }
        }

        static void OnUpdate()
        {
            if (EditorApplication.timeSinceStartup > _deadline)
            {
                Fail("Scene smoke timed out.");
                return;
            }

            switch (_state)
            {
                case State.EnteringPlayMode:
                    if (!EditorApplication.isPlaying)
                    {
                        return;
                    }

                    _playModeEntered = true;
                    var player = FindPlayerObject();
                    if (player == null)
                    {
                        if (_playerFindDeadline <= 0.0d)
                        {
                            _playerFindDeadline = EditorApplication.timeSinceStartup + PlayerFindTimeoutSeconds;
                        }

                        if (EditorApplication.timeSinceStartup < _playerFindDeadline)
                        {
                            return;
                        }

                        Fail("PlayMode entered, but no FpsPlayerController or CharacterController player object was found before the player wait timeout.");
                        return;
                    }

                    _playerFound = true;
                    _playerObjectName = player.name;
                    _initialPosition = player.transform.position;
                    _keyboardAvailable = Keyboard.current != null;
                    if (!_keyboardAvailable)
                    {
                        if (Application.isBatchMode)
                        {
                            _playerOnlySuccess = true;
                            _message = "PlayMode entered and player found; keyboard input smoke skipped because batch mode has no Keyboard.current.";
                            EditorApplication.ExitPlaymode();
                            _state = State.ExitingPlayMode;
                            break;
                        }

                        Fail("PlayMode entered and player found, but Input System Keyboard.current is unavailable.");
                        return;
                    }

                    InputSystem.QueueStateEvent(Keyboard.current, new KeyboardState(Key.W));
                    InputSystem.Update();
                    _inputReleaseTime = EditorApplication.timeSinceStartup + InputHoldSeconds;
                    _state = State.PressingInput;
                    break;

                case State.PressingInput:
                    if (EditorApplication.timeSinceStartup < _inputReleaseTime)
                    {
                        return;
                    }

                    ReleaseSyntheticInputState();
                    _state = State.WaitingForMovement;
                    break;

                case State.WaitingForMovement:
                    var movedPlayer = FindPlayerObject();
                    if (movedPlayer == null)
                    {
                        Fail("Player object disappeared during input smoke.");
                        return;
                    }

                    _finalPosition = movedPlayer.transform.position;
                    _movementObserved = Vector3.Distance(_initialPosition, _finalPosition) > 0.01f;
                    _message = _movementObserved
                        ? "Player movement observed after simulated W input."
                        : "W input was simulated, but player movement was not observed.";
                    ReleaseSyntheticInputState();
                    EditorApplication.ExitPlaymode();
                    _state = State.ExitingPlayMode;
                    break;

                case State.ExitingPlayMode:
                    if (EditorApplication.isPlayingOrWillChangePlaymode)
                    {
                        return;
                    }

                    Complete(_movementObserved || _playerOnlySuccess);
                    break;
            }
        }

        static void CreateTestObjects(int count)
        {
            var existingRoot = FindSceneRootObject(RootName);
            if (existingRoot != null)
            {
                UnityEngine.Object.DestroyImmediate(existingRoot);
            }

            var root = new GameObject(RootName);
            for (int i = 0; i < count; i++)
            {
                var cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
                cube.name = ObjectPrefix + i.ToString("00");
                cube.transform.SetParent(root.transform);
                cube.transform.position = new Vector3(i * 1.5f, 0.5f, 5f);
                cube.transform.localScale = Vector3.one;
            }

            Selection.activeObject = root;
        }

        static void SetHostProjectDebugTargetScene(string scenePath)
        {
            string sceneName = Path.GetFileNameWithoutExtension(scenePath);
            if (string.IsNullOrEmpty(sceneName))
            {
                return;
            }

            Type bootstrapType = Type.GetType("NG.Gameplay.Controllers.BootstrapSceneController, NG.Gameplay");
            var setDebugTargetScene = bootstrapType?.GetMethod("SetDebugTargetScene", System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static);
            if (setDebugTargetScene == null)
            {
                return;
            }

            setDebugTargetScene.Invoke(null, new object[] { sceneName });
        }

        static GameObject FindSceneRootObject(string rootName)
        {
            var activeScene = EditorSceneManager.GetActiveScene();
            if (!activeScene.IsValid())
            {
                return null;
            }

            var rootObjects = activeScene.GetRootGameObjects();
            foreach (var rootObject in rootObjects)
            {
                if (rootObject != null && string.Equals(rootObject.name, rootName, StringComparison.Ordinal))
                {
                    return rootObject;
                }
            }

            return null;
        }

        static GameObject FindPlayerObject()
        {
            var behaviours = UnityEngine.Object.FindObjectsByType<MonoBehaviour>(FindObjectsSortMode.None);
            foreach (var behaviour in behaviours)
            {
                if (behaviour == null)
                {
                    continue;
                }

                var typeName = behaviour.GetType().Name;
                if (typeName == "FpsPlayerController")
                {
                    return behaviour.gameObject;
                }
            }

            var controllers = UnityEngine.Object.FindObjectsByType<CharacterController>(FindObjectsSortMode.None);
            return controllers.Length == 0 ? null : controllers[0].gameObject;
        }

        static void Complete(bool success)
        {
            EditorApplication.update -= OnUpdate;
            _state = State.Complete;
            WriteResult(success);
            if (_exitEditorOnComplete)
            {
                EditorApplication.Exit(success ? 0 : 1);
            }
        }

        static void Fail(string message)
        {
            EditorApplication.update -= OnUpdate;
            _state = State.Failed;
            _message = message;
            if (EditorApplication.isPlaying)
            {
                ReleaseSyntheticInputState();
                EditorApplication.ExitPlaymode();
            }
            WriteResult(false);
            if (_exitEditorOnComplete)
            {
                EditorApplication.Exit(1);
            }
        }

        static void ResetResultState()
        {
            _message = string.Empty;
            _objectsCreated = false;
            _playModeEntered = false;
            _playerFindDeadline = 0.0d;
            _playerFound = false;
            _keyboardAvailable = false;
            _movementObserved = false;
            _playerOnlySuccess = false;
            _playerObjectName = string.Empty;
            _initialPosition = Vector3.zero;
            _finalPosition = Vector3.zero;
        }

        static void WriteResult(bool success)
        {
            string projectRoot = LuxBridgeSettings.GetProjectRoot();
            string resultPath = Path.Combine(projectRoot, ResultRelativePath);
            Directory.CreateDirectory(Path.GetDirectoryName(resultPath) ?? projectRoot);

            var builder = new StringBuilder(1024);
            builder.Append('{');
            AppendProperty(builder, "ok", success ? "true" : "false", false, false);
            AppendProperty(builder, "scene_path", _scenePath, true, true);
            AppendProperty(builder, "created_object_count", _objectCount.ToString(), true, false);
            AppendProperty(builder, "objects_created", _objectsCreated ? "true" : "false", true, false);
            AppendProperty(builder, "play_mode_entered", _playModeEntered ? "true" : "false", true, false);
            AppendProperty(builder, "player_found", _playerFound ? "true" : "false", true, false);
            AppendProperty(builder, "keyboard_available", _keyboardAvailable ? "true" : "false", true, false);
            AppendProperty(builder, "movement_observed", _movementObserved ? "true" : "false", true, false);
            AppendProperty(builder, "player_object_name", _playerObjectName, true, true);
            AppendProperty(builder, "initial_position", FormatVector(_initialPosition), true, true);
            AppendProperty(builder, "final_position", FormatVector(_finalPosition), true, true);
            AppendProperty(builder, "message", _message, true, true);
            AppendProperty(builder, "timestamp_utc", DateTime.UtcNow.ToString("o"), true, true);
            builder.Append("}\n");

            File.WriteAllText(resultPath, builder.ToString());
            Debug.Log($"Lux scene smoke result written to {resultPath}: {(success ? "OK" : "FAILED")}");
        }

        static int ReadIntEnvironment(string name, int defaultValue)
        {
            var value = Environment.GetEnvironmentVariable(name);
            return int.TryParse(value, out int parsed) && parsed > 0 ? parsed : defaultValue;
        }

        static string FormatVector(Vector3 value)
        {
            return $"{value.x:0.###},{value.y:0.###},{value.z:0.###}";
        }

        static void AppendProperty(StringBuilder builder, string name, string value, bool comma, bool quoteValue)
        {
            if (comma)
            {
                builder.Append(',');
            }

            builder.Append('"');
            builder.Append(Escape(name));
            builder.Append("\":");
            if (quoteValue)
            {
                builder.Append('"');
                builder.Append(Escape(value));
                builder.Append('"');
            }
            else
            {
                builder.Append(value);
            }
        }

        static string Escape(string value)
        {
            return (value ?? string.Empty)
                .Replace("\\", "\\\\")
                .Replace("\"", "\\\"")
                .Replace("\r", "\\r")
                .Replace("\n", "\\n")
                .Replace("\t", "\\t");
        }
    }
}
