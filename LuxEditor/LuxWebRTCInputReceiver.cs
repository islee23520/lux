using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Net.WebSockets;
using System.Reflection;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace Linalab.LuxEditor
{
    internal sealed class RemoteInputReceiver
    {
        public event Action<RemoteInputEvent> OnInputEvent;

        public bool ReceiveJson(string json)
        {
            if (string.IsNullOrWhiteSpace(json))
            {
                return false;
            }

            var inputEvent = JsonUtility.FromJson<RemoteInputEvent>(json);
            if (inputEvent == null || string.IsNullOrWhiteSpace(inputEvent.type))
            {
                return false;
            }

            OnInputEvent?.Invoke(inputEvent);
            return true;
        }
    }

    [Serializable]
    public sealed class RemoteInputEvent
    {
        public string type;
        public float x;
        public float y;
        public int button;
        public string key;
        public int touchId;
        public float deltaX;
        public float deltaY;
    }
}
