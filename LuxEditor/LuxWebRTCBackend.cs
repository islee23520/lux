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
    internal interface IWebRTCBackend
    {
        void Initialize();
        void StartUpdatePump();
        void StopUpdatePump();
        object CreatePeerConnection(IReadOnlyList<LuxIceServer> iceServers);
        object CaptureEditorCamera(int width, int height, int frameRate);
        void AddTrack(object peerConnection, object videoTrack);
        void OnDataChannel(object peerConnection, Action<object> onDataChannel);
        string ReadDataChannelLabel(object dataChannel);
        void OnDataChannelMessage(object dataChannel, Action<string> onMessage);
        void OnIceCandidate(object peerConnection, Action<string, string, int> onIceCandidate);
        Task SetRemoteDescriptionAsync(object peerConnection, string type, string sdp);
        Task<string> CreateAnswerAsync(object peerConnection, CancellationToken cancellationToken);
        Task SetLocalDescriptionAsync(object peerConnection, string type, string sdp);
        void AddIceCandidate(object peerConnection, string candidate, string sdpMid, int sdpMLineIndex);
        void DisposeObject(object instance);
    }

    internal sealed class ReflectionWebRTCBackend : IWebRTCBackend
    {
        private Type webRtcType;
        private MethodInfo updateMethod;
        private IEnumerator updateEnumerator;
        private bool updatePumpRunning;

        public void Initialize()
        {
            webRtcType = FindType("Unity.WebRTC.WebRTC");
            if (webRtcType == null)
            {
                throw new InvalidOperationException("com.unity.webrtc 3.0.0 is required for Lux WebRTC streaming. Install it with Package Manager before starting remote streaming.");
            }

            webRtcType.GetMethod("Initialize", BindingFlags.Public | BindingFlags.Static)?.Invoke(null, null);
            updateMethod = webRtcType.GetMethod("Update", BindingFlags.Public | BindingFlags.Static);
        }

        public void StartUpdatePump()
        {
            if (updatePumpRunning)
            {
                return;
            }

            updatePumpRunning = true;
            updateEnumerator = updateMethod?.Invoke(null, null) as IEnumerator;
            EditorApplication.update += PumpWebRTCUpdate;
        }

        public void StopUpdatePump()
        {
            if (!updatePumpRunning)
            {
                return;
            }

            updatePumpRunning = false;
            EditorApplication.update -= PumpWebRTCUpdate;
            updateEnumerator = null;
        }

        public object CreatePeerConnection(IReadOnlyList<LuxIceServer> iceServers)
        {
            var peerType = FindType("Unity.WebRTC.RTCPeerConnection");
            if (peerType == null)
            {
                throw new InvalidOperationException("Unity.WebRTC.RTCPeerConnection was not found.");
            }

            return Activator.CreateInstance(peerType);
        }

        public object CaptureEditorCamera(int width, int height, int frameRate)
        {
            var camera = Camera.main;
            if (camera == null)
            {
                camera = UnityEngine.Object.FindObjectOfType<Camera>();
            }

            if (camera == null)
            {
                throw new InvalidOperationException("No Unity camera was found to capture for Lux WebRTC streaming.");
            }

            var method = FindExtensionMethod("Unity.WebRTC.CameraExtension", "CaptureStream", typeof(Camera), typeof(int), typeof(int))
                ?? FindExtensionMethod("Unity.WebRTC.CameraExtension", "CaptureStreamTrack", typeof(Camera), typeof(int), typeof(int));
            if (method == null)
            {
                throw new InvalidOperationException("Camera.CaptureStream/CaptureStreamTrack is unavailable. Verify com.unity.webrtc 3.0.0 is installed.");
            }

            return method.Invoke(null, new object[] { camera, width, height });
        }

        public void AddTrack(object peerConnection, object videoTrack)
        {
            InvokeBestMatch(peerConnection, "AddTrack", videoTrack);
        }

        public void OnDataChannel(object peerConnection, Action<object> onDataChannel)
        {
            if (peerConnection == null || onDataChannel == null)
            {
                return;
            }

            var eventInfo = peerConnection.GetType().GetEvent("OnDataChannel");
            if (eventInfo == null)
            {
                return;
            }

            Action<object> handler = channel => onDataChannel?.Invoke(channel);
            eventInfo.AddEventHandler(peerConnection, Delegate.CreateDelegate(eventInfo.EventHandlerType, handler.Target, handler.Method));
        }

        public string ReadDataChannelLabel(object dataChannel)
        {
            return dataChannel?.GetType().GetProperty("Label")?.GetValue(dataChannel) as string ?? string.Empty;
        }

        public void OnDataChannelMessage(object dataChannel, Action<string> onMessage)
        {
            if (dataChannel == null)
            {
                return;
            }

            var eventInfo = dataChannel.GetType().GetEvent("OnMessage");
            if (eventInfo == null)
            {
                return;
            }

            Action<byte[]> bytesHandler = bytes => onMessage?.Invoke(Encoding.UTF8.GetString(bytes ?? new byte[0]));
            eventInfo.AddEventHandler(dataChannel, Delegate.CreateDelegate(eventInfo.EventHandlerType, bytesHandler.Target, bytesHandler.Method));
        }

        public void OnIceCandidate(object peerConnection, Action<string, string, int> onIceCandidate)
        {
            var eventInfo = peerConnection?.GetType().GetEvent("OnIceCandidate");
            if (eventInfo == null)
            {
                return;
            }

            Action<object> handler = candidate =>
            {
                if (candidate == null)
                {
                    return;
                }

                onIceCandidate?.Invoke(
                    ReadString(candidate, "Candidate"),
                    ReadString(candidate, "SdpMid"),
                    ReadInt(candidate, "SdpMLineIndex"));
            };
            eventInfo.AddEventHandler(peerConnection, Delegate.CreateDelegate(eventInfo.EventHandlerType, handler.Target, handler.Method));
        }

        public Task SetRemoteDescriptionAsync(object peerConnection, string type, string sdp)
        {
            return InvokeDescriptionAsync(peerConnection, "SetRemoteDescription", type, sdp);
        }

        public Task<string> CreateAnswerAsync(object peerConnection, CancellationToken cancellationToken)
        {
            var operation = InvokeBestMatch(peerConnection, "CreateAnswer");
            if (operation == null)
            {
                return Task.FromResult((string)null);
            }

            var doneProp = operation.GetType().GetProperty("IsDone");
            if (doneProp == null)
            {
                var desc = ReadProperty(operation, "Desc") ?? operation;
                return Task.FromResult(ReadString(desc, "sdp"));
            }

            var tcs = new TaskCompletionSource<string>();
            EditorApplication.update += PumpAnswer;
            return tcs.Task;

            void PumpAnswer()
            {
                try
                {
                    if ((bool)doneProp.GetValue(operation))
                    {
                        EditorApplication.update -= PumpAnswer;
                        var desc = ReadProperty(operation, "Desc");
                        var sdp = desc != null ? ReadString(desc, "sdp") : string.Empty;
                        tcs.TrySetResult(sdp);
                    }
                }
                catch (Exception ex)
                {
                    EditorApplication.update -= PumpAnswer;
                    tcs.TrySetException(ex);
                }
            }
        }

        public Task SetLocalDescriptionAsync(object peerConnection, string type, string sdp)
        {
            return InvokeDescriptionAsync(peerConnection, "SetLocalDescription", type, sdp);
        }

        public void AddIceCandidate(object peerConnection, string candidate, string sdpMid, int sdpMLineIndex)
        {
            var candidateInitType = FindType("Unity.WebRTC.RTCIceCandidateInit");
            if (candidateInitType == null)
            {
                return;
            }

            var init = Activator.CreateInstance(candidateInitType);
            WritePropertyOrField(init, "candidate", candidate);
            WritePropertyOrField(init, "sdpMid", sdpMid);
            WritePropertyOrField(init, "sdpMLineIndex", (int?)sdpMLineIndex);

            var candidateType = FindType("Unity.WebRTC.RTCIceCandidate");
            if (candidateType == null)
            {
                return;
            }

            var instance = Activator.CreateInstance(candidateType, init);
            InvokeBestMatch(peerConnection, "AddIceCandidate", instance);
        }

        public void DisposeObject(object instance)
        {
            (instance as IDisposable)?.Dispose();
        }

        private void PumpWebRTCUpdate()
        {
            if (updateEnumerator != null)
            {
                if (!updateEnumerator.MoveNext())
                {
                    updateEnumerator = null;
                }

                return;
            }

            updateMethod?.Invoke(null, null);
        }

        private void InvokeDescription(object peerConnection, string methodName, string type, string sdp)
        {
            var descType = FindType("Unity.WebRTC.RTCSessionDescription");
            if (descType == null)
            {
                InvokeBestMatch(peerConnection, methodName, sdp);
                return;
            }

            var desc = Activator.CreateInstance(descType);
            WritePropertyOrField(desc, "type", ParseDescriptionType(type));
            WritePropertyOrField(desc, "sdp", sdp);
            InvokeBestMatchRef(peerConnection, methodName, desc);
        }

        private Task InvokeDescriptionAsync(object peerConnection, string methodName, string type, string sdp)
        {
            var descType = FindType("Unity.WebRTC.RTCSessionDescription");
            if (descType == null)
            {
                InvokeBestMatch(peerConnection, methodName, sdp);
                return Task.CompletedTask;
            }

            var desc = Activator.CreateInstance(descType);
            WritePropertyOrField(desc, "type", ParseDescriptionType(type));
            WritePropertyOrField(desc, "sdp", sdp);
            var operation = InvokeBestMatchRef(peerConnection, methodName, desc);
            if (operation == null)
            {
                return Task.CompletedTask;
            }

            var doneProp = operation.GetType().GetProperty("IsDone");
            if (doneProp == null)
            {
                return Task.CompletedTask;
            }

            var tcs = new TaskCompletionSource<bool>();
            EditorApplication.update += PumpDone;
            return tcs.Task;

            void PumpDone()
            {
                try
                {
                    if ((bool)doneProp.GetValue(operation))
                    {
                        EditorApplication.update -= PumpDone;
                        tcs.TrySetResult(true);
                    }
                }
                catch (Exception ex)
                {
                    EditorApplication.update -= PumpDone;
                    tcs.TrySetException(ex);
                }
            }
        }

        private static object ParseDescriptionType(string type)
        {
            var enumType = FindType("Unity.WebRTC.RTCSdpType");
            return enumType == null ? type : Enum.Parse(enumType, type, true);
        }

        private static object InvokeBestMatch(object target, string methodName, params object[] arguments)
        {
            if (target == null)
            {
                return null;
            }

            var methods = target.GetType().GetMethods(BindingFlags.Public | BindingFlags.Instance);
            for (var index = 0; index < methods.Length; index++)
            {
                var method = methods[index];
                if (method.Name == methodName && method.GetParameters().Length == arguments.Length)
                {
                    return method.Invoke(target, arguments);
                }
            }

            return null;
        }
        private static object InvokeBestMatchRef(object target, string methodName, object arg)
        {
            if (target == null)
            {
                return null;
            }

            var methods = target.GetType().GetMethods(BindingFlags.Public | BindingFlags.Instance);
            for (var index = 0; index < methods.Length; index++)
            {
                var method = methods[index];
                if (method.Name != methodName)
                {
                    continue;
                }

                var parameters = method.GetParameters();
                if (parameters.Length != 1)
                {
                    continue;
                }

                if (!parameters[0].ParameterType.IsByRef)
                {
                    return method.Invoke(target, new object[] { arg });
                }

                var args = new object[1];
                args[0] = arg;
                var result = method.Invoke(target, args);
                return result;
            }

            return InvokeBestMatch(target, methodName, arg);
        }

        private static Type FindType(string fullName)
        {
            var type = Type.GetType(fullName);
            if (type != null)
            {
                return type;
            }

            var assemblies = AppDomain.CurrentDomain.GetAssemblies();
            for (var index = 0; index < assemblies.Length; index++)
            {
                type = assemblies[index].GetType(fullName);
                if (type != null)
                {
                    return type;
                }
            }

            return null;
        }

        private static MethodInfo FindStaticMethod(string typeName, string methodName, params Type[] parameterTypes)
        {
            var type = FindType(typeName);
            if (type == null)
            {
                return null;
            }

            var methods = type.GetMethods(BindingFlags.Public | BindingFlags.Static);
            for (var index = 0; index < methods.Length; index++)
            {
                var method = methods[index];
                var parameters = method.GetParameters();
                if (!string.Equals(method.Name, methodName, StringComparison.Ordinal) || parameters.Length != parameterTypes.Length)
                {
                    continue;
                }

                var matches = true;
                for (var parameterIndex = 0; parameterIndex < parameters.Length; parameterIndex++)
                {
                    if (parameters[parameterIndex].ParameterType != parameterTypes[parameterIndex])
                    {
                        matches = false;
                        break;
                    }
                }

                if (matches)
                {
                    return method;
                }
            }

            return null;
        }

        private static MethodInfo FindExtensionMethod(string typeName, string methodName, params Type[] firstParamTypes)
        {
            var type = FindType(typeName);
            if (type == null)
            {
                return null;
            }

            var methods = type.GetMethods(BindingFlags.Public | BindingFlags.Static);
            for (var index = 0; index < methods.Length; index++)
            {
                var method = methods[index];
                if (!string.Equals(method.Name, methodName, StringComparison.Ordinal))
                {
                    continue;
                }

                var parameters = method.GetParameters();
                if (parameters.Length < firstParamTypes.Length)
                {
                    continue;
                }

                var matches = true;
                for (var i = 0; i < firstParamTypes.Length; i++)
                {
                    if (parameters[i].ParameterType != firstParamTypes[i])
                    {
                        matches = false;
                        break;
                    }
                }

                if (matches)
                {
                    return method;
                }
            }

            return null;
        }

        private static object ReadProperty(object instance, string name)
        {
            return instance?.GetType().GetProperty(name, BindingFlags.Public | BindingFlags.Instance)?.GetValue(instance, null);
        }

        private static string ReadString(object instance, string name)
        {
            var value = ReadProperty(instance, name) ?? instance?.GetType().GetField(name, BindingFlags.Public | BindingFlags.Instance)?.GetValue(instance);
            return value == null ? string.Empty : value.ToString();
        }

        private static int ReadInt(object instance, string name)
        {
            var value = ReadProperty(instance, name) ?? instance?.GetType().GetField(name, BindingFlags.Public | BindingFlags.Instance)?.GetValue(instance);
            return value == null ? 0 : Convert.ToInt32(value);
        }

        private static void WritePropertyOrField(object instance, string name, object value)
        {
            var property = instance.GetType().GetProperty(name, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase);
            if (property != null && property.CanWrite)
            {
                property.SetValue(instance, value, null);
                return;
            }

            var field = instance.GetType().GetField(name, BindingFlags.Public | BindingFlags.Instance | BindingFlags.IgnoreCase);
            if (field != null)
            {
                field.SetValue(instance, value);
            }
        }
    }
}
