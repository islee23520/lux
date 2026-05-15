using NUnit.Framework;
using UnityEditor;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    public sealed class UnityAiSelectionSnapshotCollectorTests
    {
        [Test]
        public void CaptureCurrentSnapshot_EmptySelection_ReturnsEmptySnapshot()
        {
            var previousSelection = Selection.objects;

            try
            {
                Selection.objects = new Object[0];

                var snapshot = UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();

                Assert.That(snapshot, Is.Not.Null);
                Assert.That(snapshot.selectionCount, Is.EqualTo(0));
                Assert.That(snapshot.selectedFiles, Is.Not.Null);
                Assert.That(snapshot.selectedFiles, Is.Empty);
            }
            finally
            {
                Selection.objects = previousSelection;
            }
        }

        [Test]
        public void CaptureCurrentSnapshot_SelectedTempAssetIncludesMetadata()
        {
            var previousSelection = Selection.objects;

            using (var scope = TemporaryAssetScope.Create())
            {
                var assetPath = scope.CreateTextAsset("selection-metadata", "metadata payload");
                var asset = AssetDatabase.LoadAssetAtPath<TextAsset>(assetPath);

                Assert.That(asset, Is.Not.Null);

                try
                {
                    Selection.objects = new Object[] { asset };

                    var snapshot = UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();
                    var metadata = snapshot.selectedFiles[0];

                    Assert.That(snapshot.selectionCount, Is.EqualTo(1));
                    Assert.That(metadata.selectionIndex, Is.EqualTo(0));
                    Assert.That(metadata.selectionCount, Is.EqualTo(1));
                    Assert.That(metadata.assetPath, Is.EqualTo(assetPath));
                    Assert.That(metadata.absolutePath, Is.Not.Empty);
                    Assert.That(metadata.guid, Is.Not.Empty);
                    Assert.That(metadata.name, Is.EqualTo("selection-metadata"));
                    Assert.That(metadata.extension, Is.EqualTo(".txt"));
                    Assert.That(metadata.isFolder, Is.False);
                    Assert.That(metadata.exists, Is.True);
                    Assert.That(metadata.mainAssetType, Is.Not.Empty);
                    Assert.That(metadata.fileSizeBytes, Is.GreaterThan(0));
                    Assert.That(metadata.lastModifiedUtc, Is.Not.Empty);
                }
                finally
                {
                    Selection.objects = previousSelection;
                }
            }
        }

        [Test]
        public void CaptureCurrentSnapshot_RoundTripsThroughJsonUtility()
        {
            var previousSelection = Selection.objects;

            using (var scope = TemporaryAssetScope.Create())
            {
                var assetPath = scope.CreateTextAsset("json-roundtrip", "roundtrip payload");
                var asset = AssetDatabase.LoadAssetAtPath<TextAsset>(assetPath);

                Assert.That(asset, Is.Not.Null);

                try
                {
                    Selection.objects = new Object[] { asset };

                    var snapshot = UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();
                    var json = JsonUtility.ToJson(snapshot, false);
                    var roundTripped = JsonUtility.FromJson<UnityAiSelectionSnapshot>(json);

                    Assert.That(roundTripped, Is.Not.Null);
                    Assert.That(roundTripped.selectionCount, Is.EqualTo(1));
                    Assert.That(roundTripped.selectedFiles, Is.Not.Null);
                    Assert.That(roundTripped.selectedFiles.Length, Is.EqualTo(1));
                    Assert.That(roundTripped.selectedFiles[0].assetPath, Is.EqualTo(assetPath));
                    Assert.That(roundTripped.selectedFiles[0].selectionIndex, Is.EqualTo(0));
                }
                finally
                {
                    Selection.objects = previousSelection;
                }
            }
        }

        [Test]
        public void GetCachedSnapshot_ReturnsDefensiveCopy()
        {
            var previousSelection = Selection.objects;

            using (var scope = TemporaryAssetScope.Create())
            {
                var assetPath = scope.CreateTextAsset("defensive-copy", "copy payload");
                var asset = AssetDatabase.LoadAssetAtPath<TextAsset>(assetPath);

                Assert.That(asset, Is.Not.Null);

                try
                {
                    Selection.objects = new Object[] { asset };

                    var firstSnapshot = UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();
                    var cachedSnapshot = UnityAiSelectionSnapshotCollector.GetCachedSnapshot();
                    cachedSnapshot.selectedFiles[0].name = "mutated-name";

                    var freshSnapshot = UnityAiSelectionSnapshotCollector.GetCachedSnapshot();

                    Assert.That(firstSnapshot.selectedFiles[0].name, Is.EqualTo("defensive-copy"));
                    Assert.That(freshSnapshot.selectedFiles[0].name, Is.EqualTo("defensive-copy"));
                }
                finally
                {
                    Selection.objects = previousSelection;
                }
            }
        }

        [Test]
        public void CaptureCurrentSnapshot_DoesNotIncludeFileContentFields()
        {
            var previousSelection = Selection.objects;

            using (var scope = TemporaryAssetScope.Create())
            {
                var fileContentsProbe = "file-content-probe-7f8d5c9a";
                var assetPath = scope.CreateTextAsset("content-free-metadata", fileContentsProbe);
                var asset = AssetDatabase.LoadAssetAtPath<TextAsset>(assetPath);

                Assert.That(asset, Is.Not.Null);

                try
                {
                    Selection.objects = new Object[] { asset };

                    var snapshot = UnityAiSelectionSnapshotCollector.CaptureCurrentSnapshot();
                    var json = JsonUtility.ToJson(snapshot, false);

                    Assert.That(json, Does.Not.Contain(fileContentsProbe));
                    Assert.That(typeof(UnityAiSelectedFileMetadata).GetField("contents"), Is.Null);
                    Assert.That(typeof(UnityAiSelectedFileMetadata).GetField("fileContents"), Is.Null);
                    Assert.That(typeof(UnityAiSelectedFileMetadata).GetField("bytes"), Is.Null);
                }
                finally
                {
                    Selection.objects = previousSelection;
                }
            }
        }
    }
}
