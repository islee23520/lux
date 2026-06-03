using Linalab.UnityAiBridge.Editor.Ast;
using NUnit.Framework;
using UnityEngine;

namespace Linalab.UnityAiBridge.Editor.Tests
{
    public sealed class UnityAiBridgeAstContractTests
    {
        [Test]
        public void AstNodeIdentity_RoundTripsThroughJsonUtility()
        {
            var node = new UnityAstNode
            {
                id = "g0",
                stableId = "scene-root/player",
                hierarchyPath = "/SceneRoot/Player",
                name = "Player",
                activeSelf = true,
                layer = 0,
                tag = "Player",
                coordinateMappings = new[]
                {
                    UnityCoordinateMapping.Create(UnityCoordinateFrame.World, "meters", "unity_world", Vector3.zero),
                    UnityCoordinateMapping.Create(UnityCoordinateFrame.UiCanvas, "canvas_units", "canvas_bottom_left", new Vector3(10f, 20f, 0f))
                },
                components = new UnityAstComponent[0],
                children = new UnityAstNode[0]
            };

            var json = JsonUtility.ToJson(node);
            var loaded = JsonUtility.FromJson<UnityAstNode>(json);

            Assert.That(loaded.stableId, Is.EqualTo("scene-root/player"));
            Assert.That(loaded.hierarchyPath, Is.EqualTo("/SceneRoot/Player"));
            Assert.That(loaded.coordinateMappings.Length, Is.EqualTo(2));
            Assert.That(loaded.coordinateMappings[1].frame, Is.EqualTo(UnityCoordinateFrame.UiCanvas));
        }

        [Test]
        public void CoordinateMapping_DistinguishesAllSupportedFrames()
        {
            var payload = new UnityCoordinateMappingPayload
            {
                nodeId = "scene-root/player",
                mappings = new[]
                {
                    UnityCoordinateMapping.Create(UnityCoordinateFrame.World, "meters", "unity_world", Vector3.zero),
                    UnityCoordinateMapping.Create(UnityCoordinateFrame.Local, "meters", "parent_local", Vector3.zero),
                    UnityCoordinateMapping.Create(UnityCoordinateFrame.Screen, "pixels", "screen_bottom_left", Vector3.zero),
                    UnityCoordinateMapping.Create(UnityCoordinateFrame.Viewport, "normalized", "viewport_bottom_left", Vector3.zero),
                    UnityCoordinateMapping.Create(UnityCoordinateFrame.UiCanvas, "canvas_units", "canvas_bottom_left", Vector3.zero),
                    UnityCoordinateMapping.Create(UnityCoordinateFrame.Input, "screen_pixels", "input_screen", Vector3.zero)
                }
            };

            var json = JsonUtility.ToJson(payload);
            var loaded = JsonUtility.FromJson<UnityCoordinateMappingPayload>(json);

            Assert.That(loaded.mappings.Length, Is.EqualTo(6));
            Assert.That(loaded.mappings[0].frame, Is.EqualTo(UnityCoordinateFrame.World));
            Assert.That(loaded.mappings[5].frame, Is.EqualTo(UnityCoordinateFrame.Input));
        }
    }
}
