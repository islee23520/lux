using System;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    internal static class InputHandler
    {
        private const string MouseMove = "mouse_move";
        private const string MouseDown = "mouse_down";
        private const string MouseUp = "mouse_up";
        private const string KeyDown = "key_down";
        private const string KeyUp = "key_up";
        private const string Scroll = "scroll";

        public static void HandleInputEventJson(string json)
        {
            if (string.IsNullOrWhiteSpace(json))
            {
                throw new ArgumentException("Input event JSON is required.", nameof(json));
            }

            var payload = JsonUtility.FromJson<InputEventPayload>(json);
            if (payload == null)
            {
                throw new ArgumentException("Input event JSON could not be parsed.", nameof(json));
            }

            HandleInputEvent(payload.event_type, payload.x, payload.y, payload.button, payload.key_code);
        }

        public static void HandleInputEvent(string eventType, float x, float y, int button, string keyCode)
        {
            if (string.IsNullOrWhiteSpace(eventType))
            {
                throw new ArgumentException("Input event type is required.", nameof(eventType));
            }

            var targetWindow = ResolveTargetWindow();
            if (targetWindow == null)
            {
                throw new InvalidOperationException("No Unity editor window is available for input replay.");
            }

            switch (eventType)
            {
                case MouseMove:
                    SendMouseEvent(targetWindow, EventType.MouseMove, x, y, button);
                    break;
                case MouseDown:
                    SendMouseEvent(targetWindow, EventType.MouseDown, x, y, button);
                    break;
                case MouseUp:
                    SendMouseEvent(targetWindow, EventType.MouseUp, x, y, button);
                    break;
                case Scroll:
                    SendScrollEvent(targetWindow, x, y, button);
                    break;
                case KeyDown:
                    SendKeyboardEvent(targetWindow, keyCode, EventType.KeyDown);
                    break;
                case KeyUp:
                    SendKeyboardEvent(targetWindow, keyCode, EventType.KeyUp);
                    break;
                default:
                    throw new ArgumentOutOfRangeException(nameof(eventType), eventType, "Unsupported LUX input event type.");
            }
        }

        public static void HandleInputEvent(string eventType, float x, float y, int button, string keyCode, float scrollX, float scrollY)
        {
            if (!string.Equals(eventType, Scroll, StringComparison.Ordinal))
            {
                HandleInputEvent(eventType, x, y, button, keyCode);
                return;
            }

            var targetWindow = ResolveTargetWindow();
            if (targetWindow == null)
            {
                throw new InvalidOperationException("No Unity editor window is available for input replay.");
            }

            SendScrollEvent(targetWindow, x, y, scrollX, scrollY);
        }

        private static void SendMouseEvent(EditorWindow targetWindow, EventType eventType, float normalizedX, float normalizedY, int button)
        {
            var mouseEvent = new Event
            {
                type = eventType,
                mousePosition = NormalizedToScreenPixels(normalizedX, normalizedY),
                button = Mathf.Max(0, button),
                clickCount = eventType == EventType.MouseDown ? 1 : 0
            };

            targetWindow.SendEvent(mouseEvent);
            targetWindow.Repaint();
        }

        private static void SendScrollEvent(EditorWindow targetWindow, float normalizedX, float normalizedY, int button)
        {
            SendScrollEvent(targetWindow, normalizedX, normalizedY, 0f, button == 0 ? 1f : button);
        }

        private static void SendScrollEvent(EditorWindow targetWindow, float normalizedX, float normalizedY, float scrollX, float scrollY)
        {
            var scrollEvent = new Event
            {
                type = EventType.ScrollWheel,
                mousePosition = NormalizedToScreenPixels(normalizedX, normalizedY),
                button = 0,
                delta = new Vector2(scrollX, scrollY == 0f ? 1f : scrollY)
            };

            targetWindow.SendEvent(scrollEvent);
            targetWindow.Repaint();
        }

        private static void SendKeyboardEvent(EditorWindow targetWindow, string keyCode, EventType eventType)
        {
            if (string.IsNullOrWhiteSpace(keyCode))
            {
                throw new ArgumentException("Input key_code is required for keyboard events.", nameof(keyCode));
            }

            var keyboardEvent = Event.KeyboardEvent(keyCode);
            keyboardEvent.type = eventType;
            targetWindow.SendEvent(keyboardEvent);
            targetWindow.Repaint();
        }

        private static EditorWindow ResolveTargetWindow()
        {
            var focusedWindow = EditorWindow.focusedWindow;
            if (IsGameView(focusedWindow))
            {
                return focusedWindow;
            }

            var gameView = FindGameView();
            if (gameView != null)
            {
                return gameView;
            }

            return focusedWindow ?? SceneView.lastActiveSceneView;
        }

        private static EditorWindow FindGameView()
        {
            var gameViewType = Type.GetType("UnityEditor.GameView,UnityEditor");
            if (gameViewType == null)
            {
                return null;
            }

            var windows = Resources.FindObjectsOfTypeAll(gameViewType);
            for (var index = 0; index < windows.Length; index++)
            {
                if (windows[index] is EditorWindow window)
                {
                    return window;
                }
            }

            return null;
        }

        private static bool IsGameView(EditorWindow window)
        {
            return window != null && string.Equals(window.GetType().FullName, "UnityEditor.GameView", StringComparison.Ordinal);
        }

        private static Vector2 NormalizedToScreenPixels(float normalizedX, float normalizedY)
        {
            var clampedX = Mathf.Clamp01(normalizedX);
            var clampedY = Mathf.Clamp01(normalizedY);
            var width = Mathf.Max(1, Screen.width);
            var height = Mathf.Max(1, Screen.height);
            return new Vector2(width * clampedX, height * (1f - clampedY));
        }

        [Serializable]
        private sealed class InputEventPayload
        {
            public string session_id;
            public string event_type;
            public float x;
            public float y;
            public int button;
            public string key_code;
        }
    }
}
