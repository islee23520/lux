using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor
{
    internal sealed class CaptureAgent : IDisposable
    {
        public const int DefaultWidth = 1280;
        public const int DefaultHeight = 720;
        public const int DefaultFps = 30;
        public const int DefaultQuality = 75;

        private const int FrameQueueCapacity = 2;
        private static readonly object SharedLock = new object();
        private static CaptureAgent sharedAgent;

        private readonly object frameQueueLock = new object();
        private readonly Queue<UnityAiBridgeStreamFramePayload> frameQueue = new Queue<UnityAiBridgeStreamFramePayload>(FrameQueueCapacity);
        private readonly MemoryStream jpegBuffer = new MemoryStream(512 * 1024);
        private readonly int width;
        private readonly int height;
        private readonly int fps;
        private readonly int quality;

        private CancellationTokenSource senderCancellationTokenSource;
        private Task senderTask;
        private RenderTexture renderTexture;
        private Texture2D frameTexture;
        private Camera streamCamera;
        private CaptureAgentRunner runner;
        private Coroutine captureCoroutine;
        private string activeSessionId;
        private long sequence;
        private bool disposed;

        public CaptureAgent(int width = DefaultWidth, int height = DefaultHeight, int fps = DefaultFps, int quality = DefaultQuality, Camera camera = null)
        {
            ValidatePositive(width, nameof(width));
            ValidatePositive(height, nameof(height));
            ValidatePositive(fps, nameof(fps));

            if (quality < 1 || quality > 100)
            {
                throw new ArgumentOutOfRangeException(nameof(quality), "Capture JPEG quality must be between 1 and 100.");
            }

            this.width = width;
            this.height = height;
            this.fps = fps;
            this.quality = quality;
            streamCamera = camera;
        }

        public bool IsRunning => !string.IsNullOrEmpty(activeSessionId);

        public static void StartStream(int width, int height, int fps, string sessionId)
        {
            StartStream(width, height, fps, DefaultQuality, sessionId, null);
        }

        public static void StartStream(int width, int height, int fps, int quality, string sessionId)
        {
            StartStream(width, height, fps, quality, sessionId, null);
        }

        public static void StartStream(int width, int height, int fps, int quality, string sessionId, Camera camera = null)
        {
            if (string.IsNullOrWhiteSpace(sessionId))
            {
                throw new ArgumentException("Capture stream session_id is required.", nameof(sessionId));
            }

            lock (SharedLock)
            {
                StopSharedAgent();
                sharedAgent = new CaptureAgent(width, height, fps, quality, camera);
                try
                {
                    sharedAgent.Start(sessionId);
                }
                catch
                {
                    StopSharedAgent();
                    throw;
                }
            }
        }

        public static void StopStream(string sessionId)
        {
            lock (SharedLock)
            {
                if (sharedAgent == null)
                {
                    return;
                }

                if (!string.IsNullOrEmpty(sharedAgent.activeSessionId)
                    && !string.IsNullOrEmpty(sessionId)
                    && !string.Equals(sharedAgent.activeSessionId, sessionId, StringComparison.Ordinal))
                {
                    return;
                }

                StopSharedAgent();
            }
        }

        public void Start(string sessionId)
        {
            ThrowIfDisposed();

            if (string.IsNullOrWhiteSpace(sessionId))
            {
                throw new ArgumentException("Capture stream session_id is required.", nameof(sessionId));
            }

            Stop();

            streamCamera = streamCamera != null ? streamCamera : Camera.main;
            if (streamCamera == null)
            {
                throw new InvalidOperationException("Camera.main or a configured camera is required to start a LUX capture stream.");
            }

            renderTexture = new RenderTexture(width, height, 24, RenderTextureFormat.ARGB32)
            {
                name = "LuxCaptureStreamRenderTexture",
                hideFlags = HideFlags.HideAndDontSave
            };
            renderTexture.Create();

            frameTexture = new Texture2D(width, height, TextureFormat.RGB24, false)
            {
                name = "LuxCaptureStreamFrameTexture",
                hideFlags = HideFlags.HideAndDontSave
            };

            streamCamera.targetTexture = renderTexture;
            activeSessionId = sessionId;
            sequence = 1;
            AssemblyReloadEvents.beforeAssemblyReload += StopStreamOnEditorLifecycle;
            EditorApplication.quitting += StopStreamOnEditorLifecycle;
            senderCancellationTokenSource = new CancellationTokenSource();
            senderTask = Task.Run(() => SendFrames(senderCancellationTokenSource.Token));
            runner = CreateRunner();
            captureCoroutine = runner.StartCoroutine(CaptureLoop());
        }

        public void Stop()
        {
            var sessionWasActive = !string.IsNullOrEmpty(activeSessionId);
            activeSessionId = null;
            AssemblyReloadEvents.beforeAssemblyReload -= StopStreamOnEditorLifecycle;
            EditorApplication.quitting -= StopStreamOnEditorLifecycle;

            if (runner != null && captureCoroutine != null)
            {
                runner.StopCoroutine(captureCoroutine);
            }

            captureCoroutine = null;

            if (senderCancellationTokenSource != null)
            {
                senderCancellationTokenSource.Cancel();
            }

            if (senderTask != null && !senderTask.IsCompleted)
            {
                try
                {
                    senderTask.Wait(250);
                }
                catch (AggregateException)
                {
                }
            }

            senderTask = null;

            if (senderCancellationTokenSource != null)
            {
                senderCancellationTokenSource.Dispose();
                senderCancellationTokenSource = null;
            }

            lock (frameQueueLock)
            {
                frameQueue.Clear();
                Monitor.PulseAll(frameQueueLock);
            }

            if (streamCamera != null && streamCamera.targetTexture == renderTexture)
            {
                streamCamera.targetTexture = null;
            }

            streamCamera = null;
            sequence = 0;

            if (renderTexture != null)
            {
                renderTexture.Release();
                UnityEngine.Object.DestroyImmediate(renderTexture);
                renderTexture = null;
            }

            if (frameTexture != null)
            {
                UnityEngine.Object.DestroyImmediate(frameTexture);
                frameTexture = null;
            }

            if (runner != null)
            {
                UnityEngine.Object.DestroyImmediate(runner.gameObject);
                runner = null;
            }

            if (sessionWasActive)
            {
                jpegBuffer.SetLength(0);
            }
        }

        public void Dispose()
        {
            if (disposed)
            {
                return;
            }

            Stop();
            jpegBuffer.Dispose();
            disposed = true;
        }

        private IEnumerator CaptureLoop()
        {
            var wait = new WaitForSeconds(1f / fps);
            while (!string.IsNullOrEmpty(activeSessionId))
            {
                yield return new WaitForEndOfFrame();
                CaptureFrame();
                yield return wait;
            }
        }

        private void CaptureFrame()
        {
            if (streamCamera == null || renderTexture == null || frameTexture == null || string.IsNullOrEmpty(activeSessionId))
            {
                return;
            }

            var previousActive = RenderTexture.active;
            try
            {
                streamCamera.Render();
                RenderTexture.active = renderTexture;
                frameTexture.ReadPixels(new Rect(0, 0, renderTexture.width, renderTexture.height), 0, 0, false);
                frameTexture.Apply(false);

                var jpgBytes = ImageConversion.EncodeToJPG(frameTexture, quality);
                jpegBuffer.SetLength(0);
                jpegBuffer.Write(jpgBytes, 0, jpgBytes.Length);

                EnqueueFrame(new UnityAiBridgeStreamFramePayload
                {
                    session_id = activeSessionId,
                    frame = Convert.ToBase64String(jpegBuffer.GetBuffer(), 0, (int)jpegBuffer.Length),
                    sequence = sequence++,
                    timestamp = DateTime.UtcNow.ToString("O")
                });
            }
            finally
            {
                RenderTexture.active = previousActive;
            }
        }

        private void EnqueueFrame(UnityAiBridgeStreamFramePayload frame)
        {
            lock (frameQueueLock)
            {
                while (frameQueue.Count >= FrameQueueCapacity)
                {
                    frameQueue.Dequeue();
                }

                frameQueue.Enqueue(frame);
                Monitor.Pulse(frameQueueLock);
            }
        }

        private void SendFrames(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                UnityAiBridgeStreamFramePayload frame;
                lock (frameQueueLock)
                {
                    while (frameQueue.Count == 0 && !cancellationToken.IsCancellationRequested)
                    {
                        Monitor.Wait(frameQueueLock, 100);
                    }

                    if (cancellationToken.IsCancellationRequested)
                    {
                        return;
                    }

                    frame = frameQueue.Dequeue();
                }

                UnityAiBridgeTcpServer.BroadcastCommand(UnityAiBridgeProtocol.CommandLuxStreamFrame, frame);
            }
        }

        private static CaptureAgentRunner CreateRunner()
        {
            var gameObject = new GameObject("LuxCaptureAgent")
            {
                hideFlags = HideFlags.HideAndDontSave
            };
            return gameObject.AddComponent<CaptureAgentRunner>();
        }

        private static void StopSharedAgent()
        {
            if (sharedAgent == null)
            {
                return;
            }

            sharedAgent.Dispose();
            sharedAgent = null;
        }

        private static void StopStreamOnEditorLifecycle()
        {
            lock (SharedLock)
            {
                StopSharedAgent();
            }
        }

        private static void ValidatePositive(int value, string parameterName)
        {
            if (value <= 0)
            {
                throw new ArgumentOutOfRangeException(parameterName, "Capture configuration values must be greater than zero.");
            }
        }

        private void ThrowIfDisposed()
        {
            if (disposed)
            {
                throw new ObjectDisposedException(nameof(CaptureAgent));
            }
        }

        private sealed class CaptureAgentRunner : MonoBehaviour
        {
        }
    }
}
