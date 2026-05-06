using System.Collections.Generic;
using UnityEngine;

namespace UnityEditor
{
    [CreateAssetMenu(fileName = "LuxRuntimeEventChannel", menuName = "Linalab/Lux Runtime Event Channel")]
    public class LuxRuntimeEventChannel : ScriptableObject
    {
        public void Raise(string eventType, Dictionary<string, object> payload)
        {
            LuxRuntimeEvent.Log(eventType, payload);
        }

        public void Raise(string eventType)
        {
            Raise(eventType, null);
        }
    }
}
