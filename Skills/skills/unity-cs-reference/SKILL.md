---
name: unity-cs-reference
description: "Lazy-load only when this workflow is explicitly needed. Use as a compact lookup guide for Unity Editor API decisions. Prefer public UnityEditor APIs. Key areas: AssetDatabase import/GUID/path flow; BuildPipeline and BuildReport; CompilationPipeline and domain reload; EditorApplication events; UI Toolkit EditorWindow lifecycle; PackageManager requests; serialization and ScriptableObject persistence. Do not copy Unity implementation code."
---

# Unity C# Reference

Developer-facing lookup guide for public Unity C# APIs, organized by namespace and API area.

## Table of Contents

- [About This Reference](#about-this-reference)
- [Lookup Conventions](#lookup-conventions)
- [UnityEngine Core](#unityengine-core)
- [UnityEngine.Math / Geometry](#unityenginemath--geometry)
- [UnityEngine.Physics](#unityenginephysics)
- [UnityEngine.Rendering / Graphics](#unityenginerendering--graphics)
- [UnityEngine.Audio](#unityengineaudio)
- [UnityEngine.Input](#unityengineinput)
- [UnityEngine.SceneManagement](#unityenginescenemanagement)
- [UnityEngine.Serialization](#unityengineserialization)
- [UnityEngine.Debug / Logging / Assertions](#unityenginedebug--logging--assertions)
- [UnityEngine.UIElements (UI Toolkit Runtime)](#unityengineuielements-ui-toolkit-runtime)
- [UnityEditor Core](#unityeditor-core)
- [UnityEditor Asset / Build](#unityeditor-asset--build)
- [UnityEditor UI (IMGUI)](#unityeditor-ui-imgui)
- [UnityEditor UIElements (UI Toolkit Editor)](#unityeditor-uielements-ui-toolkit-editor)
- [Unity.Collections (Native Collections)](#unitycollections-native-collections)
- [Unity.Mathematics (External Package - NOT in UnityCsReference)](#unitymathematics-external-package---not-in-unitycsreference)
- [UnityEngine.Pool](#unityenginepool)
- [UnityEngine.Networking](#unityenginenetworking)
- [UnityEngine.Subsystems](#unityenginesubsystems)
- [Notable Exclusions](#notable-exclusions)
- [Cross-Reference with LUX Bridge](#cross-reference-with-lux-bridge)

## About This Reference

- Based on UnityCsReference repository surface for Unity 6000.2.0b4.
- Intended for LUX agents and developers who need quick Unity API decisions without copying Unity implementation code.
- Lists practical public properties, methods, attributes, events, and common usage patterns.
- API signatures are concise C#-style lookup shapes, not exhaustive overload lists.
- Prefer public UnityEditor APIs for editor automation; avoid internal UnityEditor classes unless no public API exists and the risk is explicit.
- Marked `[Editor]` entries require editor assemblies and must not be compiled into player builds.
- Package sections identify APIs that are separate packages rather than UnityCsReference core.

## Lookup Conventions

- `public var X { get; set; }` means the exact type is obvious from Unity docs or varies by overload; verify exact type when writing production code.
- `Method(...)` groups overloads. Check parameter order when overload ambiguity matters.
- `Use when` gives the common decision point for LUX/Unity automation.
- `Notes` call out lifecycle, allocation, serialization, version, or platform constraints.
- Do not paste Unity implementation code from UnityCsReference; use signatures and behavior only.
- Member entries intentionally group overloads and omit internal/protected implementation details.
- For mutating editor APIs, pair changes with Undo, SerializedObject, SetDirty, SaveAssets, or SaveAndReimport as appropriate.
- For native collections, verify allocator ownership and call Dispose for owned memory.
- For events/delegates, subscribe and unsubscribe at matching lifecycle boundaries.
- For external packages, verify package version because APIs can move faster than UnityCsReference core.

### Common Unity patterns

```csharp
// Runtime component lookup and caching
private Rigidbody _body;
void Awake() => _body = GetComponent<Rigidbody>();

// Editor serialized editing
serializedObject.Update();
EditorGUILayout.PropertyField(serializedObject.FindProperty("fieldName"));
serializedObject.ApplyModifiedProperties();
```

## UnityEngine Core

### Object

- Namespace/type: `UnityEngine.Object`
- Use when: Base engine object identity, naming, cloning, lifetime, and editor visibility.
- Key public API:
  - `name`
    - Signature: `public var name { get; set; }`
  - `hideFlags`
    - Signature: `public var hideFlags { get; set; }`
  - `Instantiate`
    - Signature: `public var Instantiate(...);`
  - `Destroy`
    - Signature: `public var Destroy(...);`
  - `DestroyImmediate`
    - Signature: `public var DestroyImmediate(...);`
  - `DontDestroyOnLoad`
    - Signature: `public var DontDestroyOnLoad(...);`
  - `FindObjectOfType`
    - Signature: `public var FindObjectOfType(...);`
  - `FindObjectsOfType`
    - Signature: `public var FindObjectsOfType(...);`
  - `FindFirstObjectByType`
    - Signature: `public var FindFirstObjectByType(...);`
  - `FindAnyObjectByType`
    - Signature: `public var FindAnyObjectByType(...);`
  - `FindObjectsByType`
    - Signature: `public var FindObjectsByType(...);`

### GameObject

- Namespace/type: `UnityEngine.GameObject`
- Use when: Scene object container for Components, activation, tags, layers, and message dispatch.
- Key public API:
  - `SetActive`
    - Signature: `public var SetActive(...);`
  - `activeSelf`
    - Signature: `public var activeSelf { get; set; }`
  - `activeInHierarchy`
    - Signature: `public var activeInHierarchy { get; set; }`
  - `AddComponent`
    - Signature: `public var AddComponent(...);`
  - `GetComponent`
    - Signature: `public var GetComponent(...);`
  - `GetComponentInChildren`
    - Signature: `public var GetComponentInChildren(...);`
  - `GetComponentInParent`
    - Signature: `public var GetComponentInParent(...);`
  - `GetComponents`
    - Signature: `public var GetComponents(...);`
  - `TryGetComponent`
    - Signature: `public var TryGetComponent(...);`
  - `CompareTag`
    - Signature: `public var CompareTag(...);`
  - `tag`
    - Signature: `public var tag { get; set; }`
  - `layer`
    - Signature: `public var layer { get; set; }`
  - `transform`
    - Signature: `public var transform { get; set; }`
  - `scene`
    - Signature: `public var scene { get; set; }`
  - `SendMessage`
    - Signature: `public var SendMessage(...);`
  - `BroadcastMessage`
    - Signature: `public var BroadcastMessage(...);`
  - `SendMessageUpwards`
    - Signature: `public var SendMessageUpwards(...);`
  - `Find`
    - Signature: `public var Find(...);`
  - `FindWithTag`
    - Signature: `public var FindWithTag(...);`
  - `FindGameObjectsWithTag`
    - Signature: `public var FindGameObjectsWithTag(...);`

### Component

- Namespace/type: `UnityEngine.Component`
- Use when: Base class for objects attached to GameObjects.
- Key public API:
  - `transform`
    - Signature: `public var transform { get; set; }`
  - `gameObject`
    - Signature: `public var gameObject { get; set; }`
  - `GetComponent`
    - Signature: `public var GetComponent(...);`
  - `TryGetComponent`
    - Signature: `public var TryGetComponent(...);`
  - `GetComponentInChildren`
    - Signature: `public var GetComponentInChildren(...);`
  - `GetComponentInParent`
    - Signature: `public var GetComponentInParent(...);`
  - `GetComponents`
    - Signature: `public var GetComponents(...);`
  - `CompareTag`
    - Signature: `public var CompareTag(...);`
  - `tag`
    - Signature: `public var tag { get; set; }`
  - `name`
    - Signature: `public var name { get; set; }`
  - `hideFlags`
    - Signature: `public var hideFlags { get; set; }`
  - `SendMessage`
    - Signature: `public var SendMessage(...);`
  - `BroadcastMessage`
    - Signature: `public var BroadcastMessage(...);`
  - `SendMessageUpwards`
    - Signature: `public var SendMessageUpwards(...);`

### Transform

- Namespace/type: `UnityEngine.Transform`
- Use when: Hierarchy, local/world position, rotation, scale, and child traversal.
- Key public API:
  - `position`
    - Signature: `public var position { get; set; }`
  - `rotation`
    - Signature: `public var rotation { get; set; }`
  - `localPosition`
    - Signature: `public var localPosition { get; set; }`
  - `localRotation`
    - Signature: `public var localRotation { get; set; }`
  - `localScale`
    - Signature: `public var localScale { get; set; }`
  - `parent`
    - Signature: `public var parent { get; set; }`
  - `root`
    - Signature: `public var root { get; set; }`
  - `childCount`
    - Signature: `public var childCount { get; set; }`
  - `lossyScale`
    - Signature: `public var lossyScale { get; set; }`
  - `forward`
    - Signature: `public var forward { get; set; }`
  - `right`
    - Signature: `public var right { get; set; }`
  - `up`
    - Signature: `public var up { get; set; }`
  - `hasChanged`
    - Signature: `public event Action hasChanged;`
  - `hierarchyCapacity`
    - Signature: `public var hierarchyCapacity { get; set; }`
  - `hierarchyCount`
    - Signature: `public var hierarchyCount { get; set; }`
  - `GetChild`
    - Signature: `public var GetChild(...);`
  - `Find`
    - Signature: `public var Find(...);`
  - `Rotate`
    - Signature: `public var Rotate(...);`
  - `RotateAround`
    - Signature: `public var RotateAround(...);`
  - `Translate`
    - Signature: `public var Translate(...);`
  - `LookAt`
    - Signature: `public var LookAt(...);`
  - `SetParent`
    - Signature: `public var SetParent(...);`
  - `DetachChildren`
    - Signature: `public var DetachChildren(...);`
  - `SetAsFirstSibling`
    - Signature: `public var SetAsFirstSibling(...);`
  - `SetAsLastSibling`
    - Signature: `public var SetAsLastSibling(...);`
  - `SetSiblingIndex`
    - Signature: `public var SetSiblingIndex(...);`
  - `GetSiblingIndex`
    - Signature: `public var GetSiblingIndex(...);`
  - `TransformPoint`
    - Signature: `public var TransformPoint(...);`
  - `InverseTransformPoint`
    - Signature: `public var InverseTransformPoint(...);`
  - `TransformDirection`
    - Signature: `public var TransformDirection(...);`
  - `InverseTransformDirection`
    - Signature: `public var InverseTransformDirection(...);`
  - `TransformVector`
    - Signature: `public var TransformVector(...);`
  - `InverseTransformVector`
    - Signature: `public var InverseTransformVector(...);`
  - `IsChildOf`
    - Signature: `public var IsChildOf(...);`

### MonoBehaviour

- Namespace/type: `UnityEngine.MonoBehaviour`
- Use when: Runtime behaviour lifecycle, coroutines, invoking, and enabled state.
- Key public API:
  - `Start`
    - Signature: `public var Start(...);`
  - `Update`
    - Signature: `public var Update(...);`
  - `FixedUpdate`
    - Signature: `public var FixedUpdate(...);`
  - `LateUpdate`
    - Signature: `public var LateUpdate(...);`
  - `Awake`
    - Signature: `public var Awake(...);`
  - `OnEnable`
    - Signature: `public var OnEnable(...);`
  - `OnDisable`
    - Signature: `public var OnDisable(...);`
  - `OnDestroy`
    - Signature: `public var OnDestroy(...);`
  - `OnCollisionEnter`
    - Signature: `public var OnCollisionEnter(...);`
  - `OnCollisionStay`
    - Signature: `public var OnCollisionStay(...);`
  - `OnCollisionExit`
    - Signature: `public var OnCollisionExit(...);`
  - `OnTriggerEnter`
    - Signature: `public var OnTriggerEnter(...);`
  - `OnTriggerStay`
    - Signature: `public var OnTriggerStay(...);`
  - `OnTriggerExit`
    - Signature: `public var OnTriggerExit(...);`
  - `StartCoroutine`
    - Signature: `public var StartCoroutine(...);`
  - `StopCoroutine`
    - Signature: `public var StopCoroutine(...);`
  - `StopAllCoroutines`
    - Signature: `public var StopAllCoroutines(...);`
  - `Invoke`
    - Signature: `public var Invoke(...);`
  - `InvokeRepeating`
    - Signature: `public var InvokeRepeating(...);`
  - `CancelInvoke`
    - Signature: `public var CancelInvoke(...);`
  - `IsInvoking`
    - Signature: `public var IsInvoking(...);`
  - `destroyCancellationToken`
    - Signature: `public var destroyCancellationToken { get; set; }`
  - `gameObject`
    - Signature: `public var gameObject { get; set; }`
  - `transform`
    - Signature: `public var transform { get; set; }`
  - `enabled`
    - Signature: `public var enabled { get; set; }`
  - `isActiveAndEnabled`
    - Signature: `public var isActiveAndEnabled { get; set; }`
  - `useGUILayout`
    - Signature: `public var useGUILayout { get; set; }`
  - `runInEditMode`
    - Signature: `public var runInEditMode { get; set; }`
- Common pattern:
```csharp
void OnEnable() => StartCoroutine(Run());
IEnumerator Run() { yield return new WaitForSeconds(1f); }
void OnDisable() => StopAllCoroutines();
```

### ScriptableObject

- Namespace/type: `UnityEngine.ScriptableObject`
- Use when: Asset or in-memory serializable data object independent of GameObject.
- Key public API:
  - `CreateInstance`
    - Signature: `public var CreateInstance(...);`
  - `OnEnable`
    - Signature: `public var OnEnable(...);`
  - `OnDisable`
    - Signature: `public var OnDisable(...);`
  - `OnDestroy`
    - Signature: `public var OnDestroy(...);`
  - `name`
    - Signature: `public var name { get; set; }`
  - `hideFlags`
    - Signature: `public var hideFlags { get; set; }`

### Coroutine

- Namespace/type: `UnityEngine.Coroutine`
- Use when: Opaque handle returned from MonoBehaviour.StartCoroutine.
- Key public API:
  - `StartCoroutine`
    - Signature: `public var StartCoroutine(...);`
  - `StopCoroutine`
    - Signature: `public var StopCoroutine(...);`

### YieldInstruction

- Namespace/type: `UnityEngine.YieldInstruction`
- Use when: Base class for built-in coroutine waits.
- Key public API:
  - `WaitForSeconds`
    - Signature: `public var WaitForSeconds(...);`
  - `WaitForEndOfFrame`
    - Signature: `public var WaitForEndOfFrame(...);`
  - `WaitForFixedUpdate`
    - Signature: `public var WaitForFixedUpdate(...);`
  - `WaitUntil`
    - Signature: `public var WaitUntil(...);`
  - `WaitWhile`
    - Signature: `public var WaitWhile(...);`
  - `CustomYieldInstruction`
    - Signature: `public var CustomYieldInstruction(...);`

## UnityEngine.Math / Geometry

### Mathf

- Namespace/type: `UnityEngine.Mathf`
- Use when: Float math helpers and constants.
- Key public API:
  - `Sin`
    - Signature: `public static var Sin(...);`
  - `Cos`
    - Signature: `public static var Cos(...);`
  - `Tan`
    - Signature: `public static var Tan(...);`
  - `Asin`
    - Signature: `public static var Asin(...);`
  - `Acos`
    - Signature: `public static var Acos(...);`
  - `Atan`
    - Signature: `public static var Atan(...);`
  - `Atan2`
    - Signature: `public static var Atan2(...);`
  - `Sqrt`
    - Signature: `public static var Sqrt(...);`
  - `Abs`
    - Signature: `public static var Abs(...);`
  - `Min`
    - Signature: `public static var Min(...);`
  - `Max`
    - Signature: `public static var Max(...);`
  - `Clamp`
    - Signature: `public static var Clamp(...);`
  - `Clamp01`
    - Signature: `public static var Clamp01(...);`
  - `Lerp`
    - Signature: `public static var Lerp(...);`
  - `LerpUnclamped`
    - Signature: `public static var LerpUnclamped(...);`
  - `MoveTowards`
    - Signature: `public static var MoveTowards(...);`
  - `SmoothStep`
    - Signature: `public static var SmoothStep(...);`
  - `SmoothDamp`
    - Signature: `public static var SmoothDamp(...);`
  - `SmoothDampAngle`
    - Signature: `public static var SmoothDampAngle(...);`
  - `PingPong`
    - Signature: `public static var PingPong(...);`
  - `Repeat`
    - Signature: `public static var Repeat(...);`
  - `InverseLerp`
    - Signature: `public static var InverseLerp(...);`
  - `ClosestPowerOfTwo`
    - Signature: `public static var ClosestPowerOfTwo(...);`
  - `NextPowerOfTwo`
    - Signature: `public static var NextPowerOfTwo(...);`
  - `IsPowerOfTwo`
    - Signature: `public static var IsPowerOfTwo(...);`
  - `CorruptedAngle`
    - Signature: `public static var CorruptedAngle(...);`
  - `DeltaAngle`
    - Signature: `public static var DeltaAngle(...);`
  - `GammaToLinearSpace`
    - Signature: `public static var GammaToLinearSpace(...);`
  - `LinearToGammaSpace`
    - Signature: `public static var LinearToGammaSpace(...);`
  - `Approximately`
    - Signature: `public static var Approximately(...);`
  - `Round`
    - Signature: `public static var Round(...);`
  - `RoundToInt`
    - Signature: `public static var RoundToInt(...);`
  - `Floor`
    - Signature: `public static var Floor(...);`
  - `FloorToInt`
    - Signature: `public static var FloorToInt(...);`
  - `Ceil`
    - Signature: `public static var Ceil(...);`
  - `CeilToInt`
    - Signature: `public static var CeilToInt(...);`
  - `Sign`
    - Signature: `public static var Sign(...);`
  - `Pow`
    - Signature: `public static var Pow(...);`
  - `Exp`
    - Signature: `public static var Exp(...);`
  - `Log`
    - Signature: `public static var Log(...);`
  - `Log10`
    - Signature: `public static var Log10(...);`
  - `PI`
    - Signature: `public var PI { get; set; }`
  - `Infinity`
    - Signature: `public var Infinity { get; set; }`
  - `NegativeInfinity`
    - Signature: `public var NegativeInfinity { get; set; }`
  - `Deg2Rad`
    - Signature: `public var Deg2Rad { get; set; }`
  - `Rad2Deg`
    - Signature: `public var Rad2Deg { get; set; }`
  - `Epsilon`
    - Signature: `public var Epsilon { get; set; }`
- Note: `CorruptedAngle` is not a public Unity Mathf API in current references; use `DeltaAngle` or `SmoothDampAngle` after verifying intent.

### Vector2

- Namespace/type: `UnityEngine.Vector2`
- Use when: 2D vector math.
- Key public API:
  - `x`
    - Signature: `public var x { get; set; }`
  - `y`
    - Signature: `public var y { get; set; }`
  - `magnitude`
    - Signature: `public var magnitude { get; set; }`
  - `sqrMagnitude`
    - Signature: `public var sqrMagnitude { get; set; }`
  - `normalized`
    - Signature: `public var normalized { get; set; }`
  - `zero`
    - Signature: `public var zero { get; set; }`
  - `one`
    - Signature: `public event Action one;`
  - `up`
    - Signature: `public var up { get; set; }`
  - `down`
    - Signature: `public var down { get; set; }`
  - `left`
    - Signature: `public var left { get; set; }`
  - `right`
    - Signature: `public var right { get; set; }`
  - `positiveInfinity`
    - Signature: `public var positiveInfinity { get; set; }`
  - `negativeInfinity`
    - Signature: `public var negativeInfinity { get; set; }`
  - `+`
    - Signature: `public static operator +(...);`
  - `-`
    - Signature: `public static operator -(...);`
  - `*`
    - Signature: `public static operator *(...);`
  - `/`
    - Signature: `public static operator /(...);`
  - `==`
    - Signature: `public static operator ==(...);`
  - `!=`
    - Signature: `public static operator !=(...);`
  - `Dot`
    - Signature: `public var Dot(...);`
  - `Cross`
    - Signature: `public var Cross(...);`
  - `Lerp`
    - Signature: `public var Lerp(...);`
  - `LerpUnclamped`
    - Signature: `public var LerpUnclamped(...);`
  - `MoveTowards`
    - Signature: `public var MoveTowards(...);`
  - `Scale`
    - Signature: `public var Scale(...);`
  - `ClampMagnitude`
    - Signature: `public var ClampMagnitude(...);`
  - `Reflect`
    - Signature: `public var Reflect(...);`
  - `Perpendicular`
    - Signature: `public var Perpendicular(...);`
  - `Angle`
    - Signature: `public var Angle(...);`
  - `SignedAngle`
    - Signature: `public var SignedAngle(...);`
  - `Distance`
    - Signature: `public var Distance(...);`
  - `Min`
    - Signature: `public var Min(...);`
  - `Max`
    - Signature: `public var Max(...);`
  - `Normalize`
    - Signature: `public var Normalize(...);`
  - `Set`
    - Signature: `public var Set(...);`
  - `ToString`
    - Signature: `public var ToString(...);`

### Vector3

- Namespace/type: `UnityEngine.Vector3`
- Use when: 3D vector math.
- Key public API:
  - `x`
    - Signature: `public var x { get; set; }`
  - `y`
    - Signature: `public var y { get; set; }`
  - `z`
    - Signature: `public var z { get; set; }`
  - `magnitude`
    - Signature: `public var magnitude { get; set; }`
  - `sqrMagnitude`
    - Signature: `public var sqrMagnitude { get; set; }`
  - `normalized`
    - Signature: `public var normalized { get; set; }`
  - `zero`
    - Signature: `public var zero { get; set; }`
  - `one`
    - Signature: `public event Action one;`
  - `up`
    - Signature: `public var up { get; set; }`
  - `down`
    - Signature: `public var down { get; set; }`
  - `left`
    - Signature: `public var left { get; set; }`
  - `right`
    - Signature: `public var right { get; set; }`
  - `forward`
    - Signature: `public var forward { get; set; }`
  - `back`
    - Signature: `public var back { get; set; }`
  - `positiveInfinity`
    - Signature: `public var positiveInfinity { get; set; }`
  - `negativeInfinity`
    - Signature: `public var negativeInfinity { get; set; }`
  - `+`
    - Signature: `public static operator +(...);`
  - `-`
    - Signature: `public static operator -(...);`
  - `*`
    - Signature: `public static operator *(...);`
  - `/`
    - Signature: `public static operator /(...);`
  - `==`
    - Signature: `public static operator ==(...);`
  - `!=`
    - Signature: `public static operator !=(...);`
  - `Dot`
    - Signature: `public var Dot(...);`
  - `Cross`
    - Signature: `public var Cross(...);`
  - `Lerp`
    - Signature: `public var Lerp(...);`
  - `LerpUnclamped`
    - Signature: `public var LerpUnclamped(...);`
  - `MoveTowards`
    - Signature: `public var MoveTowards(...);`
  - `Scale`
    - Signature: `public var Scale(...);`
  - `ClampMagnitude`
    - Signature: `public var ClampMagnitude(...);`
  - `Reflect`
    - Signature: `public var Reflect(...);`
  - `Angle`
    - Signature: `public var Angle(...);`
  - `SignedAngle`
    - Signature: `public var SignedAngle(...);`
  - `Distance`
    - Signature: `public var Distance(...);`
  - `Min`
    - Signature: `public var Min(...);`
  - `Max`
    - Signature: `public var Max(...);`
  - `Project`
    - Signature: `public var Project(...);`
  - `ProjectOnPlane`
    - Signature: `public var ProjectOnPlane(...);`
  - `OrthoNormalize`
    - Signature: `public var OrthoNormalize(...);`
  - `RotateTowards`
    - Signature: `public var RotateTowards(...);`
  - `Slerp`
    - Signature: `public var Slerp(...);`
  - `SlerpUnclamped`
    - Signature: `public var SlerpUnclamped(...);`
  - `Normalize`
    - Signature: `public var Normalize(...);`
  - `Set`
    - Signature: `public var Set(...);`
  - `ToString`
    - Signature: `public var ToString(...);`

### Vector4

- Namespace/type: `UnityEngine.Vector4`
- Use when: 4D vector and homogeneous coordinates.
- Key public API:
  - `x`
    - Signature: `public var x { get; set; }`
  - `y`
    - Signature: `public var y { get; set; }`
  - `z`
    - Signature: `public var z { get; set; }`
  - `w`
    - Signature: `public var w { get; set; }`
  - `magnitude`
    - Signature: `public var magnitude { get; set; }`
  - `sqrMagnitude`
    - Signature: `public var sqrMagnitude { get; set; }`
  - `normalized`
    - Signature: `public var normalized { get; set; }`
  - `zero`
    - Signature: `public var zero { get; set; }`
  - `one`
    - Signature: `public event Action one;`
  - `positiveInfinity`
    - Signature: `public var positiveInfinity { get; set; }`
  - `negativeInfinity`
    - Signature: `public var negativeInfinity { get; set; }`
  - `+`
    - Signature: `public static operator +(...);`
  - `-`
    - Signature: `public static operator -(...);`
  - `*`
    - Signature: `public static operator *(...);`
  - `/`
    - Signature: `public static operator /(...);`
  - `==`
    - Signature: `public static operator ==(...);`
  - `!=`
    - Signature: `public static operator !=(...);`
  - `Dot`
    - Signature: `public var Dot(...);`
  - `Lerp`
    - Signature: `public var Lerp(...);`
  - `LerpUnclamped`
    - Signature: `public var LerpUnclamped(...);`
  - `MoveTowards`
    - Signature: `public var MoveTowards(...);`
  - `Project`
    - Signature: `public var Project(...);`
  - `Scale`
    - Signature: `public var Scale(...);`
  - `Distance`
    - Signature: `public var Distance(...);`
  - `Min`
    - Signature: `public var Min(...);`
  - `Max`
    - Signature: `public var Max(...);`
  - `Normalize`
    - Signature: `public var Normalize(...);`
  - `Set`
    - Signature: `public var Set(...);`
  - `ToString`
    - Signature: `public var ToString(...);`

### Quaternion

- Namespace/type: `UnityEngine.Quaternion`
- Use when: Rotation without gimbal lock.
- Key public API:
  - `x`
    - Signature: `public var x { get; set; }`
  - `y`
    - Signature: `public var y { get; set; }`
  - `z`
    - Signature: `public var z { get; set; }`
  - `w`
    - Signature: `public var w { get; set; }`
  - `identity`
    - Signature: `public var identity { get; set; }`
  - `Euler`
    - Signature: `public var Euler(...);`
  - `AngleAxis`
    - Signature: `public var AngleAxis(...);`
  - `LookRotation`
    - Signature: `public var LookRotation(...);`
  - `Slerp`
    - Signature: `public var Slerp(...);`
  - `SlerpUnclamped`
    - Signature: `public var SlerpUnclamped(...);`
  - `Lerp`
    - Signature: `public var Lerp(...);`
  - `LerpUnclamped`
    - Signature: `public var LerpUnclamped(...);`
  - `RotateTowards`
    - Signature: `public var RotateTowards(...);`
  - `Angle`
    - Signature: `public var Angle(...);`
  - `Dot`
    - Signature: `public var Dot(...);`
  - `FromToRotation`
    - Signature: `public var FromToRotation(...);`
  - `Inverse`
    - Signature: `public var Inverse(...);`
  - `Normalize`
    - Signature: `public var Normalize(...);`
  - `SetLookRotation`
    - Signature: `public var SetLookRotation(...);`
  - `SetFromToRotation`
    - Signature: `public var SetFromToRotation(...);`
  - `ToAngleAxis`
    - Signature: `public var ToAngleAxis(...);`
  - `eulerAngles`
    - Signature: `public var eulerAngles { get; set; }`
  - `normalized`
    - Signature: `public var normalized { get; set; }`
  - `*`
    - Signature: `public static operator *(...);`
  - `==`
    - Signature: `public static operator ==(...);`
  - `!=`
    - Signature: `public static operator !=(...);`

### Matrix4x4

- Namespace/type: `UnityEngine.Matrix4x4`
- Use when: 4x4 transformation and projection matrix.
- Key public API:
  - `GetColumn`
    - Signature: `public var GetColumn(...);`
  - `GetRow`
    - Signature: `public var GetRow(...);`
  - `SetColumn`
    - Signature: `public var SetColumn(...);`
  - `SetRow`
    - Signature: `public var SetRow(...);`
  - `MultiplyVector`
    - Signature: `public var MultiplyVector(...);`
  - `MultiplyPoint`
    - Signature: `public var MultiplyPoint(...);`
  - `MultiplyPoint3x4`
    - Signature: `public var MultiplyPoint3x4(...);`
  - `TransformPlane`
    - Signature: `public var TransformPlane(...);`
  - `inverse`
    - Signature: `public var inverse { get; set; }`
  - `transpose`
    - Signature: `public var transpose { get; set; }`
  - `identity`
    - Signature: `public var identity { get; set; }`
  - `zero`
    - Signature: `public var zero { get; set; }`
  - `TRS`
    - Signature: `public var TRS(...);`
  - `Rotate`
    - Signature: `public var Rotate(...);`
  - `Scale`
    - Signature: `public var Scale(...);`
  - `Translate`
    - Signature: `public var Translate(...);`
  - `Ortho`
    - Signature: `public var Ortho(...);`
  - `Perspective`
    - Signature: `public var Perspective(...);`
  - `Frustum`
    - Signature: `public var Frustum(...);`
  - `LookAt`
    - Signature: `public var LookAt(...);`
  - `valid`
    - Signature: `public var valid { get; set; }`
  - `isIdentity`
    - Signature: `public var isIdentity { get; set; }`
  - `lossyScale`
    - Signature: `public var lossyScale { get; set; }`
  - `rotation`
    - Signature: `public var rotation { get; set; }`
  - `position`
    - Signature: `public var position { get; set; }`
  - `determinant`
    - Signature: `public var determinant { get; set; }`
  - `SetTRS`
    - Signature: `public var SetTRS(...);`
  - `ValidTRS`
    - Signature: `public var ValidTRS(...);`

### Plane

- Namespace/type: `UnityEngine.Plane`
- Use when: Infinite plane represented by normal and distance.
- Key public API:
  - `normal`
    - Signature: `public var normal { get; set; }`
  - `distance`
    - Signature: `public var distance { get; set; }`
  - `Raycast`
    - Signature: `public var Raycast(...);`
  - `Flip`
    - Signature: `public var Flip(...);`
  - `GetDistanceToPoint`
    - Signature: `public var GetDistanceToPoint(...);`
  - `GetSide`
    - Signature: `public var GetSide(...);`
  - `SameSide`
    - Signature: `public var SameSide(...);`
  - `ClosestPointOnPlane`
    - Signature: `public var ClosestPointOnPlane(...);`
  - `Translate`
    - Signature: `public var Translate(...);`
  - `SetNormalAndPosition`
    - Signature: `public var SetNormalAndPosition(...);`
  - `Set3Points`
    - Signature: `public var Set3Points(...);`

### Ray

- Namespace/type: `UnityEngine.Ray`
- Use when: Origin and direction query primitive.
- Key public API:
  - `origin`
    - Signature: `public var origin { get; set; }`
  - `direction`
    - Signature: `public var direction { get; set; }`
  - `GetPoint`
    - Signature: `public var GetPoint(...);`
  - `ToString`
    - Signature: `public var ToString(...);`

### Bounds

- Namespace/type: `UnityEngine.Bounds`
- Use when: Axis-aligned bounding box.
- Key public API:
  - `center`
    - Signature: `public var center { get; set; }`
  - `extents`
    - Signature: `public var extents { get; set; }`
  - `size`
    - Signature: `public var size { get; set; }`
  - `min`
    - Signature: `public var min { get; set; }`
  - `max`
    - Signature: `public var max { get; set; }`
  - `Contains`
    - Signature: `public var Contains(...);`
  - `Encapsulate`
    - Signature: `public var Encapsulate(...);`
  - `Expand`
    - Signature: `public var Expand(...);`
  - `Intersects`
    - Signature: `public var Intersects(...);`
  - `SqrDistance`
    - Signature: `public var SqrDistance(...);`
  - `ClosestPoint`
    - Signature: `public var ClosestPoint(...);`
  - `SetMinMax`
    - Signature: `public var SetMinMax(...);`

### Color

- Namespace/type: `UnityEngine.Color`
- Use when: Floating point RGBA color.
- Key public API:
  - `r`
    - Signature: `public var r { get; set; }`
  - `g`
    - Signature: `public var g { get; set; }`
  - `b`
    - Signature: `public var b { get; set; }`
  - `a`
    - Signature: `public var a { get; set; }`
  - `red`
    - Signature: `public var red { get; set; }`
  - `green`
    - Signature: `public var green { get; set; }`
  - `blue`
    - Signature: `public var blue { get; set; }`
  - `white`
    - Signature: `public var white { get; set; }`
  - `black`
    - Signature: `public var black { get; set; }`
  - `yellow`
    - Signature: `public var yellow { get; set; }`
  - `cyan`
    - Signature: `public var cyan { get; set; }`
  - `magenta`
    - Signature: `public var magenta { get; set; }`
  - `grey`
    - Signature: `public var grey { get; set; }`
  - `gray`
    - Signature: `public var gray { get; set; }`
  - `clear`
    - Signature: `public var clear { get; set; }`
  - `Lerp`
    - Signature: `public var Lerp(...);`
  - `LerpUnclamped`
    - Signature: `public var LerpUnclamped(...);`
  - `HSVToRGB`
    - Signature: `public var HSVToRGB(...);`
  - `RGBToHSV`
    - Signature: `public var RGBToHSV(...);`
  - `+`
    - Signature: `public static operator +(...);`
  - `-`
    - Signature: `public static operator -(...);`
  - `*`
    - Signature: `public static operator *(...);`
  - `/`
    - Signature: `public static operator /(...);`
  - `==`
    - Signature: `public static operator ==(...);`
  - `!=`
    - Signature: `public static operator !=(...);`
  - `grayscale`
    - Signature: `public var grayscale { get; set; }`
  - `linear`
    - Signature: `public var linear { get; set; }`
  - `gamma`
    - Signature: `public var gamma { get; set; }`
  - `maxColorComponent`
    - Signature: `public var maxColorComponent { get; set; }`
  - `ToString`
    - Signature: `public var ToString(...);`

### Color32

- Namespace/type: `UnityEngine.Color32`
- Use when: Byte RGBA color with implicit Color conversion.
- Key public API:
  - `r`
    - Signature: `public var r { get; set; }`
  - `g`
    - Signature: `public var g { get; set; }`
  - `b`
    - Signature: `public var b { get; set; }`
  - `a`
    - Signature: `public var a { get; set; }`
  - `Lerp`
    - Signature: `public var Lerp(...);`
  - `LerpUnclamped`
    - Signature: `public var LerpUnclamped(...);`
  - `implicit Color`
    - Signature: `public static operator implicit Color(...);`
  - `implicit Color32`
    - Signature: `public static operator implicit Color32(...);`
  - `ToString`
    - Signature: `public var ToString(...);`

## UnityEngine.Physics

### Physics

- Namespace/type: `UnityEngine.Physics`
- Use when: Global 3D physics queries and settings.
- Key public API:
  - `Raycast`
    - Signature: `public static var Raycast(...);`
  - `RaycastAll`
    - Signature: `public static var RaycastAll(...);`
  - `RaycastNonAlloc`
    - Signature: `public static var RaycastNonAlloc(...);`
  - `SphereCast`
    - Signature: `public static var SphereCast(...);`
  - `SphereCastAll`
    - Signature: `public static var SphereCastAll(...);`
  - `CapsuleCast`
    - Signature: `public static var CapsuleCast(...);`
  - `CapsuleCastAll`
    - Signature: `public static var CapsuleCastAll(...);`
  - `BoxCast`
    - Signature: `public static var BoxCast(...);`
  - `BoxCastAll`
    - Signature: `public static var BoxCastAll(...);`
  - `Linecast`
    - Signature: `public static var Linecast(...);`
  - `LinecastAll`
    - Signature: `public static var LinecastAll(...);`
  - `OverlapSphere`
    - Signature: `public static var OverlapSphere(...);`
  - `OverlapSphereNonAlloc`
    - Signature: `public static var OverlapSphereNonAlloc(...);`
  - `OverlapBox`
    - Signature: `public static var OverlapBox(...);`
  - `OverlapBoxNonAlloc`
    - Signature: `public static var OverlapBoxNonAlloc(...);`
  - `OverlapCapsule`
    - Signature: `public static var OverlapCapsule(...);`
  - `OverlapCapsuleNonAlloc`
    - Signature: `public static var OverlapCapsuleNonAlloc(...);`
  - `CheckBox`
    - Signature: `public static var CheckBox(...);`
  - `CheckCapsule`
    - Signature: `public static var CheckCapsule(...);`
  - `CheckSphere`
    - Signature: `public static var CheckSphere(...);`
  - `ComputePenetration`
    - Signature: `public static var ComputePenetration(...);`
  - `IgnoreCollision`
    - Signature: `public static var IgnoreCollision(...);`
  - `IgnoreLayerCollision`
    - Signature: `public static var IgnoreLayerCollision(...);`
  - `GetIgnoreLayerCollision`
    - Signature: `public static var GetIgnoreLayerCollision(...);`
  - `Simulate`
    - Signature: `public static var Simulate(...);`
  - `SyncTransforms`
    - Signature: `public static var SyncTransforms(...);`
  - `gravity`
    - Signature: `public var gravity { get; set; }`
  - `defaultSolverIterations`
    - Signature: `public var defaultSolverIterations { get; set; }`
  - `defaultSolverVelocityIterations`
    - Signature: `public var defaultSolverVelocityIterations { get; set; }`
  - `queriesHitBackfaces`
    - Signature: `public var queriesHitBackfaces { get; set; }`
  - `queriesHitTriggers`
    - Signature: `public var queriesHitTriggers { get; set; }`
  - `bounceThreshold`
    - Signature: `public var bounceThreshold { get; set; }`
  - `sleepThreshold`
    - Signature: `public var sleepThreshold { get; set; }`
  - `contactOffsetMode`
    - Signature: `public var contactOffsetMode { get; set; }`
  - `autoSimulation`
    - Signature: `public var autoSimulation { get; set; }`
  - `autoSyncTransforms`
    - Signature: `public var autoSyncTransforms { get; set; }`
  - `reuseCollisionCallbacks`
    - Signature: `public var reuseCollisionCallbacks { get; set; }`
  - `interCollisionSettings`
    - Signature: `public var interCollisionSettings { get; set; }`
  - `interCollisionDistance`
    - Signature: `public var interCollisionDistance { get; set; }`
  - `interCollisionStiffness`
    - Signature: `public var interCollisionStiffness { get; set; }`
  - `interCollisionDrag`
    - Signature: `public var interCollisionDrag { get; set; }`
  - `interCollisionIterations`
    - Signature: `public var interCollisionIterations { get; set; }`
- Common pattern:
```csharp
if (Physics.Raycast(origin, direction, out var hit, 100f, layerMask)) {
    Debug.Log(hit.collider.name);
}
```

### Rigidbody

- Namespace/type: `UnityEngine.Rigidbody`
- Use when: 3D dynamic body controlled by physics.
- Key public API:
  - `mass`
    - Signature: `public var mass { get; set; }`
  - `drag`
    - Signature: `public var drag { get; set; }`
  - `angularDrag`
    - Signature: `public var angularDrag { get; set; }`
  - `useGravity`
    - Signature: `public var useGravity { get; set; }`
  - `isKinematic`
    - Signature: `public var isKinematic { get; set; }`
  - `velocity`
    - Signature: `public var velocity { get; set; }`
  - `linearVelocity`
    - Signature: `public var linearVelocity { get; set; }`
  - `angularVelocity`
    - Signature: `public var angularVelocity { get; set; }`
  - `AddForce`
    - Signature: `public var AddForce(...);`
  - `AddForceAtPosition`
    - Signature: `public var AddForceAtPosition(...);`
  - `AddRelativeForce`
    - Signature: `public var AddRelativeForce(...);`
  - `AddTorque`
    - Signature: `public var AddTorque(...);`
  - `AddRelativeTorque`
    - Signature: `public var AddRelativeTorque(...);`
  - `MovePosition`
    - Signature: `public var MovePosition(...);`
  - `MoveRotation`
    - Signature: `public var MoveRotation(...);`
  - `FreezeRotation`
    - Signature: `public var FreezeRotation(...);`
  - `constraints`
    - Signature: `public var constraints { get; set; }`
  - `centerOfMass`
    - Signature: `public var centerOfMass { get; set; }`
  - `worldCenterOfMass`
    - Signature: `public var worldCenterOfMass { get; set; }`
  - `inertiaTensor`
    - Signature: `public var inertiaTensor { get; set; }`
  - `inertiaTensorRotation`
    - Signature: `public var inertiaTensorRotation { get; set; }`
  - `maxAngularVelocity`
    - Signature: `public var maxAngularVelocity { get; set; }`
  - `maxDepenetrationVelocity`
    - Signature: `public var maxDepenetrationVelocity { get; set; }`
  - `sleepThreshold`
    - Signature: `public var sleepThreshold { get; set; }`
  - `sleepMode`
    - Signature: `public var sleepMode { get; set; }`
  - `IsSleeping`
    - Signature: `public var IsSleeping(...);`
  - `WakeUp`
    - Signature: `public var WakeUp(...);`
  - `Sleep`
    - Signature: `public var Sleep(...);`
  - `position`
    - Signature: `public var position { get; set; }`
  - `rotation`
    - Signature: `public var rotation { get; set; }`
  - `interpolation`
    - Signature: `public var interpolation { get; set; }`
  - `collisionDetectionMode`
    - Signature: `public var collisionDetectionMode { get; set; }`
  - `detectCollisions`
    - Signature: `public var detectCollisions { get; set; }`
  - `useConeFriction`
    - Signature: `public var useConeFriction { get; set; }`
  - `solverIterations`
    - Signature: `public var solverIterations { get; set; }`
  - `solverVelocityIterations`
    - Signature: `public var solverVelocityIterations { get; set; }`

### Collider

- Namespace/type: `UnityEngine.Collider`
- Use when: Base 3D collision shape.
- Key public API:
  - `enabled`
    - Signature: `public var enabled { get; set; }`
  - `isTrigger`
    - Signature: `public var isTrigger { get; set; }`
  - `contactOffset`
    - Signature: `public var contactOffset { get; set; }`
  - `bounds`
    - Signature: `public var bounds { get; set; }`
  - `attachedRigidbody`
    - Signature: `public var attachedRigidbody { get; set; }`
  - `material`
    - Signature: `public var material { get; set; }`
  - `sharedMaterial`
    - Signature: `public var sharedMaterial { get; set; }`
  - `ClosestPoint`
    - Signature: `public var ClosestPoint(...);`
  - `ClosestPointOnBounds`
    - Signature: `public var ClosestPointOnBounds(...);`
  - `Raycast`
    - Signature: `public var Raycast(...);`
  - `hasModifiableContacts`
    - Signature: `public var hasModifiableContacts { get; set; }`
  - `providesContacts`
    - Signature: `public var providesContacts { get; set; }`
  - `excludeLayers`
    - Signature: `public var excludeLayers { get; set; }`
  - `includeLayers`
    - Signature: `public var includeLayers { get; set; }`
  - `layerOverridePriority`
    - Signature: `public var layerOverridePriority { get; set; }`

### BoxCollider

- Namespace/type: `UnityEngine.BoxCollider`
- Use when: Box collision shape.
- Key public API:
  - `center`
    - Signature: `public var center { get; set; }`
  - `size`
    - Signature: `public var size { get; set; }`
  - `ClosestPoint`
    - Signature: `public var ClosestPoint(...);`
  - `Raycast`
    - Signature: `public var Raycast(...);`

### SphereCollider

- Namespace/type: `UnityEngine.SphereCollider`
- Use when: Sphere collision shape.
- Key public API:
  - `center`
    - Signature: `public var center { get; set; }`
  - `radius`
    - Signature: `public var radius { get; set; }`
  - `ClosestPoint`
    - Signature: `public var ClosestPoint(...);`
  - `Raycast`
    - Signature: `public var Raycast(...);`

### CapsuleCollider

- Namespace/type: `UnityEngine.CapsuleCollider`
- Use when: Capsule collision shape.
- Key public API:
  - `center`
    - Signature: `public var center { get; set; }`
  - `radius`
    - Signature: `public var radius { get; set; }`
  - `height`
    - Signature: `public var height { get; set; }`
  - `direction`
    - Signature: `public var direction { get; set; }`
  - `ClosestPoint`
    - Signature: `public var ClosestPoint(...);`
  - `Raycast`
    - Signature: `public var Raycast(...);`

### MeshCollider

- Namespace/type: `UnityEngine.MeshCollider`
- Use when: Mesh-backed collision shape.
- Key public API:
  - `sharedMesh`
    - Signature: `public var sharedMesh { get; set; }`
  - `convex`
    - Signature: `public var convex { get; set; }`
  - `cookingOptions`
    - Signature: `public var cookingOptions { get; set; }`
  - `ClosestPoint`
    - Signature: `public var ClosestPoint(...);`
  - `Raycast`
    - Signature: `public var Raycast(...);`

### TerrainCollider

- Namespace/type: `UnityEngine.TerrainCollider`
- Use when: Terrain collision shape.
- Key public API:
  - `terrainData`
    - Signature: `public var terrainData { get; set; }`
  - `enableTreeColliders`
    - Signature: `public var enableTreeColliders { get; set; }`
  - `ClosestPoint`
    - Signature: `public var ClosestPoint(...);`
  - `Raycast`
    - Signature: `public var Raycast(...);`

### WheelCollider

- Namespace/type: `UnityEngine.WheelCollider`
- Use when: Vehicle wheel simulation collider.
- Key public API:
  - `center`
    - Signature: `public var center { get; set; }`
  - `radius`
    - Signature: `public var radius { get; set; }`
  - `suspensionDistance`
    - Signature: `public var suspensionDistance { get; set; }`
  - `suspensionSpring`
    - Signature: `public var suspensionSpring { get; set; }`
  - `mass`
    - Signature: `public var mass { get; set; }`
  - `wheelDampingRate`
    - Signature: `public var wheelDampingRate { get; set; }`
  - `forwardFriction`
    - Signature: `public var forwardFriction { get; set; }`
  - `sidewaysFriction`
    - Signature: `public var sidewaysFriction { get; set; }`
  - `motorTorque`
    - Signature: `public var motorTorque { get; set; }`
  - `brakeTorque`
    - Signature: `public var brakeTorque { get; set; }`
  - `steerAngle`
    - Signature: `public var steerAngle { get; set; }`
  - `rpm`
    - Signature: `public var rpm { get; set; }`
  - `isGrounded`
    - Signature: `public var isGrounded { get; set; }`
  - `sprungMass`
    - Signature: `public var sprungMass { get; set; }`
  - `GetGroundHit`
    - Signature: `public var GetGroundHit(...);`
  - `ConfigureVehicleSubsteps`
    - Signature: `public var ConfigureVehicleSubsteps(...);`

### RaycastHit

- Namespace/type: `UnityEngine.RaycastHit`
- Use when: Result of a physics cast.
- Key public API:
  - `collider`
    - Signature: `public var collider { get; set; }`
  - `distance`
    - Signature: `public var distance { get; set; }`
  - `point`
    - Signature: `public var point { get; set; }`
  - `normal`
    - Signature: `public var normal { get; set; }`
  - `barycentricCoordinate`
    - Signature: `public var barycentricCoordinate { get; set; }`
  - `textureCoord`
    - Signature: `public var textureCoord { get; set; }`
  - `textureCoord2`
    - Signature: `public var textureCoord2 { get; set; }`
  - `transform`
    - Signature: `public var transform { get; set; }`
  - `rigidbody`
    - Signature: `public var rigidbody { get; set; }`
  - `triangleIndex`
    - Signature: `public var triangleIndex { get; set; }`
  - `lightmapCoord`
    - Signature: `public var lightmapCoord { get; set; }`
  - `articulationBody`
    - Signature: `public var articulationBody { get; set; }`

### Collision

- Namespace/type: `UnityEngine.Collision`
- Use when: Collision callback payload.
- Key public API:
  - `gameObject`
    - Signature: `public var gameObject { get; set; }`
  - `transform`
    - Signature: `public var transform { get; set; }`
  - `rigidbody`
    - Signature: `public var rigidbody { get; set; }`
  - `collider`
    - Signature: `public var collider { get; set; }`
  - `contactCount`
    - Signature: `public var contactCount { get; set; }`
  - `GetContact`
    - Signature: `public var GetContact(...);`
  - `GetContacts`
    - Signature: `public var GetContacts(...);`
  - `contacts`
    - Signature: `public var contacts { get; set; }`
  - `relativeVelocity`
    - Signature: `public var relativeVelocity { get; set; }`
  - `impulse`
    - Signature: `public var impulse { get; set; }`
  - `other`
    - Signature: `public var other { get; set; }`
  - `articulationBody`
    - Signature: `public var articulationBody { get; set; }`

### ContactPoint

- Namespace/type: `UnityEngine.ContactPoint`
- Use when: Contact point data.
- Key public API:
  - `point`
    - Signature: `public var point { get; set; }`
  - `normal`
    - Signature: `public var normal { get; set; }`
  - `thisCollider`
    - Signature: `public var thisCollider { get; set; }`
  - `otherCollider`
    - Signature: `public var otherCollider { get; set; }`
  - `separation`
    - Signature: `public var separation { get; set; }`
  - `impulse`
    - Signature: `public var impulse { get; set; }`

### Joint

- Namespace/type: `UnityEngine.Joint`
- Use when: Base physics joint.
- Key public API:
  - `connectedBody`
    - Signature: `public var connectedBody { get; set; }`
  - `axis`
    - Signature: `public var axis { get; set; }`
  - `anchor`
    - Signature: `public var anchor { get; set; }`
  - `connectedAnchor`
    - Signature: `public var connectedAnchor { get; set; }`
  - `autoConfigureConnectedAnchor`
    - Signature: `public var autoConfigureConnectedAnchor { get; set; }`
  - `breakForce`
    - Signature: `public var breakForce { get; set; }`
  - `breakTorque`
    - Signature: `public var breakTorque { get; set; }`
  - `enableCollision`
    - Signature: `public var enableCollision { get; set; }`
  - `enablePreprocessing`
    - Signature: `public var enablePreprocessing { get; set; }`
  - `massScale`
    - Signature: `public var massScale { get; set; }`
  - `connectedMassScale`
    - Signature: `public var connectedMassScale { get; set; }`
  - `currentForce`
    - Signature: `public var currentForce { get; set; }`
  - `currentTorque`
    - Signature: `public var currentTorque { get; set; }`

### FixedJoint

- Namespace/type: `UnityEngine.FixedJoint`
- Use when: Rigid fixed connection.
- Key public API:
  - `connectedBody`
    - Signature: `public var connectedBody { get; set; }`
  - `breakForce`
    - Signature: `public var breakForce { get; set; }`
  - `breakTorque`
    - Signature: `public var breakTorque { get; set; }`

### SpringJoint

- Namespace/type: `UnityEngine.SpringJoint`
- Use when: Distance spring connection.
- Key public API:
  - `spring`
    - Signature: `public var spring { get; set; }`
  - `damper`
    - Signature: `public var damper { get; set; }`
  - `minDistance`
    - Signature: `public var minDistance { get; set; }`
  - `maxDistance`
    - Signature: `public var maxDistance { get; set; }`
  - `tolerance`
    - Signature: `public var tolerance { get; set; }`

### HingeJoint

- Namespace/type: `UnityEngine.HingeJoint`
- Use when: Single-axis rotational joint.
- Key public API:
  - `motor`
    - Signature: `public var motor { get; set; }`
  - `spring`
    - Signature: `public var spring { get; set; }`
  - `limits`
    - Signature: `public var limits { get; set; }`
  - `useMotor`
    - Signature: `public var useMotor { get; set; }`
  - `useSpring`
    - Signature: `public var useSpring { get; set; }`
  - `useLimits`
    - Signature: `public var useLimits { get; set; }`
  - `velocity`
    - Signature: `public var velocity { get; set; }`
  - `angle`
    - Signature: `public var angle { get; set; }`

### CharacterJoint

- Namespace/type: `UnityEngine.CharacterJoint`
- Use when: Ragdoll-style constrained joint.
- Key public API:
  - `swingAxis`
    - Signature: `public var swingAxis { get; set; }`
  - `twistLimitSpring`
    - Signature: `public var twistLimitSpring { get; set; }`
  - `swingLimitSpring`
    - Signature: `public var swingLimitSpring { get; set; }`
  - `lowTwistLimit`
    - Signature: `public var lowTwistLimit { get; set; }`
  - `highTwistLimit`
    - Signature: `public var highTwistLimit { get; set; }`
  - `swing1Limit`
    - Signature: `public var swing1Limit { get; set; }`
  - `swing2Limit`
    - Signature: `public var swing2Limit { get; set; }`
  - `enableProjection`
    - Signature: `public var enableProjection { get; set; }`
  - `projectionDistance`
    - Signature: `public var projectionDistance { get; set; }`
  - `projectionAngle`
    - Signature: `public var projectionAngle { get; set; }`

### ConfigurableJoint

- Namespace/type: `UnityEngine.ConfigurableJoint`
- Use when: General configurable 6DOF joint.
- Key public API:
  - `secondaryAxis`
    - Signature: `public var secondaryAxis { get; set; }`
  - `xMotion`
    - Signature: `public var xMotion { get; set; }`
  - `yMotion`
    - Signature: `public var yMotion { get; set; }`
  - `zMotion`
    - Signature: `public var zMotion { get; set; }`
  - `angularXMotion`
    - Signature: `public var angularXMotion { get; set; }`
  - `angularYMotion`
    - Signature: `public var angularYMotion { get; set; }`
  - `angularZMotion`
    - Signature: `public var angularZMotion { get; set; }`
  - `linearLimit`
    - Signature: `public var linearLimit { get; set; }`
  - `lowAngularXLimit`
    - Signature: `public var lowAngularXLimit { get; set; }`
  - `highAngularXLimit`
    - Signature: `public var highAngularXLimit { get; set; }`
  - `angularYLimit`
    - Signature: `public var angularYLimit { get; set; }`
  - `angularZLimit`
    - Signature: `public var angularZLimit { get; set; }`
  - `targetPosition`
    - Signature: `public var targetPosition { get; set; }`
  - `targetVelocity`
    - Signature: `public var targetVelocity { get; set; }`
  - `targetRotation`
    - Signature: `public var targetRotation { get; set; }`
  - `targetAngularVelocity`
    - Signature: `public var targetAngularVelocity { get; set; }`
  - `xDrive`
    - Signature: `public var xDrive { get; set; }`
  - `yDrive`
    - Signature: `public var yDrive { get; set; }`
  - `zDrive`
    - Signature: `public var zDrive { get; set; }`
  - `angularXDrive`
    - Signature: `public var angularXDrive { get; set; }`
  - `angularYZDrive`
    - Signature: `public var angularYZDrive { get; set; }`
  - `slerpDrive`
    - Signature: `public var slerpDrive { get; set; }`
  - `rotationDriveMode`
    - Signature: `public var rotationDriveMode { get; set; }`
  - `projectionMode`
    - Signature: `public var projectionMode { get; set; }`
  - `projectionDistance`
    - Signature: `public var projectionDistance { get; set; }`
  - `projectionAngle`
    - Signature: `public var projectionAngle { get; set; }`
  - `configuredInWorldSpace`
    - Signature: `public var configuredInWorldSpace { get; set; }`
  - `swapBodies`
    - Signature: `public var swapBodies { get; set; }`

## UnityEngine.Rendering / Graphics

### Mesh

- Namespace/type: `UnityEngine.Mesh`
- Use when: CPU/GPU mesh geometry container.
- Key public API:
  - `vertices`
    - Signature: `public var vertices { get; set; }`
  - `normals`
    - Signature: `public var normals { get; set; }`
  - `tangents`
    - Signature: `public var tangents { get; set; }`
  - `uv`
    - Signature: `public var uv { get; set; }`
  - `uv2`
    - Signature: `public var uv2 { get; set; }`
  - `uv3`
    - Signature: `public var uv3 { get; set; }`
  - `uv4`
    - Signature: `public var uv4 { get; set; }`
  - `colors`
    - Signature: `public var colors { get; set; }`
  - `colors32`
    - Signature: `public var colors32 { get; set; }`
  - `triangles`
    - Signature: `public var triangles { get; set; }`
  - `subMeshCount`
    - Signature: `public var subMeshCount { get; set; }`
  - `vertexCount`
    - Signature: `public var vertexCount { get; set; }`
  - `bounds`
    - Signature: `public var bounds { get; set; }`
  - `Clear`
    - Signature: `public var Clear(...);`
  - `RecalculateNormals`
    - Signature: `public var RecalculateNormals(...);`
  - `RecalculateTangents`
    - Signature: `public var RecalculateTangents(...);`
  - `RecalculateBounds`
    - Signature: `public var RecalculateBounds(...);`
  - `Optimize`
    - Signature: `public var Optimize(...);`
  - `OptimizeIndexBuffers`
    - Signature: `public var OptimizeIndexBuffers(...);`
  - `OptimizeReorderVertexBuffer`
    - Signature: `public var OptimizeReorderVertexBuffer(...);`
  - `CombineMeshes`
    - Signature: `public var CombineMeshes(...);`
  - `GetTriangles`
    - Signature: `public var GetTriangles(...);`
  - `SetTriangles`
    - Signature: `public var SetTriangles(...);`
  - `GetVertices`
    - Signature: `public var GetVertices(...);`
  - `SetVertices`
    - Signature: `public var SetVertices(...);`
  - `GetNormals`
    - Signature: `public var GetNormals(...);`
  - `SetNormals`
    - Signature: `public var SetNormals(...);`
  - `GetTangents`
    - Signature: `public var GetTangents(...);`
  - `SetTangents`
    - Signature: `public var SetTangents(...);`
  - `GetUVs`
    - Signature: `public var GetUVs(...);`
  - `SetUVs`
    - Signature: `public var SetUVs(...);`
  - `GetColors`
    - Signature: `public var GetColors(...);`
  - `SetColors`
    - Signature: `public var SetColors(...);`
  - `GetIndices`
    - Signature: `public var GetIndices(...);`
  - `SetIndices`
    - Signature: `public var SetIndices(...);`
  - `GetBoneWeights`
    - Signature: `public var GetBoneWeights(...);`
  - `SetBoneWeights`
    - Signature: `public var SetBoneWeights(...);`
  - `bindposes`
    - Signature: `public var bindposes { get; set; }`
  - `GetTopology`
    - Signature: `public var GetTopology(...);`
  - `SetSubMesh`
    - Signature: `public var SetSubMesh(...);`
  - `GetSubMesh`
    - Signature: `public var GetSubMesh(...);`
  - `UploadMeshData`
    - Signature: `public var UploadMeshData(...);`
  - `MarkDynamic`
    - Signature: `public var MarkDynamic(...);`
  - `isReadable`
    - Signature: `public var isReadable { get; set; }`
  - `indexFormat`
    - Signature: `public var indexFormat { get; set; }`

### Material

- Namespace/type: `UnityEngine.Material`
- Use when: Shader instance properties and render state.
- Key public API:
  - `shader`
    - Signature: `public var shader { get; set; }`
  - `color`
    - Signature: `public var color { get; set; }`
  - `mainTexture`
    - Signature: `public var mainTexture { get; set; }`
  - `mainTextureOffset`
    - Signature: `public var mainTextureOffset { get; set; }`
  - `mainTextureScale`
    - Signature: `public var mainTextureScale { get; set; }`
  - `renderQueue`
    - Signature: `public var renderQueue { get; set; }`
  - `SetFloat`
    - Signature: `public var SetFloat(...);`
  - `GetFloat`
    - Signature: `public var GetFloat(...);`
  - `SetInt`
    - Signature: `public var SetInt(...);`
  - `GetInt`
    - Signature: `public var GetInt(...);`
  - `SetColor`
    - Signature: `public var SetColor(...);`
  - `GetColor`
    - Signature: `public var GetColor(...);`
  - `SetVector`
    - Signature: `public var SetVector(...);`
  - `GetVector`
    - Signature: `public var GetVector(...);`
  - `SetMatrix`
    - Signature: `public var SetMatrix(...);`
  - `GetMatrix`
    - Signature: `public var GetMatrix(...);`
  - `SetTexture`
    - Signature: `public var SetTexture(...);`
  - `GetTexture`
    - Signature: `public var GetTexture(...);`
  - `SetTextureOffset`
    - Signature: `public var SetTextureOffset(...);`
  - `GetTextureOffset`
    - Signature: `public var GetTextureOffset(...);`
  - `SetTextureScale`
    - Signature: `public var SetTextureScale(...);`
  - `GetTextureScale`
    - Signature: `public var GetTextureScale(...);`
  - `SetFloatArray`
    - Signature: `public var SetFloatArray(...);`
  - `SetVectorArray`
    - Signature: `public var SetVectorArray(...);`
  - `SetMatrixArray`
    - Signature: `public var SetMatrixArray(...);`
  - `EnableKeyword`
    - Signature: `public var EnableKeyword(...);`
  - `DisableKeyword`
    - Signature: `public var DisableKeyword(...);`
  - `IsKeywordEnabled`
    - Signature: `public var IsKeywordEnabled(...);`
  - `CopyPropertiesFromMaterial`
    - Signature: `public var CopyPropertiesFromMaterial(...);`
  - `SetPass`
    - Signature: `public var SetPass(...);`
  - `HasProperty`
    - Signature: `public var HasProperty(...);`
  - `Lerp`
    - Signature: `public var Lerp(...);`
  - `globalIlluminationFlags`
    - Signature: `public var globalIlluminationFlags { get; set; }`
  - `enableInstancing`
    - Signature: `public var enableInstancing { get; set; }`
  - `doubleSidedGI`
    - Signature: `public var doubleSidedGI { get; set; }`
  - `passCount`
    - Signature: `public var passCount { get; set; }`
  - `FindPass`
    - Signature: `public var FindPass(...);`
  - `GetPassName`
    - Signature: `public var GetPassName(...);`

### Shader

- Namespace/type: `UnityEngine.Shader`
- Use when: Shader asset lookup and global shader state.
- Key public API:
  - `Find`
    - Signature: `public static var Find(...);`
  - `PropertyToID`
    - Signature: `public static var PropertyToID(...);`
  - `SetGlobalFloat`
    - Signature: `public static var SetGlobalFloat(...);`
  - `GetGlobalFloat`
    - Signature: `public static var GetGlobalFloat(...);`
  - `SetGlobalColor`
    - Signature: `public static var SetGlobalColor(...);`
  - `GetGlobalColor`
    - Signature: `public static var GetGlobalColor(...);`
  - `SetGlobalVector`
    - Signature: `public static var SetGlobalVector(...);`
  - `GetGlobalVector`
    - Signature: `public static var GetGlobalVector(...);`
  - `SetGlobalMatrix`
    - Signature: `public static var SetGlobalMatrix(...);`
  - `GetGlobalMatrix`
    - Signature: `public static var GetGlobalMatrix(...);`
  - `SetGlobalTexture`
    - Signature: `public static var SetGlobalTexture(...);`
  - `GetGlobalTexture`
    - Signature: `public static var GetGlobalTexture(...);`
  - `SetGlobalFloatArray`
    - Signature: `public static var SetGlobalFloatArray(...);`
  - `SetGlobalVectorArray`
    - Signature: `public static var SetGlobalVectorArray(...);`
  - `SetGlobalMatrixArray`
    - Signature: `public static var SetGlobalMatrixArray(...);`
  - `renderQueue`
    - Signature: `public var renderQueue { get; set; }`
  - `maximumLOD`
    - Signature: `public var maximumLOD { get; set; }`
  - `globalIlluminationFlags`
    - Signature: `public var globalIlluminationFlags { get; set; }`
  - `EnableKeyword`
    - Signature: `public static var EnableKeyword(...);`
  - `DisableKeyword`
    - Signature: `public static var DisableKeyword(...);`
  - `IsKeywordEnabled`
    - Signature: `public static var IsKeywordEnabled(...);`
  - `keywordSpace`
    - Signature: `public var keywordSpace { get; set; }`
  - `isEnabled`
    - Signature: `public var isEnabled { get; set; }`
  - `isSupported`
    - Signature: `public var isSupported { get; set; }`
  - `passCount`
    - Signature: `public var passCount { get; set; }`
  - `GetPassName`
    - Signature: `public static var GetPassName(...);`
  - `GetPassType`
    - Signature: `public static var GetPassType(...);`
  - `FindPass`
    - Signature: `public static var FindPass(...);`
  - `WarmupAllShaders`
    - Signature: `public static var WarmupAllShaders(...);`

### Texture

- Namespace/type: `UnityEngine.Texture`
- Use when: Base texture resource.
- Key public API:
  - `width`
    - Signature: `public var width { get; set; }`
  - `height`
    - Signature: `public var height { get; set; }`
  - `dimension`
    - Signature: `public var dimension { get; set; }`
  - `wrapMode`
    - Signature: `public var wrapMode { get; set; }`
  - `wrapModeU`
    - Signature: `public var wrapModeU { get; set; }`
  - `wrapModeV`
    - Signature: `public var wrapModeV { get; set; }`
  - `wrapModeW`
    - Signature: `public var wrapModeW { get; set; }`
  - `filterMode`
    - Signature: `public var filterMode { get; set; }`
  - `anisoLevel`
    - Signature: `public var anisoLevel { get; set; }`
  - `mipMapBias`
    - Signature: `public var mipMapBias { get; set; }`
  - `texelSize`
    - Signature: `public var texelSize { get; set; }`
  - `masterTextureLimit`
    - Signature: `public var masterTextureLimit { get; set; }`
  - `SetGlobalAnisotropicFilteringLimits`
    - Signature: `public var SetGlobalAnisotropicFilteringLimits(...);`
  - `graphicsFormat`
    - Signature: `public var graphicsFormat { get; set; }`
  - `updateCount`
    - Signature: `public var updateCount { get; set; }`

### Texture2D

- Namespace/type: `UnityEngine.Texture2D`
- Use when: 2D texture CPU/GPU pixel API.
- Key public API:
  - `Resize`
    - Signature: `public var Resize(...);`
  - `ReadPixels`
    - Signature: `public var ReadPixels(...);`
  - `EncodeToPNG`
    - Signature: `public var EncodeToPNG(...);`
  - `EncodeToJPG`
    - Signature: `public var EncodeToJPG(...);`
  - `EncodeToTGA`
    - Signature: `public var EncodeToTGA(...);`
  - `LoadImage`
    - Signature: `public var LoadImage(...);`
  - `LoadRawTextureData`
    - Signature: `public var LoadRawTextureData(...);`
  - `GetRawTextureData`
    - Signature: `public var GetRawTextureData(...);`
  - `GetPixel`
    - Signature: `public var GetPixel(...);`
  - `GetPixelBilinear`
    - Signature: `public var GetPixelBilinear(...);`
  - `GetPixels`
    - Signature: `public var GetPixels(...);`
  - `GetPixels32`
    - Signature: `public var GetPixels32(...);`
  - `SetPixel`
    - Signature: `public var SetPixel(...);`
  - `SetPixels`
    - Signature: `public var SetPixels(...);`
  - `SetPixels32`
    - Signature: `public var SetPixels32(...);`
  - `Apply`
    - Signature: `public var Apply(...);`
  - `Compress`
    - Signature: `public var Compress(...);`
  - `format`
    - Signature: `public var format { get; set; }`
  - `mipmapCount`
    - Signature: `public var mipmapCount { get; set; }`
  - `isReadable`
    - Signature: `public var isReadable { get; set; }`
  - `alphaIsTransparency`
    - Signature: `public var alphaIsTransparency { get; set; }`
  - `CreateExternalTexture`
    - Signature: `public var CreateExternalTexture(...);`
  - `UpdateExternalTexture`
    - Signature: `public var UpdateExternalTexture(...);`
  - `PackTextures`
    - Signature: `public var PackTextures(...);`

### RenderTexture

- Namespace/type: `UnityEngine.RenderTexture`
- Use when: GPU render target texture.
- Key public API:
  - `format`
    - Signature: `public var format { get; set; }`
  - `width`
    - Signature: `public var width { get; set; }`
  - `height`
    - Signature: `public var height { get; set; }`
  - `depth`
    - Signature: `public var depth { get; set; }`
  - `depthBuffer`
    - Signature: `public var depthBuffer { get; set; }`
  - `antiAliasing`
    - Signature: `public var antiAliasing { get; set; }`
  - `useMipMap`
    - Signature: `public var useMipMap { get; set; }`
  - `autoGenerateMips`
    - Signature: `public var autoGenerateMips { get; set; }`
  - `enableRandomWrite`
    - Signature: `public var enableRandomWrite { get; set; }`
  - `sRGB`
    - Signature: `public var sRGB { get; set; }`
  - `dimension`
    - Signature: `public var dimension { get; set; }`
  - `volumeDepth`
    - Signature: `public var volumeDepth { get; set; }`
  - `Create`
    - Signature: `public var Create(...);`
  - `Release`
    - Signature: `public var Release(...);`
  - `DiscardContents`
    - Signature: `public var DiscardContents(...);`
  - `IsCreated`
    - Signature: `public var IsCreated { get; set; }`
  - `current`
    - Signature: `public var current { get; set; }`
  - `active`
    - Signature: `public var active { get; set; }`
  - `GenerateMips`
    - Signature: `public var GenerateMips(...);`
  - `useDynamicScale`
    - Signature: `public var useDynamicScale { get; set; }`
  - `stencilFormat`
    - Signature: `public var stencilFormat { get; set; }`
  - `graphicsFormat`
    - Signature: `public var graphicsFormat { get; set; }`
  - `memorylessMode`
    - Signature: `public var memorylessMode { get; set; }`
  - `descriptor`
    - Signature: `public var descriptor { get; set; }`
  - `GetTemporary`
    - Signature: `public var GetTemporary(...);`
  - `ReleaseTemporary`
    - Signature: `public var ReleaseTemporary(...);`

### Camera

- Namespace/type: `UnityEngine.Camera`
- Use when: View rendering, projection, culling, and coordinate conversion.
- Key public API:
  - `fieldOfView`
    - Signature: `public var fieldOfView { get; set; }`
  - `nearClipPlane`
    - Signature: `public var nearClipPlane { get; set; }`
  - `farClipPlane`
    - Signature: `public var farClipPlane { get; set; }`
  - `aspect`
    - Signature: `public var aspect { get; set; }`
  - `orthographic`
    - Signature: `public var orthographic { get; set; }`
  - `orthographicSize`
    - Signature: `public var orthographicSize { get; set; }`
  - `clearFlags`
    - Signature: `public var clearFlags { get; set; }`
  - `backgroundColor`
    - Signature: `public var backgroundColor { get; set; }`
  - `cullingMask`
    - Signature: `public var cullingMask { get; set; }`
  - `eventMask`
    - Signature: `public var eventMask { get; set; }`
  - `cameraType`
    - Signature: `public var cameraType { get; set; }`
  - `targetTexture`
    - Signature: `public var targetTexture { get; set; }`
  - `depth`
    - Signature: `public var depth { get; set; }`
  - `depthTextureMode`
    - Signature: `public var depthTextureMode { get; set; }`
  - `layerCullDistances`
    - Signature: `public var layerCullDistances { get; set; }`
  - `layerCullSpherical`
    - Signature: `public var layerCullSpherical { get; set; }`
  - `useOcclusionCulling`
    - Signature: `public var useOcclusionCulling { get; set; }`
  - `scene`
    - Signature: `public var scene { get; set; }`
  - `ScreenPointToRay`
    - Signature: `public var ScreenPointToRay(...);`
  - `ScreenToWorldPoint`
    - Signature: `public var ScreenToWorldPoint(...);`
  - `WorldToScreenPoint`
    - Signature: `public var WorldToScreenPoint(...);`
  - `WorldToViewportPoint`
    - Signature: `public var WorldToViewportPoint(...);`
  - `ViewportToWorldPoint`
    - Signature: `public var ViewportToWorldPoint(...);`
  - `ViewportPointToRay`
    - Signature: `public var ViewportPointToRay(...);`
  - `ScreenToViewportPoint`
    - Signature: `public var ScreenToViewportPoint(...);`
  - `CalculateObliqueMatrix`
    - Signature: `public var CalculateObliqueMatrix(...);`
  - `CalculateFrustumCorners`
    - Signature: `public var CalculateFrustumCorners(...);`
  - `Render`
    - Signature: `public var Render(...);`
  - `RenderToCubemap`
    - Signature: `public var RenderToCubemap(...);`
  - `SetTargetBuffers`
    - Signature: `public var SetTargetBuffers(...);`
  - `SetReplacementShader`
    - Signature: `public var SetReplacementShader(...);`
  - `ResetReplacementShader`
    - Signature: `public var ResetReplacementShader(...);`
  - `ResetCullingMatrix`
    - Signature: `public var ResetCullingMatrix(...);`
  - `CopyFrom`
    - Signature: `public var CopyFrom(...);`
  - `GetStereoViewMatrix`
    - Signature: `public var GetStereoViewMatrix(...);`
  - `GetStereoProjectionMatrix`
    - Signature: `public var GetStereoProjectionMatrix(...);`
  - `GetStereoNonJitteredProjectionMatrix`
    - Signature: `public var GetStereoNonJitteredProjectionMatrix(...);`
  - `GetStereoViewPort`
    - Signature: `public var GetStereoViewPort(...);`
  - `CalculateProjectionMatrixFromPhysicalProperties`
    - Signature: `public var CalculateProjectionMatrixFromPhysicalProperties(...);`
  - `focalLength`
    - Signature: `public var focalLength { get; set; }`
  - `sensorSize`
    - Signature: `public var sensorSize { get; set; }`
  - `lensShift`
    - Signature: `public var lensShift { get; set; }`
  - `gateFit`
    - Signature: `public var gateFit { get; set; }`
  - `usePhysicalProperties`
    - Signature: `public var usePhysicalProperties { get; set; }`
  - `transmittanceQueryEnabled`
    - Signature: `public var transmittanceQueryEnabled { get; set; }`
  - `onPreCull`
    - Signature: `public event Action onPreCull;`
  - `onPreRender`
    - Signature: `public event Action onPreRender;`
  - `onPostRender`
    - Signature: `public event Action onPostRender;`

### Light

- Namespace/type: `UnityEngine.Light`
- Use when: Scene light source.
- Key public API:
  - `type`
    - Signature: `public var type { get; set; }`
  - `color`
    - Signature: `public var color { get; set; }`
  - `intensity`
    - Signature: `public var intensity { get; set; }`
  - `bounceIntensity`
    - Signature: `public var bounceIntensity { get; set; }`
  - `colorTemperature`
    - Signature: `public var colorTemperature { get; set; }`
  - `useColorTemperature`
    - Signature: `public var useColorTemperature { get; set; }`
  - `range`
    - Signature: `public var range { get; set; }`
  - `spotAngle`
    - Signature: `public var spotAngle { get; set; }`
  - `innerSpotAngle`
    - Signature: `public var innerSpotAngle { get; set; }`
  - `shadowCustomResolution`
    - Signature: `public var shadowCustomResolution { get; set; }`
  - `shadowBias`
    - Signature: `public var shadowBias { get; set; }`
  - `shadowNormalBias`
    - Signature: `public var shadowNormalBias { get; set; }`
  - `shadowNearPlane`
    - Signature: `public var shadowNearPlane { get; set; }`
  - `shadows`
    - Signature: `public var shadows { get; set; }`
  - `shadowStrength`
    - Signature: `public var shadowStrength { get; set; }`
  - `renderMode`
    - Signature: `public var renderMode { get; set; }`
  - `cullingMask`
    - Signature: `public var cullingMask { get; set; }`
  - `commandBufferCount`
    - Signature: `public var commandBufferCount { get; set; }`
  - `AddCommandBuffer`
    - Signature: `public var AddCommandBuffer(...);`
  - `RemoveCommandBuffer`
    - Signature: `public var RemoveCommandBuffer(...);`
  - `RemoveAllCommandBuffers`
    - Signature: `public var RemoveAllCommandBuffers(...);`
  - `GetCommandBuffers`
    - Signature: `public var GetCommandBuffers(...);`
  - `cookie`
    - Signature: `public var cookie { get; set; }`
  - `flare`
    - Signature: `public var flare { get; set; }`
  - `renderingLayerMask`
    - Signature: `public var renderingLayerMask { get; set; }`
  - `lightShadowCasterMode`
    - Signature: `public var lightShadowCasterMode { get; set; }`

### Graphics

- Namespace/type: `UnityEngine.Graphics`
- Use when: Immediate draw, blit, texture copy, and command buffer execution.
- Key public API:
  - `DrawMesh`
    - Signature: `public static var DrawMesh(...);`
  - `DrawMeshInstanced`
    - Signature: `public static var DrawMeshInstanced(...);`
  - `DrawMeshInstancedIndirect`
    - Signature: `public static var DrawMeshInstancedIndirect(...);`
  - `DrawMeshInstancedProcedural`
    - Signature: `public static var DrawMeshInstancedProcedural(...);`
  - `DrawProcedural`
    - Signature: `public static var DrawProcedural(...);`
  - `DrawProceduralIndirect`
    - Signature: `public static var DrawProceduralIndirect(...);`
  - `Blit`
    - Signature: `public static var Blit(...);`
  - `CopyTexture`
    - Signature: `public static var CopyTexture(...);`
  - `ConvertTexture`
    - Signature: `public static var ConvertTexture(...);`
  - `SetRenderTarget`
    - Signature: `public static var SetRenderTarget(...);`
  - `SetRandomWriteTarget`
    - Signature: `public static var SetRandomWriteTarget(...);`
  - `ClearRandomWriteTargets`
    - Signature: `public static var ClearRandomWriteTargets(...);`
  - `ExecuteCommandBuffer`
    - Signature: `public static var ExecuteCommandBuffer(...);`
  - `ExecuteCommandBufferAsync`
    - Signature: `public static var ExecuteCommandBufferAsync(...);`
  - `Dispose`
    - Signature: `public static var Dispose(...);`
  - `Fence`
    - Signature: `public static var Fence(...);`
  - `WaitOnAsyncGraphicsFence`
    - Signature: `public static var WaitOnAsyncGraphicsFence(...);`

### CommandBuffer

- Namespace/type: `UnityEngine.Rendering.CommandBuffer`
- Use when: Recorded rendering commands.
- Key public API:
  - `DrawMesh`
    - Signature: `public var DrawMesh(...);`
  - `DrawRenderer`
    - Signature: `public var DrawRenderer(...);`
  - `DrawProcedural`
    - Signature: `public var DrawProcedural(...);`
  - `DrawProceduralIndirect`
    - Signature: `public var DrawProceduralIndirect(...);`
  - `DrawMeshInstanced`
    - Signature: `public var DrawMeshInstanced(...);`
  - `DrawMeshInstancedIndirect`
    - Signature: `public var DrawMeshInstancedIndirect(...);`
  - `DrawMeshInstancedProcedural`
    - Signature: `public var DrawMeshInstancedProcedural(...);`
  - `Blit`
    - Signature: `public var Blit(...);`
  - `CopyTexture`
    - Signature: `public var CopyTexture(...);`
  - `ConvertTexture`
    - Signature: `public var ConvertTexture(...);`
  - `GetTemporaryRT`
    - Signature: `public var GetTemporaryRT(...);`
  - `GetTemporaryRTArray`
    - Signature: `public var GetTemporaryRTArray(...);`
  - `GetTemporaryRTDesc`
    - Signature: `public var GetTemporaryRTDesc(...);`
  - `ReleaseTemporaryRT`
    - Signature: `public var ReleaseTemporaryRT(...);`
  - `SetRenderTarget`
    - Signature: `public var SetRenderTarget(...);`
  - `SetGlobalColor`
    - Signature: `public var SetGlobalColor(...);`
  - `SetGlobalFloat`
    - Signature: `public var SetGlobalFloat(...);`
  - `SetGlobalInt`
    - Signature: `public var SetGlobalInt(...);`
  - `SetGlobalVector`
    - Signature: `public var SetGlobalVector(...);`
  - `SetGlobalMatrix`
    - Signature: `public var SetGlobalMatrix(...);`
  - `SetGlobalTexture`
    - Signature: `public var SetGlobalTexture(...);`
  - `EnableShaderKeyword`
    - Signature: `public var EnableShaderKeyword(...);`
  - `DisableShaderKeyword`
    - Signature: `public var DisableShaderKeyword(...);`
  - `SetViewProjectionMatrices`
    - Signature: `public var SetViewProjectionMatrices(...);`
  - `SetViewport`
    - Signature: `public var SetViewport(...);`
  - `SetScissorRect`
    - Signature: `public var SetScissorRect(...);`
  - `DisableScissorRect`
    - Signature: `public var DisableScissorRect(...);`
  - `ClearRenderTarget`
    - Signature: `public var ClearRenderTarget(...);`
  - `ClearRandomWriteTargets`
    - Signature: `public var ClearRandomWriteTargets(...);`
  - `SetRandomWriteTarget`
    - Signature: `public var SetRandomWriteTarget(...);`
  - `BeginSample`
    - Signature: `public var BeginSample(...);`
  - `EndSample`
    - Signature: `public var EndSample(...);`
  - `IssuePluginEvent`
    - Signature: `public var IssuePluginEvent(...);`
  - `IssuePluginEventAndData`
    - Signature: `public var IssuePluginEventAndData(...);`
  - `IssuePluginCustomBlit`
    - Signature: `public var IssuePluginCustomBlit(...);`
  - `WaitOnGPUFence`
    - Signature: `public var WaitOnGPUFence(...);`
  - `CreateGPUFence`
    - Signature: `public var CreateGPUFence(...);`
  - `Clear`
    - Signature: `public var Clear(...);`
  - `Dispose`
    - Signature: `public var Dispose(...);`
  - `name`
    - Signature: `public var name { get; set; }`
  - `sizeInBytes`
    - Signature: `public var sizeInBytes { get; set; }`

## UnityEngine.Audio

### AudioSource

- Namespace/type: `UnityEngine.AudioSource`
- Use when: Audio playback component.
- Key public API:
  - `clip`
    - Signature: `public var clip { get; set; }`
  - `volume`
    - Signature: `public var volume { get; set; }`
  - `pitch`
    - Signature: `public var pitch { get; set; }`
  - `time`
    - Signature: `public var time { get; set; }`
  - `timeSamples`
    - Signature: `public var timeSamples { get; set; }`
  - `loop`
    - Signature: `public var loop { get; set; }`
  - `playOnAwake`
    - Signature: `public var playOnAwake { get; set; }`
  - `ignoreListenerVolume`
    - Signature: `public var ignoreListenerVolume { get; set; }`
  - `ignoreListenerPause`
    - Signature: `public var ignoreListenerPause { get; set; }`
  - `isVirtual`
    - Signature: `public var isVirtual { get; set; }`
  - `isPlaying`
    - Signature: `public var isPlaying { get; set; }`
  - `isPaused`
    - Signature: `public var isPaused { get; set; }`
  - `outputAudioMixerGroup`
    - Signature: `public var outputAudioMixerGroup { get; set; }`
  - `priority`
    - Signature: `public var priority { get; set; }`
  - `spatialBlend`
    - Signature: `public var spatialBlend { get; set; }`
  - `spatialize`
    - Signature: `public var spatialize { get; set; }`
  - `spatializePostEffects`
    - Signature: `public var spatializePostEffects { get; set; }`
  - `reverbZoneMix`
    - Signature: `public var reverbZoneMix { get; set; }`
  - `bypassEffects`
    - Signature: `public var bypassEffects { get; set; }`
  - `bypassListenerEffects`
    - Signature: `public var bypassListenerEffects { get; set; }`
  - `bypassReverbZones`
    - Signature: `public var bypassReverbZones { get; set; }`
  - `dopplerLevel`
    - Signature: `public var dopplerLevel { get; set; }`
  - `spread`
    - Signature: `public var spread { get; set; }`
  - `minDistance`
    - Signature: `public var minDistance { get; set; }`
  - `maxDistance`
    - Signature: `public var maxDistance { get; set; }`
  - `rolloffMode`
    - Signature: `public var rolloffMode { get; set; }`
  - `panStereo`
    - Signature: `public var panStereo { get; set; }`
  - `Play`
    - Signature: `public var Play(...);`
  - `PlayDelayed`
    - Signature: `public var PlayDelayed(...);`
  - `PlayScheduled`
    - Signature: `public var PlayScheduled(...);`
  - `Stop`
    - Signature: `public var Stop(...);`
  - `StopScheduled`
    - Signature: `public var StopScheduled(...);`
  - `Pause`
    - Signature: `public var Pause(...);`
  - `UnPause`
    - Signature: `public var UnPause(...);`
  - `SetScheduledStartTime`
    - Signature: `public var SetScheduledStartTime(...);`
  - `SetScheduledEndTime`
    - Signature: `public var SetScheduledEndTime(...);`
  - `PlayOneShot`
    - Signature: `public var PlayOneShot(...);`
  - `PlayClipAtPoint`
    - Signature: `public var PlayClipAtPoint(...);`
  - `GetAmbisonicDecoderFloat`
    - Signature: `public var GetAmbisonicDecoderFloat(...);`
  - `SetAmbisonicDecoderFloat`
    - Signature: `public var SetAmbisonicDecoderFloat(...);`
  - `GetSpatializerFloat`
    - Signature: `public var GetSpatializerFloat(...);`
  - `SetSpatializerFloat`
    - Signature: `public var SetSpatializerFloat(...);`
  - `GetCustomCurve`
    - Signature: `public var GetCustomCurve(...);`
  - `SetCustomCurve`
    - Signature: `public var SetCustomCurve(...);`
  - `GetOutputData`
    - Signature: `public var GetOutputData(...);`
  - `GetSpectrumData`
    - Signature: `public var GetSpectrumData(...);`

### AudioClip

- Namespace/type: `UnityEngine.AudioClip`
- Use when: Audio sample data asset or generated clip.
- Key public API:
  - `length`
    - Signature: `public var length { get; set; }`
  - `samples`
    - Signature: `public var samples { get; set; }`
  - `channels`
    - Signature: `public var channels { get; set; }`
  - `frequency`
    - Signature: `public var frequency { get; set; }`
  - `loadState`
    - Signature: `public var loadState { get; set; }`
  - `loadType`
    - Signature: `public var loadType { get; set; }`
  - `preloadAudioData`
    - Signature: `public var preloadAudioData { get; set; }`
  - `ambisonic`
    - Signature: `public var ambisonic { get; set; }`
  - `LoadAudioData`
    - Signature: `public var LoadAudioData(...);`
  - `UnloadAudioData`
    - Signature: `public var UnloadAudioData(...);`
  - `SetData`
    - Signature: `public var SetData(...);`
  - `GetData`
    - Signature: `public var GetData(...);`
  - `Create`
    - Signature: `public var Create(...);`
  - `name`
    - Signature: `public var name { get; set; }`
  - `PCMReaderCallback`
    - Signature: `public var PCMReaderCallback(...);`
  - `PCMSetPositionCallback`
    - Signature: `public var PCMSetPositionCallback(...);`

### AudioListener

- Namespace/type: `UnityEngine.AudioListener`
- Use when: Scene audio listener and global output sampling.
- Key public API:
  - `volume`
    - Signature: `public var volume { get; set; }`
  - `pause`
    - Signature: `public var pause { get; set; }`
  - `GetOutputData`
    - Signature: `public var GetOutputData(...);`
  - `GetSpectrumData`
    - Signature: `public var GetSpectrumData(...);`
  - `GetOutputDataArray`
    - Signature: `public var GetOutputDataArray(...);`
  - `GetSpectrumDataArray`
    - Signature: `public var GetSpectrumDataArray(...);`

## UnityEngine.Input

### Input

- Namespace/type: `UnityEngine.Input`
- Use when: Legacy input manager polling API.
- Key public API:
  - `GetKey`
    - Signature: `public static var GetKey(...);`
  - `GetKeyDown`
    - Signature: `public static var GetKeyDown(...);`
  - `GetKeyUp`
    - Signature: `public static var GetKeyUp(...);`
  - `GetButton`
    - Signature: `public static var GetButton(...);`
  - `GetButtonDown`
    - Signature: `public static var GetButtonDown(...);`
  - `GetButtonUp`
    - Signature: `public static var GetButtonUp(...);`
  - `GetAxis`
    - Signature: `public static var GetAxis(...);`
  - `GetAxisRaw`
    - Signature: `public static var GetAxisRaw(...);`
  - `GetJoystickNames`
    - Signature: `public static var GetJoystickNames(...);`
  - `mousePosition`
    - Signature: `public var mousePosition { get; set; }`
  - `mouseScrollDelta`
    - Signature: `public var mouseScrollDelta { get; set; }`
  - `mousePresent`
    - Signature: `public var mousePresent { get; set; }`
  - `touchCount`
    - Signature: `public var touchCount { get; set; }`
  - `GetTouch`
    - Signature: `public static var GetTouch(...);`
  - `touches`
    - Signature: `public var touches { get; set; }`
  - `acceleration`
    - Signature: `public var acceleration { get; set; }`
  - `accelerationEventCount`
    - Signature: `public var accelerationEventCount { get; set; }`
  - `GetAccelerationEvent`
    - Signature: `public static var GetAccelerationEvent(...);`
  - `gyro`
    - Signature: `public var gyro { get; set; }`
  - `compass`
    - Signature: `public var compass { get; set; }`
  - `location`
    - Signature: `public var location { get; set; }`
  - `imeCompositionMode`
    - Signature: `public var imeCompositionMode { get; set; }`
  - `compositionString`
    - Signature: `public var compositionString { get; set; }`
  - `compositionCursorPos`
    - Signature: `public var compositionCursorPos { get; set; }`
  - `inputString`
    - Signature: `public var inputString { get; set; }`
  - `anyKey`
    - Signature: `public var anyKey { get; set; }`
  - `anyKeyDown`
    - Signature: `public var anyKeyDown { get; set; }`
  - `IsJoystickPreconfigured`
    - Signature: `public static var IsJoystickPreconfigured(...);`
  - `ResetInputAxes`
    - Signature: `public static var ResetInputAxes(...);`
  - `simulateMouseWithTouches`
    - Signature: `public var simulateMouseWithTouches { get; set; }`
  - `multiTouchEnabled`
    - Signature: `public var multiTouchEnabled { get; set; }`
  - `backButtonLeavesApp`
    - Signature: `public var backButtonLeavesApp { get; set; }`
  - `touchSupported`
    - Signature: `public var touchSupported { get; set; }`
  - `stylusTouchSupported`
    - Signature: `public var stylusTouchSupported { get; set; }`

## UnityEngine.SceneManagement

### SceneManager

- Namespace/type: `UnityEngine.SceneManagement.SceneManager`
- Use when: Runtime scene loading and active scene control.
- Key public API:
  - `GetActiveScene`
    - Signature: `public static var GetActiveScene(...);`
  - `SetActiveScene`
    - Signature: `public static var SetActiveScene(...);`
  - `CreateScene`
    - Signature: `public static var CreateScene(...);`
  - `GetSceneAt`
    - Signature: `public static var GetSceneAt(...);`
  - `GetSceneByBuildIndex`
    - Signature: `public static var GetSceneByBuildIndex(...);`
  - `GetSceneByName`
    - Signature: `public static var GetSceneByName(...);`
  - `GetSceneByPath`
    - Signature: `public static var GetSceneByPath(...);`
  - `LoadScene`
    - Signature: `public static var LoadScene(...);`
  - `LoadSceneAsync`
    - Signature: `public static var LoadSceneAsync(...);`
  - `UnloadSceneAsync`
    - Signature: `public static var UnloadSceneAsync(...);`
  - `MergeScenes`
    - Signature: `public static var MergeScenes(...);`
  - `MoveGameObjectToScene`
    - Signature: `public static var MoveGameObjectToScene(...);`
  - `sceneCount`
    - Signature: `public var sceneCount { get; set; }`
  - `sceneCountInBuildSettings`
    - Signature: `public var sceneCountInBuildSettings { get; set; }`
  - `sceneLoaded`
    - Signature: `public event Action sceneLoaded;`
  - `sceneUnloaded`
    - Signature: `public event Action sceneUnloaded;`
  - `activeSceneChanged`
    - Signature: `public event Action activeSceneChanged;`
- Common pattern:
```csharp
Scene scene = SceneManager.GetActiveScene();
foreach (var root in scene.GetRootGameObjects()) Debug.Log(root.name);
```

### Scene

- Namespace/type: `UnityEngine.SceneManagement.Scene`
- Use when: Value handle for a loaded or known scene.
- Key public API:
  - `name`
    - Signature: `public var name { get; set; }`
  - `path`
    - Signature: `public var path { get; set; }`
  - `isLoaded`
    - Signature: `public var isLoaded { get; set; }`
  - `isDirty`
    - Signature: `public var isDirty { get; set; }`
  - `buildIndex`
    - Signature: `public var buildIndex { get; set; }`
  - `rootCount`
    - Signature: `public var rootCount { get; set; }`
  - `GetRootGameObjects`
    - Signature: `public var GetRootGameObjects(...);`
  - `isSubScene`
    - Signature: `public var isSubScene { get; set; }`
  - `handle`
    - Signature: `public var handle { get; set; }`
  - `IsValid`
    - Signature: `public var IsValid(...);`

## UnityEngine.Serialization

### SerializeField

- Namespace/type: `UnityEngine.SerializeField`
- Use when: Attribute to serialize private fields.
- Key public API:
  - `[SerializeField]`
    - Signature: `[SerializeField]`

### SerializeReference

- Namespace/type: `UnityEngine.SerializeReference`
- Use when: Attribute for managed-reference polymorphic serialization.
- Key public API:
  - `[SerializeReference]`
    - Signature: `[SerializeReference]`

### FormerlySerializedAsAttribute

- Namespace/type: `UnityEngine.Serialization.FormerlySerializedAsAttribute`
- Use when: Field rename migration attribute.
- Key public API:
  - `FormerlySerializedAsAttribute`
    - Signature: `public var FormerlySerializedAsAttribute(...);`
  - `oldName`
    - Signature: `public var oldName { get; set; }`

### ISerializationCallbackReceiver

- Namespace/type: `UnityEngine.ISerializationCallbackReceiver`
- Use when: Serialization callbacks for custom data preparation.
- Key public API:
  - `OnBeforeSerialize`
    - Signature: `public var OnBeforeSerialize(...);`
  - `OnAfterDeserialize`
    - Signature: `public var OnAfterDeserialize(...);`

### ManagedReferenceUtility

- Namespace/type: `UnityEngine.Serialization.ManagedReferenceUtility`
- Use when: Managed reference ID utilities.
- Key public API:
  - `GetValue`
    - Signature: `public static var GetValue(...);`
  - `GetValues`
    - Signature: `public static var GetValues(...);`
  - `Assign`
    - Signature: `public static var Assign(...);`
  - `GetActualTypeFor`
    - Signature: `public static var GetActualTypeFor(...);`
  - `GetManagedReferenceIdForObject`
    - Signature: `public static var GetManagedReferenceIdForObject(...);`
  - `SetManagedReferenceIdForObject`
    - Signature: `public static var SetManagedReferenceIdForObject(...);`

## UnityEngine.Debug / Logging / Assertions

### Debug

- Namespace/type: `UnityEngine.Debug`
- Use when: Logging, assertions, and debug drawing.
- Key public API:
  - `Log`
    - Signature: `public static var Log(...);`
  - `LogWarning`
    - Signature: `public static var LogWarning(...);`
  - `LogError`
    - Signature: `public static var LogError(...);`
  - `LogFormat`
    - Signature: `public static var LogFormat(...);`
  - `LogWarningFormat`
    - Signature: `public static var LogWarningFormat(...);`
  - `LogErrorFormat`
    - Signature: `public static var LogErrorFormat(...);`
  - `LogException`
    - Signature: `public static var LogException(...);`
  - `Assert`
    - Signature: `public static var Assert(...);`
  - `AssertFormat`
    - Signature: `public static var AssertFormat(...);`
  - `DrawLine`
    - Signature: `public static var DrawLine(...);`
  - `DrawRay`
    - Signature: `public static var DrawRay(...);`
  - `Break`
    - Signature: `public static var Break(...);`
  - `DebugBreak`
    - Signature: `public static var DebugBreak(...);`
  - `ClearDeveloperConsole`
    - Signature: `public static var ClearDeveloperConsole(...);`
  - `isDebugBuild`
    - Signature: `public var isDebugBuild { get; set; }`
  - `developerConsoleVisible`
    - Signature: `public var developerConsoleVisible { get; set; }`
  - `unityLogger`
    - Signature: `public var unityLogger { get; set; }`

### Logger

- Namespace/type: `UnityEngine.Logger`
- Use when: ILogger implementation with handler and filtering.
- Key public API:
  - `logHandler`
    - Signature: `public var logHandler { get; set; }`
  - `logEnabled`
    - Signature: `public var logEnabled { get; set; }`
  - `filterLogType`
    - Signature: `public var filterLogType { get; set; }`
  - `Log`
    - Signature: `public var Log(...);`
  - `LogFormat`
    - Signature: `public var LogFormat(...);`
  - `LogWarning`
    - Signature: `public var LogWarning(...);`
  - `LogWarningFormat`
    - Signature: `public var LogWarningFormat(...);`
  - `LogError`
    - Signature: `public var LogError(...);`
  - `LogErrorFormat`
    - Signature: `public var LogErrorFormat(...);`
  - `LogException`
    - Signature: `public var LogException(...);`
  - `IsLogTypeAllowed`
    - Signature: `public var IsLogTypeAllowed(...);`

### Assert

- Namespace/type: `UnityEngine.Assertions.Assert`
- Use when: Runtime assertion helpers.
- Key public API:
  - `IsTrue`
    - Signature: `public static var IsTrue(...);`
  - `IsFalse`
    - Signature: `public static var IsFalse(...);`
  - `IsNull`
    - Signature: `public static var IsNull(...);`
  - `IsNotNull`
    - Signature: `public static var IsNotNull(...);`
  - `AreEqual`
    - Signature: `public static var AreEqual(...);`
  - `AreNotEqual`
    - Signature: `public static var AreNotEqual(...);`
  - `AreApproximatelyEqual`
    - Signature: `public static var AreApproximatelyEqual(...);`
  - `AreNotApproximatelyEqual`
    - Signature: `public static var AreNotApproximatelyEqual(...);`
  - `Fail`
    - Signature: `public static var Fail(...);`

## UnityEngine.UIElements (UI Toolkit Runtime)

### VisualElement

- Namespace/type: `UnityEngine.UIElements.VisualElement`
- Use when: Core retained-mode UI node.
- Key public API:
  - `name`
    - Signature: `public var name { get; set; }`
  - `userData`
    - Signature: `public var userData { get; set; }`
  - `enabledSelf`
    - Signature: `public var enabledSelf { get; set; }`
  - `visible`
    - Signature: `public var visible { get; set; }`
  - `display`
    - Signature: `public var display { get; set; }`
  - `style`
    - Signature: `public var style { get; set; }`
  - `styleSheets`
    - Signature: `public var styleSheets { get; set; }`
  - `classList`
    - Signature: `public var classList { get; set; }`
  - `hierarchy`
    - Signature: `public var hierarchy { get; set; }`
  - `parent`
    - Signature: `public var parent { get; set; }`
  - `childCount`
    - Signature: `public var childCount { get; set; }`
  - `contentRect`
    - Signature: `public var contentRect { get; set; }`
  - `layout`
    - Signature: `public var layout { get; set; }`
  - `worldBound`
    - Signature: `public var worldBound { get; set; }`
  - `localBound`
    - Signature: `public var localBound { get; set; }`
  - `resolvedStyle`
    - Signature: `public var resolvedStyle { get; set; }`
  - `Add`
    - Signature: `public var Add(...);`
  - `Insert`
    - Signature: `public var Insert(...);`
  - `Remove`
    - Signature: `public var Remove(...);`
  - `RemoveAt`
    - Signature: `public var RemoveAt(...);`
  - `RemoveFromHierarchy`
    - Signature: `public var RemoveFromHierarchy(...);`
  - `Clear`
    - Signature: `public var Clear(...);`
  - `GetFirstChild`
    - Signature: `public var GetFirstChild(...);`
  - `GetLastChild`
    - Signature: `public var GetLastChild(...);`
  - `ElementAt`
    - Signature: `public var ElementAt(...);`
  - `Contains`
    - Signature: `public var Contains(...);`
  - `IndexOf`
    - Signature: `public var IndexOf(...);`
  - `Find`
    - Signature: `public var Find(...);`
  - `Q`
    - Signature: `public var Q(...);`
  - `Query`
    - Signature: `public var Query(...);`
  - `RegisterCallback`
    - Signature: `public var RegisterCallback(...);`
  - `UnregisterCallback`
    - Signature: `public var UnregisterCallback(...);`
  - `RegisterCallbackOnce`
    - Signature: `public var RegisterCallbackOnce(...);`
  - `SendEvent`
    - Signature: `public var SendEvent(...);`
  - `AddToClassList`
    - Signature: `public var AddToClassList(...);`
  - `RemoveFromClassList`
    - Signature: `public var RemoveFromClassList(...);`
  - `ToggleInClassList`
    - Signature: `public var ToggleInClassList(...);`
  - `EnableInClassList`
    - Signature: `public var EnableInClassList(...);`
  - `ContainsClass`
    - Signature: `public var ContainsClass(...);`
  - `SetEnabled`
    - Signature: `public var SetEnabled(...);`
  - `MarkDirtyRepaint`
    - Signature: `public var MarkDirtyRepaint(...);`
  - `schedule`
    - Signature: `public var schedule { get; set; }`
  - `visualTreeAssetSource`
    - Signature: `public var visualTreeAssetSource { get; set; }`
  - `Focus`
    - Signature: `public var Focus(...);`
  - `Blur`
    - Signature: `public var Blur(...);`
  - `CaptureMouse`
    - Signature: `public var CaptureMouse(...);`
  - `ReleaseMouse`
    - Signature: `public var ReleaseMouse(...);`
- Common pattern:
```csharp
var root = new VisualElement();
root.AddToClassList("panel");
root.RegisterCallback<ClickEvent>(_ => Debug.Log("clicked"));
```

### TemplateContainer

- Namespace/type: `UnityEngine.UIElements.TemplateContainer`
- Use when: Root element produced by VisualTreeAsset cloning.
- Key public API:
  - `templateId`
    - Signature: `public var templateId { get; set; }`
  - `contentContainer`
    - Signature: `public var contentContainer { get; set; }`
  - `Add`
    - Signature: `public var Add(...);`
  - `Clear`
    - Signature: `public var Clear(...);`
  - `Q`
    - Signature: `public var Q(...);`
  - `Query`
    - Signature: `public var Query(...);`

### StyleSheet

- Namespace/type: `UnityEngine.UIElements.StyleSheet`
- Use when: USS style sheet asset.
- Key public API:
  - `contentHash`
    - Signature: `public var contentHash { get; set; }`

## UnityEditor Core

All APIs in this section are editor-only unless explicitly noted. Put calls under an editor assembly, `#if UNITY_EDITOR`, or an Editor folder.

### Editor

- Namespace/type: `UnityEditor.Editor`
- Use when: [Editor] Custom inspector base class.
- Key public API:
  - `target`
    - Signature: `public var target { get; set; }`
  - `targets`
    - Signature: `public var targets { get; set; }`
  - `serializedObject`
    - Signature: `public var serializedObject { get; set; }`
  - `DrawDefaultInspector`
    - Signature: `public var DrawDefaultInspector(...);`
  - `Repaint`
    - Signature: `public var Repaint(...);`
  - `OnInspectorGUI`
    - Signature: `public var OnInspectorGUI(...);`
  - `OnSceneGUI`
    - Signature: `public var OnSceneGUI(...);`
  - `HasPreviewGUI`
    - Signature: `public var HasPreviewGUI(...);`
  - `GetPreviewTitle`
    - Signature: `public var GetPreviewTitle(...);`
  - `OnPreviewGUI`
    - Signature: `public var OnPreviewGUI(...);`
  - `OnInteractivePreviewGUI`
    - Signature: `public var OnInteractivePreviewGUI(...);`
  - `OnPreviewSettings`
    - Signature: `public var OnPreviewSettings(...);`
  - `RequiresConstantRepaint`
    - Signature: `public var RequiresConstantRepaint(...);`
  - `DrawHeader`
    - Signature: `public var DrawHeader(...);`
  - `DrawFoldoutHeader`
    - Signature: `public var DrawFoldoutHeader(...);`
  - `CreateInspectorGUI`
    - Signature: `public var CreateInspectorGUI(...);`
  - `CreateCachedEditor`
    - Signature: `public var CreateCachedEditor(...);`
  - `CreateEditor`
    - Signature: `public var CreateEditor(...);`

### EditorWindow

- Namespace/type: `UnityEditor.EditorWindow`
- Use when: [Editor] Dockable editor window.
- Key public API:
  - `GetWindow`
    - Signature: `public var GetWindow(...);`
  - `GetWindowWithRect`
    - Signature: `public var GetWindowWithRect(...);`
  - `CreateWindow`
    - Signature: `public var CreateWindow(...);`
  - `Show`
    - Signature: `public var Show(...);`
  - `ShowModal`
    - Signature: `public var ShowModal(...);`
  - `ShowAuxWindow`
    - Signature: `public var ShowAuxWindow(...);`
  - `ShowUtility`
    - Signature: `public var ShowUtility(...);`
  - `Close`
    - Signature: `public var Close(...);`
  - `Repaint`
    - Signature: `public var Repaint(...);`
  - `SendEvent`
    - Signature: `public var SendEvent(...);`
  - `wantsMouseEnterLeaveWindow`
    - Signature: `public var wantsMouseEnterLeaveWindow { get; set; }`
  - `position`
    - Signature: `public var position { get; set; }`
  - `minSize`
    - Signature: `public var minSize { get; set; }`
  - `maxSize`
    - Signature: `public var maxSize { get; set; }`
  - `titleContent`
    - Signature: `public var titleContent { get; set; }`
  - `autoRepaintOnSceneChange`
    - Signature: `public var autoRepaintOnSceneChange { get; set; }`
  - `focusedWindow`
    - Signature: `public var focusedWindow { get; set; }`
  - `mouseOverWindow`
    - Signature: `public var mouseOverWindow { get; set; }`
  - `rootVisualElement`
    - Signature: `public var rootVisualElement { get; set; }`
  - `hasFocus`
    - Signature: `public var hasFocus { get; set; }`
  - `docked`
    - Signature: `public var docked { get; set; }`
  - `Focus`
    - Signature: `public var Focus(...);`
  - `ShowNotification`
    - Signature: `public var ShowNotification(...);`
  - `RemoveNotification`
    - Signature: `public var RemoveNotification(...);`
  - `OnEnable`
    - Signature: `public var OnEnable(...);`
  - `OnDisable`
    - Signature: `public var OnDisable(...);`
  - `OnGUI`
    - Signature: `public var OnGUI(...);`
  - `CreateGUI`
    - Signature: `public var CreateGUI(...);`
  - `Update`
    - Signature: `public var Update(...);`
  - `OnFocus`
    - Signature: `public var OnFocus(...);`
  - `OnLostFocus`
    - Signature: `public var OnLostFocus(...);`
- Common pattern:
```csharp
public class MyWindow : EditorWindow {
    [MenuItem("Window/My Window")] static void Open() => GetWindow<MyWindow>();
    void CreateGUI() => rootVisualElement.Add(new Label("Ready"));
}
```

### Selection

- Namespace/type: `UnityEditor.Selection`
- Use when: [Editor] Current editor selection.
- Key public API:
  - `activeGameObject`
    - Signature: `public var activeGameObject { get; set; }`
  - `activeTransform`
    - Signature: `public var activeTransform { get; set; }`
  - `activeObject`
    - Signature: `public var activeObject { get; set; }`
  - `activeInstanceIDs`
    - Signature: `public var activeInstanceIDs { get; set; }`
  - `activeContext`
    - Signature: `public var activeContext { get; set; }`
  - `gameObjects`
    - Signature: `public var gameObjects { get; set; }`
  - `transforms`
    - Signature: `public var transforms { get; set; }`
  - `objects`
    - Signature: `public var objects { get; set; }`
  - `instanceIDs`
    - Signature: `public var instanceIDs { get; set; }`
  - `Contains`
    - Signature: `public var Contains(...);`
  - `GetFiltered`
    - Signature: `public var GetFiltered(...);`
  - `GetTransforms`
    - Signature: `public var GetTransforms(...);`
  - `SetActiveObjectWithContext`
    - Signature: `public var SetActiveObjectWithContext(...);`
  - `selectionChanged`
    - Signature: `public event Action selectionChanged;`

### Undo

- Namespace/type: `UnityEditor.Undo`
- Use when: [Editor] Undo/redo recording.
- Key public API:
  - `RecordObject`
    - Signature: `public var RecordObject(...);`
  - `RecordObjects`
    - Signature: `public var RecordObjects(...);`
  - `RegisterCreatedObjectUndo`
    - Signature: `public var RegisterCreatedObjectUndo(...);`
  - `AddComponent`
    - Signature: `public var AddComponent(...);`
  - `DestroyObjectImmediate`
    - Signature: `public var DestroyObjectImmediate(...);`
  - `SetTransformParent`
    - Signature: `public var SetTransformParent(...);`
  - `MoveGameObjectToScene`
    - Signature: `public var MoveGameObjectToScene(...);`
  - `SetCurrentGroupName`
    - Signature: `public var SetCurrentGroupName(...);`
  - `GetCurrentGroupName`
    - Signature: `public var GetCurrentGroupName(...);`
  - `IncrementGroupIndex`
    - Signature: `public var IncrementGroupIndex(...);`
  - `IncrementCurrentGroup`
    - Signature: `public var IncrementCurrentGroup(...);`
  - `GetCurrentGroup`
    - Signature: `public var GetCurrentGroup(...);`
  - `CollapseUndoOperations`
    - Signature: `public var CollapseUndoOperations(...);`
  - `PerformUndo`
    - Signature: `public var PerformUndo(...);`
  - `PerformRedo`
    - Signature: `public var PerformRedo(...);`
  - `ClearUndo`
    - Signature: `public var ClearUndo(...);`
  - `RevertAllInCurrentGroup`
    - Signature: `public var RevertAllInCurrentGroup(...);`
  - `RevertAllDownToGroup`
    - Signature: `public var RevertAllDownToGroup(...);`
  - `GetGroupName`
    - Signature: `public var GetGroupName(...);`
  - `FlushUndoRecordObjects`
    - Signature: `public var FlushUndoRecordObjects(...);`
  - `undoRedoPerformed`
    - Signature: `public event Action undoRedoPerformed;`
  - `postprocessModifications`
    - Signature: `public var postprocessModifications { get; set; }`
  - `willFlushUndoRecord`
    - Signature: `public var willFlushUndoRecord { get; set; }`

### EditorApplication

- Namespace/type: `UnityEditor.EditorApplication`
- Use when: [Editor] Global editor loop, play mode, compiling, and menu hooks.
- Key public API:
  - `update`
    - Signature: `public event Action update;`
  - `delayCall`
    - Signature: `public event Action delayCall;`
  - `delayedCall`
    - Signature: `public event Action delayedCall;`
  - `isPlaying`
    - Signature: `public var isPlaying { get; set; }`
  - `isPaused`
    - Signature: `public var isPaused { get; set; }`
  - `isCompiling`
    - Signature: `public var isCompiling { get; set; }`
  - `isUpdating`
    - Signature: `public var isUpdating { get; set; }`
  - `isPlayingOrWillChangePlaymode`
    - Signature: `public var isPlayingOrWillChangePlaymode { get; set; }`
  - `timeSinceStartup`
    - Signature: `public var timeSinceStartup { get; set; }`
  - `Beep`
    - Signature: `public var Beep(...);`
  - `Exit`
    - Signature: `public var Exit(...);`
  - `LockReloadAssemblies`
    - Signature: `public var LockReloadAssemblies(...);`
  - `UnlockReloadAssemblies`
    - Signature: `public var UnlockReloadAssemblies(...);`
  - `Reload`
    - Signature: `public var Reload(...);`
  - `ExecuteMenuItem`
    - Signature: `public var ExecuteMenuItem(...);`
  - `hierarchyModeChanged`
    - Signature: `public event Action hierarchyModeChanged;`
  - `hierarchicalSorting`
    - Signature: `public var hierarchicalSorting { get; set; }`
  - `QueuePlayerLoopUpdate`
    - Signature: `public var QueuePlayerLoopUpdate(...);`
  - `SetTemporaryDatabaseProviders`
    - Signature: `public var SetTemporaryDatabaseProviders(...);`
  - `SceneDragging`
    - Signature: `public var SceneDragging(...);`
  - `DoDelayedReload`
    - Signature: `public var DoDelayedReload(...);`
  - `Step`
    - Signature: `public var Step(...);`
  - `playModeStateChanged`
    - Signature: `public event Action playModeStateChanged;`
  - `pauseStateChanged`
    - Signature: `public event Action pauseStateChanged;`
  - `projectChanged`
    - Signature: `public event Action projectChanged;`
  - `hierarchyChanged`
    - Signature: `public event Action hierarchyChanged;`
  - `quitting`
    - Signature: `public event Action quitting;`
  - `modifierKeysChanged`
    - Signature: `public event Action modifierKeysChanged;`

### SerializedObject

- Namespace/type: `UnityEditor.SerializedObject`
- Use when: [Editor] Serialized wrapper for one or more Unity objects.
- Key public API:
  - `targetObject`
    - Signature: `public var targetObject { get; set; }`
  - `targetObjects`
    - Signature: `public var targetObjects { get; set; }`
  - `FindProperty`
    - Signature: `public var FindProperty(...);`
  - `FindPropertyRelative`
    - Signature: `public var FindPropertyRelative(...);`
  - `GetIterator`
    - Signature: `public var GetIterator(...);`
  - `ApplyModifiedProperties`
    - Signature: `public var ApplyModifiedProperties(...);`
  - `ApplyModifiedPropertiesWithoutUndo`
    - Signature: `public var ApplyModifiedPropertiesWithoutUndo(...);`
  - `Update`
    - Signature: `public var Update(...);`
  - `UpdateIfRequiredOrScript`
    - Signature: `public var UpdateIfRequiredOrScript(...);`
  - `hasModifiedProperties`
    - Signature: `public var hasModifiedProperties { get; set; }`
  - `isEditingMultipleObjects`
    - Signature: `public var isEditingMultipleObjects { get; set; }`
  - `SetIsDifferentCacheDirty`
    - Signature: `public var SetIsDifferentCacheDirty(...);`
  - `Dispose`
    - Signature: `public var Dispose(...);`
  - `inspectorMode`
    - Signature: `public var inspectorMode { get; set; }`
  - `context`
    - Signature: `public var context { get; set; }`
- Common pattern:
```csharp
serializedObject.Update();
var prop = serializedObject.FindProperty("speed");
EditorGUILayout.PropertyField(prop);
serializedObject.ApplyModifiedProperties();
```

### SerializedProperty

- Namespace/type: `UnityEditor.SerializedProperty`
- Use when: [Editor] Cursor and value API for serialized fields.
- Key public API:
  - `propertyType`
    - Signature: `public var propertyType { get; set; }`
  - `name`
    - Signature: `public var name { get; set; }`
  - `displayName`
    - Signature: `public var displayName { get; set; }`
  - `type`
    - Signature: `public var type { get; set; }`
  - `path`
    - Signature: `public var path { get; set; }`
  - `propertyPath`
    - Signature: `public var propertyPath { get; set; }`
  - `depth`
    - Signature: `public var depth { get; set; }`
  - `isArray`
    - Signature: `public var isArray { get; set; }`
  - `arraySize`
    - Signature: `public var arraySize { get; set; }`
  - `editable`
    - Signature: `public var editable { get; set; }`
  - `isExpanded`
    - Signature: `public var isExpanded { get; set; }`
  - `boolValue`
    - Signature: `public var boolValue { get; set; }`
  - `floatValue`
    - Signature: `public var floatValue { get; set; }`
  - `doubleValue`
    - Signature: `public var doubleValue { get; set; }`
  - `intValue`
    - Signature: `public var intValue { get; set; }`
  - `longValue`
    - Signature: `public var longValue { get; set; }`
  - `stringValue`
    - Signature: `public var stringValue { get; set; }`
  - `colorValue`
    - Signature: `public var colorValue { get; set; }`
  - `vector2Value`
    - Signature: `public var vector2Value { get; set; }`
  - `vector3Value`
    - Signature: `public var vector3Value { get; set; }`
  - `vector4Value`
    - Signature: `public var vector4Value { get; set; }`
  - `quaternionValue`
    - Signature: `public var quaternionValue { get; set; }`
  - `rectValue`
    - Signature: `public var rectValue { get; set; }`
  - `boundsValue`
    - Signature: `public var boundsValue { get; set; }`
  - `enumValueIndex`
    - Signature: `public var enumValueIndex { get; set; }`
  - `enumDisplayName`
    - Signature: `public var enumDisplayName { get; set; }`
  - `objectReferenceValue`
    - Signature: `public var objectReferenceValue { get; set; }`
  - `exposedReferenceValue`
    - Signature: `public var exposedReferenceValue { get; set; }`
  - `managedReferenceValue`
    - Signature: `public var managedReferenceValue { get; set; }`
  - `managedReferenceFullTypename`
    - Signature: `public var managedReferenceFullTypename { get; set; }`
  - `Next`
    - Signature: `public var Next(...);`
  - `NextVisible`
    - Signature: `public var NextVisible(...);`
  - `Copy`
    - Signature: `public var Copy(...);`
  - `Reset`
    - Signature: `public var Reset(...);`
  - `DeleteArrayElementAtIndex`
    - Signature: `public var DeleteArrayElementAtIndex(...);`
  - `InsertArrayElementAtIndex`
    - Signature: `public var InsertArrayElementAtIndex(...);`
  - `GetArrayElementAtIndex`
    - Signature: `public var GetArrayElementAtIndex(...);`
  - `MoveArrayElement`
    - Signature: `public var MoveArrayElement(...);`
  - `FindPropertyRelative`
    - Signature: `public var FindPropertyRelative(...);`
  - `CountInProperty`
    - Signature: `public var CountInProperty(...);`
  - `CountRemaining`
    - Signature: `public var CountRemaining(...);`
  - `EqualContents`
    - Signature: `public var EqualContents(...);`

## UnityEditor Asset / Build

All APIs in this section are editor-only unless explicitly noted. Put calls under an editor assembly, `#if UNITY_EDITOR`, or an Editor folder.

### AssetDatabase

- Namespace/type: `UnityEditor.AssetDatabase`
- Use when: [Editor] Project asset database, GUID, import, labels, dependencies, and persistence.
- Key public API:
  - `ImportAsset`
    - Signature: `public static var ImportAsset(...);`
  - `Refresh`
    - Signature: `public static var Refresh(...);`
  - `SaveAssets`
    - Signature: `public static var SaveAssets(...);`
  - `FindAssets`
    - Signature: `public static var FindAssets(...);`
  - `LoadAssetAtPath`
    - Signature: `public static var LoadAssetAtPath(...);`
  - `LoadAllAssetsAtPath`
    - Signature: `public static var LoadAllAssetsAtPath(...);`
  - `LoadMainAssetAtPath`
    - Signature: `public static var LoadMainAssetAtPath(...);`
  - `CreateAsset`
    - Signature: `public static var CreateAsset(...);`
  - `AddObjectToAsset`
    - Signature: `public static var AddObjectToAsset(...);`
  - `CreateFolder`
    - Signature: `public static var CreateFolder(...);`
  - `DeleteAsset`
    - Signature: `public static var DeleteAsset(...);`
  - `MoveAsset`
    - Signature: `public static var MoveAsset(...);`
  - `MoveAssetToTrash`
    - Signature: `public static var MoveAssetToTrash(...);`
  - `RenameAsset`
    - Signature: `public static var RenameAsset(...);`
  - `CopyAsset`
    - Signature: `public static var CopyAsset(...);`
  - `GetAssetPath`
    - Signature: `public static var GetAssetPath(...);`
  - `GetAssetOrScenePath`
    - Signature: `public static var GetAssetOrScenePath(...);`
  - `GetGUIDFromAssetPath`
    - Signature: `public static var GetGUIDFromAssetPath(...);`
  - `GUIDToAssetPath`
    - Signature: `public static var GUIDToAssetPath(...);`
  - `AssetPathToGUID`
    - Signature: `public static var AssetPathToGUID(...);`
  - `GetTextMetaDataPathFromAssetPath`
    - Signature: `public static var GetTextMetaDataPathFromAssetPath(...);`
  - `GetDependencies`
    - Signature: `public static var GetDependencies(...);`
  - `GetCachedDependencyHash`
    - Signature: `public static var GetCachedDependencyHash(...);`
  - `Contains`
    - Signature: `public static var Contains(...);`
  - `IsValidFolder`
    - Signature: `public static var IsValidFolder(...);`
  - `GetLabels`
    - Signature: `public static var GetLabels(...);`
  - `SetLabels`
    - Signature: `public static var SetLabels(...);`
  - `IsOpenForEdit`
    - Signature: `public static var IsOpenForEdit(...);`
  - `IsEditable`
    - Signature: `public static var IsEditable(...);`
  - `NeedsResaving`
    - Signature: `public static var NeedsResaving(...);`
  - `ReleaseCachedFileHandles`
    - Signature: `public static var ReleaseCachedFileHandles(...);`
  - `IsNativeAsset`
    - Signature: `public static var IsNativeAsset(...);`
  - `IsForeignAsset`
    - Signature: `public static var IsForeignAsset(...);`
  - `IsMainAsset`
    - Signature: `public static var IsMainAsset(...);`
  - `IsSubAsset`
    - Signature: `public static var IsSubAsset(...);`
  - `GetMainAssetTypeAtPath`
    - Signature: `public static var GetMainAssetTypeAtPath(...);`
  - `GetMainAssetInstanceID`
    - Signature: `public static var GetMainAssetInstanceID(...);`
  - `SearchInStoreAssetPath`
    - Signature: `public static var SearchInStoreAssetPath(...);`
  - `StartAssetEditing`
    - Signature: `public static var StartAssetEditing(...);`
  - `StopAssetEditing`
    - Signature: `public static var StopAssetEditing(...);`
  - `DisallowAutoRefresh`
    - Signature: `public static var DisallowAutoRefresh(...);`
  - `AllowAutoRefresh`
    - Signature: `public static var AllowAutoRefresh(...);`
- Common pattern:
```csharp
string guid = AssetDatabase.AssetPathToGUID(path);
string assetPath = AssetDatabase.GUIDToAssetPath(guid);
var asset = AssetDatabase.LoadAssetAtPath<UnityEngine.Object>(assetPath);
```

### BuildPipeline

- Namespace/type: `UnityEditor.BuildPipeline`
- Use when: [Editor] Player and AssetBundle build entry points.
- Key public API:
  - `BuildPlayer`
    - Signature: `public static var BuildPlayer(...);`
  - `BuildAssetBundles`
    - Signature: `public static var BuildAssetBundles(...);`
  - `GetCRCForAssetBundle`
    - Signature: `public static var GetCRCForAssetBundle(...);`
  - `GetHashForAssetBundle`
    - Signature: `public static var GetHashForAssetBundle(...);`
  - `IsBuildTargetSupported`
    - Signature: `public static var IsBuildTargetSupported(...);`
  - `GetBuildTargetGroup`
    - Signature: `public static var GetBuildTargetGroup(...);`
  - `BuildPlayerWithDefaultSettings`
    - Signature: `public static var BuildPlayerWithDefaultSettings(...);`
- Common pattern:
```csharp
var report = BuildPipeline.BuildPlayer(options);
if (report.summary.result != BuildResult.Succeeded) throw new Exception(report.summary.result.ToString());
```

### BuildReport

- Namespace/type: `UnityEditor.Build.Reporting.BuildReport`
- Use when: [Editor] Build result details.
- Key public API:
  - `summary`
    - Signature: `public var summary { get; set; }`
  - `steps`
    - Signature: `public var steps { get; set; }`
  - `files`
    - Signature: `public var files { get; set; }`
  - `packedAssets`
    - Signature: `public var packedAssets { get; set; }`
  - `strippingInfo`
    - Signature: `public var strippingInfo { get; set; }`

### PlayerSettings

- Namespace/type: `UnityEditor.PlayerSettings`
- Use when: [Editor] Project player settings.
- Key public API:
  - `companyName`
    - Signature: `public var companyName { get; set; }`
  - `productName`
    - Signature: `public var productName { get; set; }`
  - `bundleVersion`
    - Signature: `public var bundleVersion { get; set; }`
  - `applicationIdentifier`
    - Signature: `public var applicationIdentifier { get; set; }`
  - `targetDevice`
    - Signature: `public var targetDevice { get; set; }`
  - `architecture`
    - Signature: `public var architecture { get; set; }`
  - `stripEngineCode`
    - Signature: `public var stripEngineCode { get; set; }`
  - `managedStrippingLevel`
    - Signature: `public var managedStrippingLevel { get; set; }`
  - `SetScriptingDefineSymbols`
    - Signature: `public static var SetScriptingDefineSymbols(...);`
  - `GetScriptingDefineSymbols`
    - Signature: `public static var GetScriptingDefineSymbols(...);`
  - `SetPlatformStreamingInstall`
    - Signature: `public static var SetPlatformStreamingInstall(...);`
  - `graphicsJobs`
    - Signature: `public var graphicsJobs { get; set; }`
  - `graphicsDeviceJobLegacy`
    - Signature: `public var graphicsDeviceJobLegacy { get; set; }`
  - `graphicsDeviceTypes`
    - Signature: `public var graphicsDeviceTypes { get; set; }`
  - `colorSpace`
    - Signature: `public var colorSpace { get; set; }`
  - `scriptCompilation`
    - Signature: `public var scriptCompilation { get; set; }`
  - `scriptingBackend`
    - Signature: `public var scriptingBackend { get; set; }`
  - `il2CppCompilerConfiguration`
    - Signature: `public var il2CppCompilerConfiguration { get; set; }`
  - `apiCompatibilityLevel`
    - Signature: `public var apiCompatibilityLevel { get; set; }`
  - `acceleratedRendering`
    - Signature: `public var acceleratedRendering { get; set; }`
  - `acceleratedRenderingTier`
    - Signature: `public var acceleratedRenderingTier { get; set; }`
  - `useIncrementalCompiler`
    - Signature: `public var useIncrementalCompiler { get; set; }`
  - `GetGraphicsAPIs`
    - Signature: `public static var GetGraphicsAPIs(...);`
  - `SetGraphicsAPIs`
    - Signature: `public static var SetGraphicsAPIs(...);`
  - `SetUseDefaultGraphicsAPIs`
    - Signature: `public static var SetUseDefaultGraphicsAPIs(...);`
  - `GetUseDefaultGraphicsAPIs`
    - Signature: `public static var GetUseDefaultGraphicsAPIs(...);`

### CompilationPipeline

- Namespace/type: `UnityEditor.Compilation.CompilationPipeline`
- Use when: [Editor] Script compilation and assembly metadata.
- Key public API:
  - `GetAssemblies`
    - Signature: `public static var GetAssemblies(...);`
  - `GetAssemblyDefinitionFilePathFromAssemblyName`
    - Signature: `public static var GetAssemblyDefinitionFilePathFromAssemblyName(...);`
  - `GetAssemblyDefinitionFilePathFromAssetPath`
    - Signature: `public static var GetAssemblyDefinitionFilePathFromAssetPath(...);`
  - `GetAssemblyDefinitionPlatforms`
    - Signature: `public static var GetAssemblyDefinitionPlatforms(...);`
  - `GetPrecompiledAssemblyNames`
    - Signature: `public static var GetPrecompiledAssemblyNames(...);`
  - `GetPrecompiledAssemblyPath`
    - Signature: `public static var GetPrecompiledAssemblyPath(...);`
  - `GetCompileErrors`
    - Signature: `public static var GetCompileErrors(...);`
  - `compilingAssembly`
    - Signature: `public var compilingAssembly { get; set; }`
  - `RequestScriptCompilation`
    - Signature: `public static var RequestScriptCompilation(...);`
  - `assemblyCompilationStarted`
    - Signature: `public event Action assemblyCompilationStarted;`
  - `assemblyCompilationFinished`
    - Signature: `public event Action assemblyCompilationFinished;`
  - `compilationStarted`
    - Signature: `public event Action compilationStarted;`
  - `compilationFinished`
    - Signature: `public event Action compilationFinished;`

### EditorUtility

- Namespace/type: `UnityEditor.EditorUtility`
- Use when: [Editor] Misc editor dialogs, dirty state, progress, and object helpers.
- Key public API:
  - `SetDirty`
    - Signature: `public static var SetDirty(...);`
  - `IsDirty`
    - Signature: `public static var IsDirty(...);`
  - `DisplayDialog`
    - Signature: `public static var DisplayDialog(...);`
  - `DisplayDialogComplex`
    - Signature: `public static var DisplayDialogComplex(...);`
  - `DisplayProgressBar`
    - Signature: `public static var DisplayProgressBar(...);`
  - `ClearProgressBar`
    - Signature: `public static var ClearProgressBar(...);`
  - `DisplayCancelableProgressBar`
    - Signature: `public static var DisplayCancelableProgressBar(...);`
  - `FormatBytes`
    - Signature: `public static var FormatBytes(...);`
  - `GetDialogOptOutDecision`
    - Signature: `public static var GetDialogOptOutDecision(...);`
  - `SetDialogOptOutDecision`
    - Signature: `public static var SetDialogOptOutDecision(...);`
  - `GetObjectEnabled`
    - Signature: `public static var GetObjectEnabled(...);`
  - `SetObjectEnabled`
    - Signature: `public static var SetObjectEnabled(...);`
  - `CollectDependencies`
    - Signature: `public static var CollectDependencies(...);`
  - `CollectDeepHierarchy`
    - Signature: `public static var CollectDeepHierarchy(...);`
  - `GetDirtyCount`
    - Signature: `public static var GetDirtyCount(...);`
  - `CopySerialized`
    - Signature: `public static var CopySerialized(...);`
  - `CopySerializedManagedFields`
    - Signature: `public static var CopySerializedManagedFields(...);`
  - `SetCustomDiffTool`
    - Signature: `public static var SetCustomDiffTool(...);`
  - `GetObjectHideFlags`
    - Signature: `public static var GetObjectHideFlags(...);`
  - `SetObjectHideFlags`
    - Signature: `public static var SetObjectHideFlags(...);`
  - `OpenPropertyEditor`
    - Signature: `public static var OpenPropertyEditor(...);`
  - `FocusProjectWindow`
    - Signature: `public static var FocusProjectWindow(...);`
  - `DelayedCall`
    - Signature: `public static var DelayedCall(...);`
  - `RequireRenderPipeChange`
    - Signature: `public static var RequireRenderPipeChange(...);`
  - `InstanceIDToObject`
    - Signature: `public static var InstanceIDToObject(...);`
  - `IsPersistent`
    - Signature: `public static var IsPersistent(...);`
  - `UnloadUnusedAssetsImmediate`
    - Signature: `public static var UnloadUnusedAssetsImmediate(...);`

### AssetImporter

- Namespace/type: `UnityEditor.AssetImporter`
- Use when: [Editor] Base importer settings for an asset path.
- Key public API:
  - `assetPath`
    - Signature: `public var assetPath { get; set; }`
  - `userData`
    - Signature: `public var userData { get; set; }`
  - `assetBundleName`
    - Signature: `public var assetBundleName { get; set; }`
  - `assetBundleVariant`
    - Signature: `public var assetBundleVariant { get; set; }`
  - `SaveAndReimport`
    - Signature: `public var SaveAndReimport(...);`
  - `SetAssetBundleNameAndVariant`
    - Signature: `public var SetAssetBundleNameAndVariant(...);`
  - `GetAtPath`
    - Signature: `public var GetAtPath(...);`
  - `importSettingsMissing`
    - Signature: `public var importSettingsMissing { get; set; }`

### MonoImporter

- Namespace/type: `UnityEditor.MonoImporter`
- Use when: [Editor] Script importer and default references.
- Key public API:
  - `GetRuntimePlatform`
    - Signature: `public var GetRuntimePlatform(...);`
  - `GetClass`
    - Signature: `public var GetClass(...);`
  - `SetClass`
    - Signature: `public var SetClass(...);`
  - `SetDefaultReferences`
    - Signature: `public var SetDefaultReferences(...);`
  - `GetDefaultReferences`
    - Signature: `public var GetDefaultReferences(...);`
  - `SetIcon`
    - Signature: `public var SetIcon(...);`
  - `GetIcon`
    - Signature: `public var GetIcon(...);`

## UnityEditor UI (IMGUI)

All APIs in this section are editor-only unless explicitly noted. Put calls under an editor assembly, `#if UNITY_EDITOR`, or an Editor folder.

### EditorGUI

- Namespace/type: `UnityEditor.EditorGUI`
- Use when: [Editor] Immediate-mode editor controls with explicit Rect.
- Key public API:
  - `LabelField`
    - Signature: `public var LabelField(...);`
  - `TextField`
    - Signature: `public var TextField(...);`
  - `TextArea`
    - Signature: `public var TextArea(...);`
  - `FloatField`
    - Signature: `public var FloatField(...);`
  - `IntField`
    - Signature: `public var IntField(...);`
  - `LongField`
    - Signature: `public var LongField(...);`
  - `DoubleField`
    - Signature: `public var DoubleField(...);`
  - `Slider`
    - Signature: `public var Slider(...);`
  - `IntSlider`
    - Signature: `public var IntSlider(...);`
  - `ColorField`
    - Signature: `public var ColorField(...);`
  - `BoundsField`
    - Signature: `public var BoundsField(...);`
  - `RectField`
    - Signature: `public var RectField(...);`
  - `Vector2Field`
    - Signature: `public var Vector2Field(...);`
  - `Vector3Field`
    - Signature: `public var Vector3Field(...);`
  - `Vector4Field`
    - Signature: `public var Vector4Field(...);`
  - `QuaternionField`
    - Signature: `public var QuaternionField(...);`
  - `ObjectField`
    - Signature: `public var ObjectField(...);`
  - `LayerField`
    - Signature: `public var LayerField(...);`
  - `LayerMaskField`
    - Signature: `public var LayerMaskField(...);`
  - `TagField`
    - Signature: `public var TagField(...);`
  - `MaskField`
    - Signature: `public var MaskField(...);`
  - `EnumPopup`
    - Signature: `public var EnumPopup(...);`
  - `Popup`
    - Signature: `public var Popup(...);`
  - `IntPopup`
    - Signature: `public var IntPopup(...);`
  - `ColorPicker`
    - Signature: `public var ColorPicker(...);`
  - `BeginChangeCheck`
    - Signature: `public var BeginChangeCheck(...);`
  - `EndChangeCheck`
    - Signature: `public var EndChangeCheck(...);`
  - `BeginDisabledGroup`
    - Signature: `public var BeginDisabledGroup(...);`
  - `EndDisabledGroup`
    - Signature: `public var EndDisabledGroup(...);`
  - `BeginProperty`
    - Signature: `public var BeginProperty(...);`
  - `EndProperty`
    - Signature: `public var EndProperty(...);`
  - `PrefixLabel`
    - Signature: `public var PrefixLabel(...);`
  - `Toggle`
    - Signature: `public var Toggle(...);`
  - `ToggleLeft`
    - Signature: `public var ToggleLeft(...);`
  - `Foldout`
    - Signature: `public var Foldout(...);`
  - `InspectorTitlebar`
    - Signature: `public var InspectorTitlebar(...);`
  - `HelpBox`
    - Signature: `public var HelpBox(...);`
  - `DrawCurveSwatch`
    - Signature: `public var DrawCurveSwatch(...);`
  - `ProgressBar`
    - Signature: `public var ProgressBar(...);`
  - `DropShadowLabel`
    - Signature: `public var DropShadowLabel(...);`
  - `SelectableLabel`
    - Signature: `public var SelectableLabel(...);`
  - `DrawPreviewTexture`
    - Signature: `public var DrawPreviewTexture(...);`
  - `DrawTextureAlpha`
    - Signature: `public var DrawTextureAlpha(...);`
  - `TextureField`
    - Signature: `public var TextureField(...);`
  - `PropertyField`
    - Signature: `public var PropertyField(...);`

### EditorGUILayout

- Namespace/type: `UnityEditor.EditorGUILayout`
- Use when: [Editor] IMGUI editor controls with automatic layout.
- Key public API:
  - `BeginHorizontal`
    - Signature: `public var BeginHorizontal(...);`
  - `EndHorizontal`
    - Signature: `public var EndHorizontal(...);`
  - `BeginVertical`
    - Signature: `public var BeginVertical(...);`
  - `EndVertical`
    - Signature: `public var EndVertical(...);`
  - `BeginFadeGroup`
    - Signature: `public var BeginFadeGroup(...);`
  - `EndFadeGroup`
    - Signature: `public var EndFadeGroup(...);`
  - `BeginScrollView`
    - Signature: `public var BeginScrollView(...);`
  - `EndScrollView`
    - Signature: `public var EndScrollView(...);`
  - `BeginToggleGroup`
    - Signature: `public var BeginToggleGroup(...);`
  - `EndToggleGroup`
    - Signature: `public var EndToggleGroup(...);`
  - `BeginDisabledGroup`
    - Signature: `public var BeginDisabledGroup(...);`
  - `EndDisabledGroup`
    - Signature: `public var EndDisabledGroup(...);`
  - `Space`
    - Signature: `public var Space(...);`
  - `Separator`
    - Signature: `public var Separator(...);`
  - `LabelField`
    - Signature: `public var LabelField(...);`
  - `TextField`
    - Signature: `public var TextField(...);`
  - `TextArea`
    - Signature: `public var TextArea(...);`
  - `FloatField`
    - Signature: `public var FloatField(...);`
  - `IntField`
    - Signature: `public var IntField(...);`
  - `LongField`
    - Signature: `public var LongField(...);`
  - `DoubleField`
    - Signature: `public var DoubleField(...);`
  - `Slider`
    - Signature: `public var Slider(...);`
  - `IntSlider`
    - Signature: `public var IntSlider(...);`
  - `ColorField`
    - Signature: `public var ColorField(...);`
  - `BoundsField`
    - Signature: `public var BoundsField(...);`
  - `RectField`
    - Signature: `public var RectField(...);`
  - `Vector2Field`
    - Signature: `public var Vector2Field(...);`
  - `Vector3Field`
    - Signature: `public var Vector3Field(...);`
  - `Vector4Field`
    - Signature: `public var Vector4Field(...);`
  - `ObjectField`
    - Signature: `public var ObjectField(...);`
  - `LayerField`
    - Signature: `public var LayerField(...);`
  - `TagField`
    - Signature: `public var TagField(...);`
  - `LayerMaskField`
    - Signature: `public var LayerMaskField(...);`
  - `MaskField`
    - Signature: `public var MaskField(...);`
  - `EnumPopup`
    - Signature: `public var EnumPopup(...);`
  - `Popup`
    - Signature: `public var Popup(...);`
  - `IntPopup`
    - Signature: `public var IntPopup(...);`
  - `Toggle`
    - Signature: `public var Toggle(...);`
  - `ToggleLeft`
    - Signature: `public var ToggleLeft(...);`
  - `Foldout`
    - Signature: `public var Foldout(...);`
  - `HelpBox`
    - Signature: `public var HelpBox(...);`
  - `InspectorTitlebar`
    - Signature: `public var InspectorTitlebar(...);`
  - `ProgressBar`
    - Signature: `public var ProgressBar(...);`
  - `CurveField`
    - Signature: `public var CurveField(...);`
  - `GradientField`
    - Signature: `public var GradientField(...);`
  - `PropertyField`
    - Signature: `public var PropertyField(...);`
  - `GetControlRect`
    - Signature: `public var GetControlRect(...);`
  - `BeginChangeCheck`
    - Signature: `public var BeginChangeCheck(...);`
  - `EndChangeCheck`
    - Signature: `public var EndChangeCheck(...);`

## UnityEditor UIElements (UI Toolkit Editor)

All APIs in this section are editor-only unless explicitly noted. Put calls under an editor assembly, `#if UNITY_EDITOR`, or an Editor folder.

### EditorToolbar

- Namespace/type: `UnityEditor.Toolbars.EditorToolbar`
- Use when: [Editor] Toolbar container for editor tools.
- Key public API:
  - `Add`
    - Signature: `public var Add(...);`
  - `Insert`
    - Signature: `public var Insert(...);`
  - `Clear`
    - Signature: `public var Clear(...);`
  - `RegisterCallback`
    - Signature: `public var RegisterCallback(...);`

### Toolbar

- Namespace/type: `UnityEditor.UIElements.Toolbar`
- Use when: [Editor] Horizontal UI Toolkit toolbar.
- Key public API:
  - `Add`
    - Signature: `public var Add(...);`
  - `Insert`
    - Signature: `public var Insert(...);`
  - `Q`
    - Signature: `public var Q(...);`
  - `RegisterCallback`
    - Signature: `public var RegisterCallback(...);`
  - `contentContainer`
    - Signature: `public var contentContainer { get; set; }`

### ToolbarButton

- Namespace/type: `UnityEditor.UIElements.ToolbarButton`
- Use when: [Editor] Clickable toolbar button.
- Key public API:
  - `text`
    - Signature: `public var text { get; set; }`
  - `tooltip`
    - Signature: `public var tooltip { get; set; }`
  - `icon`
    - Signature: `public var icon { get; set; }`
  - `clicked`
    - Signature: `public event Action clicked;`
  - `SetEnabled`
    - Signature: `public var SetEnabled(...);`
  - `RegisterCallback`
    - Signature: `public var RegisterCallback(...);`

### ToolbarMenu

- Namespace/type: `UnityEditor.UIElements.ToolbarMenu`
- Use when: [Editor] Toolbar dropdown menu.
- Key public API:
  - `text`
    - Signature: `public var text { get; set; }`
  - `menu`
    - Signature: `public var menu { get; set; }`
  - `AppendAction`
    - Signature: `public var AppendAction(...);`
  - `AppendSeparator`
    - Signature: `public var AppendSeparator(...);`

### ToolbarPopupSearchField

- Namespace/type: `UnityEditor.UIElements.ToolbarPopupSearchField`
- Use when: [Editor] Toolbar search field with popup.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### ToolbarSearchField

- Namespace/type: `UnityEditor.UIElements.ToolbarSearchField`
- Use when: [Editor] Toolbar search text field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### ToolbarSpacer

- Namespace/type: `UnityEditor.UIElements.ToolbarSpacer`
- Use when: [Editor] Toolbar spacing element.
- Key public API:
  - `contentContainer`
    - Signature: `public var contentContainer { get; set; }`

### ToolbarBreadCrumb

- Namespace/type: `UnityEditor.UIElements.ToolbarBreadcrumbs`
- Use when: [Editor] Breadcrumb toolbar navigation.
- Key public API:
  - `PushItem`
    - Signature: `public var PushItem(...);`
  - `PopItem`
    - Signature: `public var PopItem(...);`
  - `Clear`
    - Signature: `public var Clear(...);`

### PropertyField

- Namespace/type: `UnityEditor.UIElements.PropertyField`
- Use when: [Editor] UI Toolkit SerializedProperty field.
- Key public API:
  - `bindingPath`
    - Signature: `public var bindingPath { get; set; }`
  - `label`
    - Signature: `public var label { get; set; }`
  - `Bind`
    - Signature: `public var Bind(...);`
  - `BindProperty`
    - Signature: `public var BindProperty(...);`
  - `Unbind`
    - Signature: `public var Unbind(...);`

### PropertyControl

- Namespace/type: `UnityEditor.UIElements.PropertyControl`
- Use when: [Editor] Property control base/helper.
- Key public API:
  - `bindingPath`
    - Signature: `public var bindingPath { get; set; }`
  - `label`
    - Signature: `public var label { get; set; }`
  - `Bind`
    - Signature: `public var Bind(...);`
  - `Unbind`
    - Signature: `public var Unbind(...);`

### ObjectField

- Namespace/type: `UnityEditor.UIElements.ObjectField`
- Use when: [Editor] Object reference picker field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `objectType`
    - Signature: `public var objectType { get; set; }`
  - `allowSceneObjects`
    - Signature: `public var allowSceneObjects { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### ColorField

- Namespace/type: `UnityEditor.UIElements.ColorField`
- Use when: [Editor] Color picker field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `showEyeDropper`
    - Signature: `public var showEyeDropper { get; set; }`
  - `showAlpha`
    - Signature: `public var showAlpha { get; set; }`
  - `hdr`
    - Signature: `public var hdr { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### CurveField

- Namespace/type: `UnityEditor.UIElements.CurveField`
- Use when: [Editor] AnimationCurve field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `ranges`
    - Signature: `public var ranges { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### GradientField

- Namespace/type: `UnityEditor.UIElements.GradientField`
- Use when: [Editor] Gradient field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `hdr`
    - Signature: `public var hdr { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### EnumField

- Namespace/type: `UnityEditor.UIElements.EnumField`
- Use when: [Editor] Enum dropdown field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `Init`
    - Signature: `public var Init(...);`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### FloatField

- Namespace/type: `UnityEngine.UIElements.FloatField`
- Use when: Numeric float UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `isDelayed`
    - Signature: `public var isDelayed { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### IntegerField

- Namespace/type: `UnityEngine.UIElements.IntegerField`
- Use when: Numeric int UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `isDelayed`
    - Signature: `public var isDelayed { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### LongField

- Namespace/type: `UnityEngine.UIElements.LongField`
- Use when: Numeric long UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `isDelayed`
    - Signature: `public var isDelayed { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### DoubleField

- Namespace/type: `UnityEngine.UIElements.DoubleField`
- Use when: Numeric double UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `isDelayed`
    - Signature: `public var isDelayed { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### Vector2Field

- Namespace/type: `UnityEngine.UIElements.Vector2Field`
- Use when: Vector2 UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### Vector3Field

- Namespace/type: `UnityEngine.UIElements.Vector3Field`
- Use when: Vector3 UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### Vector4Field

- Namespace/type: `UnityEngine.UIElements.Vector4Field`
- Use when: Vector4 UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### RectField

- Namespace/type: `UnityEngine.UIElements.RectField`
- Use when: Rect UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### BoundsField

- Namespace/type: `UnityEngine.UIElements.BoundsField`
- Use when: Bounds UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### MaskField

- Namespace/type: `UnityEngine.UIElements.MaskField`
- Use when: Bitmask UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `choices`
    - Signature: `public var choices { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### LayerMaskField

- Namespace/type: `UnityEditor.UIElements.LayerMaskField`
- Use when: [Editor] Layer mask field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### LayerField

- Namespace/type: `UnityEditor.UIElements.LayerField`
- Use when: [Editor] Layer selection field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### TagField

- Namespace/type: `UnityEditor.UIElements.TagField`
- Use when: [Editor] Tag selection field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### TextField

- Namespace/type: `UnityEngine.UIElements.TextField`
- Use when: Text UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `multiline`
    - Signature: `public var multiline { get; set; }`
  - `isPasswordField`
    - Signature: `public var isPasswordField { get; set; }`
  - `isDelayed`
    - Signature: `public var isDelayed { get; set; }`
  - `maxLength`
    - Signature: `public var maxLength { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`
  - `SelectAll`
    - Signature: `public var SelectAll(...);`

### Toggle

- Namespace/type: `UnityEngine.UIElements.Toggle`
- Use when: Boolean UI Toolkit field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `text`
    - Signature: `public var text { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### PopupField

- Namespace/type: `UnityEngine.UIElements.PopupField<T>`
- Use when: Generic popup selection field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `index`
    - Signature: `public var index { get; set; }`
  - `choices`
    - Signature: `public var choices { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### DropdownField

- Namespace/type: `UnityEngine.UIElements.DropdownField`
- Use when: String dropdown field.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `index`
    - Signature: `public var index { get; set; }`
  - `choices`
    - Signature: `public var choices { get; set; }`
  - `RegisterValueChangedCallback`
    - Signature: `public var RegisterValueChangedCallback(...);`
  - `SetValueWithoutNotify`
    - Signature: `public var SetValueWithoutNotify(...);`

### InspectorElement

- Namespace/type: `UnityEditor.UIElements.InspectorElement`
- Use when: [Editor] UI Toolkit inspector host.
- Key public API:
  - `Bind`
    - Signature: `public var Bind(...);`
  - `Unbind`
    - Signature: `public var Unbind(...);`
  - `FillDefaultInspector`
    - Signature: `public var FillDefaultInspector(...);`

### InspectorView

- Namespace/type: `UnityEditor.InspectorWindow / InspectorView`
- Use when: [Editor] Inspector view integration concept.
- Key public API:
  - `InspectorElement`
    - Signature: `public var InspectorElement(...);`
  - `FillDefaultInspector`
    - Signature: `public var FillDefaultInspector(...);`

## Unity.Collections (Native Collections)

### NativeArray<T>

- Namespace/type: `Unity.Collections.NativeArray<T>`
- Use when: Fixed-length native memory array.
- Key public API:
  - `Length`
    - Signature: `public var Length { get; set; }`
  - `IsCreated`
    - Signature: `public var IsCreated { get; set; }`
  - `this[int]`
    - Signature: `public var this[int] { get; set; }`
  - `CopyFrom`
    - Signature: `public var CopyFrom(...);`
  - `CopyTo`
    - Signature: `public var CopyTo(...);`
  - `GetEnumerator`
    - Signature: `public var GetEnumerator(...);`
  - `Dispose`
    - Signature: `public var Dispose(...);`
  - `AsReadOnly`
    - Signature: `public var AsReadOnly(...);`
  - `GetUnsafeReadOnlyPtr`
    - Signature: `public var GetUnsafeReadOnlyPtr(...);`
  - `GetUnsafePtr`
    - Signature: `public var GetUnsafePtr(...);`
  - `ToArray`
    - Signature: `public var ToArray(...);`
  - `Reinterpret`
    - Signature: `public var Reinterpret(...);`
  - `Slice`
    - Signature: `public var Slice(...);`

### NativeSlice<T>

- Namespace/type: `Unity.Collections.NativeSlice<T>`
- Use when: Strided view into native memory.
- Key public API:
  - `Length`
    - Signature: `public var Length { get; set; }`
  - `Stride`
    - Signature: `public var Stride { get; set; }`
  - `this[int]`
    - Signature: `public var this[int] { get; set; }`
  - `GetEnumerator`
    - Signature: `public var GetEnumerator(...);`
  - `CopyFrom`
    - Signature: `public var CopyFrom(...);`
  - `CopyTo`
    - Signature: `public var CopyTo(...);`
  - `GetUnsafeReadOnlyPtr`
    - Signature: `public var GetUnsafeReadOnlyPtr(...);`
  - `GetUnsafePtr`
    - Signature: `public var GetUnsafePtr(...);`
  - `Slice`
    - Signature: `public var Slice(...);`
  - `SliceWithStride`
    - Signature: `public var SliceWithStride(...);`
  - `AsReadOnly`
    - Signature: `public var AsReadOnly(...);`
  - `ToArray`
    - Signature: `public var ToArray(...);`

### CollectionsPackageTypes

- Namespace/type: `Unity.Collections package`
- Use when: NativeList, NativeHashMap, and NativeQueue live in the Collections package, not UnityCsReference.
- Key public API:
  - `NativeList<T>`
    - Signature: `public var NativeList<T>(...);`
  - `NativeHashMap<TKey`
    - Signature: `public var NativeHashMap<TKey(...);`
  - `TValue>`
    - Signature: `public var TValue>(...);`
  - `NativeQueue<T>`
    - Signature: `public var NativeQueue<T>(...);`
- Note: `NativeList<T>`, `NativeHashMap<TKey, TValue>`, and `NativeQueue<T>` are in `com.unity.collections`, not UnityCsReference core.

## Unity.Mathematics (External Package - NOT in UnityCsReference)

### math

- Namespace/type: `Unity.Mathematics.math`
- Use when: Static math functions for Burst/DOTS-style code.
- Key public API:
  - `sin`
    - Signature: `public var sin { get; set; }`
  - `cos`
    - Signature: `public var cos { get; set; }`
  - `tan`
    - Signature: `public var tan { get; set; }`
  - `asin`
    - Signature: `public var asin { get; set; }`
  - `acos`
    - Signature: `public var acos { get; set; }`
  - `atan`
    - Signature: `public var atan { get; set; }`
  - `atan2`
    - Signature: `public var atan2 { get; set; }`
  - `sqrt`
    - Signature: `public var sqrt { get; set; }`
  - `rsqrt`
    - Signature: `public var rsqrt { get; set; }`
  - `abs`
    - Signature: `public var abs { get; set; }`
  - `min`
    - Signature: `public var min { get; set; }`
  - `max`
    - Signature: `public var max { get; set; }`
  - `clamp`
    - Signature: `public var clamp { get; set; }`
  - `saturate`
    - Signature: `public var saturate { get; set; }`
  - `lerp`
    - Signature: `public var lerp { get; set; }`
  - `lerpunclamped`
    - Signature: `public var lerpunclamped { get; set; }`
  - `smoothstep`
    - Signature: `public var smoothstep { get; set; }`
  - `step`
    - Signature: `public var step { get; set; }`
  - `sign`
    - Signature: `public var sign { get; set; }`
  - `pow`
    - Signature: `public var pow { get; set; }`
  - `exp`
    - Signature: `public var exp { get; set; }`
  - `log`
    - Signature: `public var log { get; set; }`
  - `log2`
    - Signature: `public var log2 { get; set; }`
  - `log10`
    - Signature: `public var log10 { get; set; }`
  - `floor`
    - Signature: `public var floor { get; set; }`
  - `ceil`
    - Signature: `public var ceil { get; set; }`
  - `round`
    - Signature: `public var round { get; set; }`
  - `trunc`
    - Signature: `public var trunc { get; set; }`
  - `frac`
    - Signature: `public var frac { get; set; }`
  - `mad`
    - Signature: `public var mad { get; set; }`
  - `remap`
    - Signature: `public var remap { get; set; }`
  - `select`
    - Signature: `public var select { get; set; }`
  - `shuffle`
    - Signature: `public var shuffle { get; set; }`
  - `csum`
    - Signature: `public var csum { get; set; }`
  - `dot`
    - Signature: `public var dot { get; set; }`
  - `cross`
    - Signature: `public var cross { get; set; }`
  - `length`
    - Signature: `public var length { get; set; }`
  - `lengthsq`
    - Signature: `public var lengthsq { get; set; }`
  - `normalize`
    - Signature: `public var normalize { get; set; }`
  - `normalizesafe`
    - Signature: `public var normalizesafe { get; set; }`
  - `distance`
    - Signature: `public var distance { get; set; }`
  - `reflect`
    - Signature: `public var reflect { get; set; }`
  - `refract`
    - Signature: `public var refract { get; set; }`
  - `ortho`
    - Signature: `public var ortho { get; set; }`
  - `project`
    - Signature: `public var project { get; set; }`
  - `any`
    - Signature: `public var any { get; set; }`
  - `all`
    - Signature: `public var all { get; set; }`
  - `bitfield`
    - Signature: `public var bitfield { get; set; }`
  - `countbits`
    - Signature: `public var countbits { get; set; }`
  - `reversebits`
    - Signature: `public var reversebits { get; set; }`
  - `lzcnt`
    - Signature: `public var lzcnt { get; set; }`
  - `tzcnt`
    - Signature: `public var tzcnt { get; set; }`
  - `countleadingzeros`
    - Signature: `public var countleadingzeros { get; set; }`
  - `counttrailingzeros`
    - Signature: `public var counttrailingzeros { get; set; }`
  - `asfloat`
    - Signature: `public var asfloat { get; set; }`
  - `ashalf`
    - Signature: `public var ashalf { get; set; }`
  - `asint`
    - Signature: `public var asint { get; set; }`
  - `asuint`
    - Signature: `public var asuint { get; set; }`
  - `asdouble`
    - Signature: `public var asdouble { get; set; }`
- Note: This is a separate package (`com.unity.mathematics`), included here only as a boundary reminder.

### float vectors

- Namespace/type: `Unity.Mathematics vector structs`
- Use when: SIMD-friendly scalar and vector types.
- Key public API:
  - `float2`
    - Signature: `public var float2 { get; set; }`
  - `float3`
    - Signature: `public var float3 { get; set; }`
  - `float4`
    - Signature: `public var float4 { get; set; }`
  - `double2`
    - Signature: `public var double2 { get; set; }`
  - `double3`
    - Signature: `public var double3 { get; set; }`
  - `double4`
    - Signature: `public var double4 { get; set; }`
  - `int2`
    - Signature: `public var int2 { get; set; }`
  - `int3`
    - Signature: `public var int3 { get; set; }`
  - `int4`
    - Signature: `public var int4 { get; set; }`
  - `uint2`
    - Signature: `public var uint2 { get; set; }`
  - `uint3`
    - Signature: `public var uint3 { get; set; }`
  - `uint4`
    - Signature: `public var uint4 { get; set; }`
  - `bool2`
    - Signature: `public var bool2 { get; set; }`
  - `bool3`
    - Signature: `public var bool3 { get; set; }`
  - `bool4`
    - Signature: `public var bool4 { get; set; }`
  - `half`
    - Signature: `public var half { get; set; }`
  - `half2`
    - Signature: `public var half2 { get; set; }`
  - `half3`
    - Signature: `public var half3 { get; set; }`
  - `half4`
    - Signature: `public var half4 { get; set; }`
- Note: This is a separate package (`com.unity.mathematics`), included here only as a boundary reminder.

### quaternion

- Namespace/type: `Unity.Mathematics.quaternion`
- Use when: Mathematics package quaternion.
- Key public API:
  - `value`
    - Signature: `public var value { get; set; }`
  - `identity`
    - Signature: `public var identity { get; set; }`
  - `Euler`
    - Signature: `public var Euler(...);`
  - `AxisAngle`
    - Signature: `public var AxisAngle(...);`
  - `LookRotation`
    - Signature: `public var LookRotation(...);`
  - `LookRotationSafe`
    - Signature: `public var LookRotationSafe(...);`
  - `RotateX`
    - Signature: `public var RotateX(...);`
  - `RotateY`
    - Signature: `public var RotateY(...);`
  - `RotateZ`
    - Signature: `public var RotateZ(...);`
  - `Slerp`
    - Signature: `public var Slerp(...);`
  - `Normalize`
    - Signature: `public var Normalize(...);`
  - `conjugate`
    - Signature: `public var conjugate { get; set; }`
  - `inverse`
    - Signature: `public var inverse { get; set; }`
- Note: This is a separate package (`com.unity.mathematics`), included here only as a boundary reminder.

### matrices

- Namespace/type: `Unity.Mathematics matrix structs`
- Use when: Mathematics package matrices.
- Key public API:
  - `float3x3`
    - Signature: `public var float3x3 { get; set; }`
  - `float3x4`
    - Signature: `public var float3x4 { get; set; }`
  - `float4x3`
    - Signature: `public var float4x3 { get; set; }`
  - `float4x4`
    - Signature: `public var float4x4 { get; set; }`
  - `double3x3`
    - Signature: `public var double3x3 { get; set; }`
  - `double4x4`
    - Signature: `public var double4x4 { get; set; }`
  - `mul`
    - Signature: `public var mul { get; set; }`
  - `transpose`
    - Signature: `public var transpose { get; set; }`
  - `inverse`
    - Signature: `public var inverse { get; set; }`
  - `determinant`
    - Signature: `public var determinant { get; set; }`
- Note: This is a separate package (`com.unity.mathematics`), included here only as a boundary reminder.

### Random

- Namespace/type: `Unity.Mathematics.Random`
- Use when: Deterministic random generator struct.
- Key public API:
  - `CreateFromIndex`
    - Signature: `public var CreateFromIndex(...);`
  - `InitState`
    - Signature: `public var InitState(...);`
  - `NextInt`
    - Signature: `public var NextInt(...);`
  - `NextUInt`
    - Signature: `public var NextUInt(...);`
  - `NextFloat`
    - Signature: `public var NextFloat(...);`
  - `NextDouble`
    - Signature: `public var NextDouble(...);`
  - `NextFloat2`
    - Signature: `public var NextFloat2(...);`
  - `NextFloat3`
    - Signature: `public var NextFloat3(...);`
  - `NextFloat4`
    - Signature: `public var NextFloat4(...);`
  - `NextBool`
    - Signature: `public var NextBool(...);`
  - `NextState`
    - Signature: `public var NextState(...);`
  - `state`
    - Signature: `public var state { get; set; }`
- Note: This is a separate package (`com.unity.mathematics`), included here only as a boundary reminder.

## UnityEngine.Pool

### ObjectPool<T>

- Namespace/type: `UnityEngine.Pool.ObjectPool<T>`
- Use when: Stack-based object pool with callbacks.
- Key public API:
  - `Get`
    - Signature: `public var Get(...);`
  - `Release`
    - Signature: `public var Release(...);`
  - `Clear`
    - Signature: `public var Clear(...);`
  - `Dispose`
    - Signature: `public var Dispose(...);`
  - `CountActive`
    - Signature: `public var CountActive { get; set; }`
  - `CountInactive`
    - Signature: `public var CountInactive { get; set; }`
  - `CountAll`
    - Signature: `public var CountAll { get; set; }`
  - `OnGet`
    - Signature: `public var OnGet(...);`
  - `OnRelease`
    - Signature: `public var OnRelease(...);`
  - `OnDestroy`
    - Signature: `public var OnDestroy(...);`
- Common pattern:
```csharp
var pool = new ObjectPool<GameObject>(Create, OnGet, OnRelease, Destroy);
GameObject item = pool.Get();
pool.Release(item);
```

### GenericPool<T>

- Namespace/type: `UnityEngine.Pool.GenericPool<T>`
- Use when: Static generic pool for default-constructible objects.
- Key public API:
  - `Get`
    - Signature: `public static var Get(...);`
  - `Release`
    - Signature: `public static var Release(...);`

### LinkedPool<T>

- Namespace/type: `UnityEngine.Pool.LinkedPool<T>`
- Use when: Linked-list object pool.
- Key public API:
  - `Get`
    - Signature: `public var Get(...);`
  - `Release`
    - Signature: `public var Release(...);`
  - `Clear`
    - Signature: `public var Clear(...);`
  - `Dispose`
    - Signature: `public var Dispose(...);`
  - `CountActive`
    - Signature: `public var CountActive { get; set; }`
  - `CountInactive`
    - Signature: `public var CountInactive { get; set; }`
  - `CountAll`
    - Signature: `public var CountAll { get; set; }`

### UnsafeObjectPool<T>

- Namespace/type: `UnityEngine.Pool.UnsafeObjectPool<T>`
- Use when: Pool without collection safety checks.
- Key public API:
  - `Get`
    - Signature: `public var Get(...);`
  - `Release`
    - Signature: `public var Release(...);`
  - `Clear`
    - Signature: `public var Clear(...);`
  - `Dispose`
    - Signature: `public var Dispose(...);`
  - `CountActive`
    - Signature: `public var CountActive { get; set; }`
  - `CountInactive`
    - Signature: `public var CountInactive { get; set; }`
  - `CountAll`
    - Signature: `public var CountAll { get; set; }`

### CollectionPool<TCollection,TItem>

- Namespace/type: `UnityEngine.Pool.CollectionPool<TCollection,TItem>`
- Use when: Static pool for ICollection containers.
- Key public API:
  - `Get`
    - Signature: `public static var Get(...);`
  - `Release`
    - Signature: `public static var Release(...);`

### DictionaryPool<TKey,TValue>

- Namespace/type: `UnityEngine.Pool.DictionaryPool<TKey,TValue>`
- Use when: Static Dictionary pool.
- Key public API:
  - `Get`
    - Signature: `public static var Get(...);`
  - `Release`
    - Signature: `public static var Release(...);`

### HashSetPool<T>

- Namespace/type: `UnityEngine.Pool.HashSetPool<T>`
- Use when: Static HashSet pool.
- Key public API:
  - `Get`
    - Signature: `public static var Get(...);`
  - `Release`
    - Signature: `public static var Release(...);`

### ListPool<T>

- Namespace/type: `UnityEngine.Pool.ListPool<T>`
- Use when: Static List pool.
- Key public API:
  - `Get`
    - Signature: `public static var Get(...);`
  - `Release`
    - Signature: `public static var Release(...);`

## UnityEngine.Networking

### UnityWebRequest

- Namespace/type: `UnityEngine.Networking.UnityWebRequest`
- Use when: HTTP request object.
- Key public API:
  - `url`
    - Signature: `public var url { get; set; }`
  - `method`
    - Signature: `public var method { get; set; }`
  - `result`
    - Signature: `public var result { get; set; }`
  - `responseCode`
    - Signature: `public var responseCode { get; set; }`
  - `error`
    - Signature: `public var error { get; set; }`
  - `isDone`
    - Signature: `public var isDone { get; set; }`
  - `downloadHandler`
    - Signature: `public var downloadHandler { get; set; }`
  - `uploadHandler`
    - Signature: `public var uploadHandler { get; set; }`
  - `certificateHandler`
    - Signature: `public var certificateHandler { get; set; }`
  - `redirects`
    - Signature: `public var redirects { get; set; }`
  - `redirectLimit`
    - Signature: `public var redirectLimit { get; set; }`
  - `timeout`
    - Signature: `public var timeout { get; set; }`
  - `SendWebRequest`
    - Signature: `public var SendWebRequest(...);`
  - `Abort`
    - Signature: `public var Abort(...);`
  - `SetRequestHeader`
    - Signature: `public var SetRequestHeader(...);`
  - `GetRequestHeader`
    - Signature: `public var GetRequestHeader(...);`
  - `GetResponseHeader`
    - Signature: `public var GetResponseHeader(...);`
  - `GetResponseHeaders`
    - Signature: `public var GetResponseHeaders(...);`
  - `Dispose`
    - Signature: `public var Dispose(...);`
  - `Get`
    - Signature: `public var Get(...);`
  - `Post`
    - Signature: `public var Post(...);`
  - `Put`
    - Signature: `public var Put(...);`
  - `Delete`
    - Signature: `public var Delete(...);`
  - `Head`
    - Signature: `public var Head(...);`
- Common pattern:
```csharp
using var request = UnityWebRequest.Get(url);
yield return request.SendWebRequest();
if (request.result != UnityWebRequest.Result.Success) Debug.LogError(request.error);
```

### DownloadHandler

- Namespace/type: `UnityEngine.Networking.DownloadHandler`
- Use when: Base response body handler.
- Key public API:
  - `data`
    - Signature: `public var data { get; set; }`
  - `text`
    - Signature: `public var text { get; set; }`
  - `nativeData`
    - Signature: `public var nativeData { get; set; }`
  - `GetContent`
    - Signature: `public var GetContent(...);`
  - `ReceiveData`
    - Signature: `public var ReceiveData(...);`
  - `ReceiveContentLengthHeader`
    - Signature: `public var ReceiveContentLengthHeader(...);`
  - `CompleteContent`
    - Signature: `public var CompleteContent(...);`
  - `GetProgress`
    - Signature: `public var GetProgress(...);`
  - `Dispose`
    - Signature: `public var Dispose(...);`
  - `isDone`
    - Signature: `public var isDone { get; set; }`
  - `error`
    - Signature: `public var error { get; set; }`

### DownloadHandlerBuffer

- Namespace/type: `UnityEngine.Networking.DownloadHandlerBuffer`
- Use when: Stores response bytes in memory.
- Key public API:
  - `data`
    - Signature: `public var data { get; set; }`
  - `text`
    - Signature: `public var text { get; set; }`
  - `nativeData`
    - Signature: `public var nativeData { get; set; }`
  - `GetContent`
    - Signature: `public var GetContent(...);`

### DownloadHandlerFile

- Namespace/type: `UnityEngine.Networking.DownloadHandlerFile`
- Use when: Streams response to a file.
- Key public API:
  - `removeFileOnAbort`
    - Signature: `public var removeFileOnAbort { get; set; }`
  - `GetContent`
    - Signature: `public var GetContent(...);`

### DownloadHandlerAssetBundle

- Namespace/type: `UnityEngine.Networking.DownloadHandlerAssetBundle`
- Use when: Downloads AssetBundle content.
- Key public API:
  - `assetBundle`
    - Signature: `public var assetBundle { get; set; }`
  - `GetContent`
    - Signature: `public var GetContent(...);`

### DownloadHandlerTexture

- Namespace/type: `UnityEngine.Networking.DownloadHandlerTexture`
- Use when: Downloads image into Texture2D.
- Key public API:
  - `texture`
    - Signature: `public var texture { get; set; }`
  - `GetContent`
    - Signature: `public var GetContent(...);`

### DownloadHandlerScript

- Namespace/type: `UnityEngine.Networking.DownloadHandlerScript`
- Use when: Custom streamed download handler.
- Key public API:
  - `ReceiveData`
    - Signature: `public var ReceiveData(...);`
  - `CompleteContent`
    - Signature: `public var CompleteContent(...);`
  - `ReceiveContentLengthHeader`
    - Signature: `public var ReceiveContentLengthHeader(...);`
  - `GetProgress`
    - Signature: `public var GetProgress(...);`

### UploadHandler

- Namespace/type: `UnityEngine.Networking.UploadHandler`
- Use when: Base request body handler.
- Key public API:
  - `contentType`
    - Signature: `public var contentType { get; set; }`
  - `contentLength`
    - Signature: `public var contentLength { get; set; }`
  - `data`
    - Signature: `public var data { get; set; }`
  - `nativeData`
    - Signature: `public var nativeData { get; set; }`
  - `progress`
    - Signature: `public var progress { get; set; }`
  - `Dispose`
    - Signature: `public var Dispose(...);`

### UploadHandlerRaw

- Namespace/type: `UnityEngine.Networking.UploadHandlerRaw`
- Use when: Uploads byte array or NativeArray data.
- Key public API:
  - `contentType`
    - Signature: `public var contentType { get; set; }`
  - `data`
    - Signature: `public var data { get; set; }`
  - `Dispose`
    - Signature: `public var Dispose(...);`

### UploadHandlerFile

- Namespace/type: `UnityEngine.Networking.UploadHandlerFile`
- Use when: Uploads file content.
- Key public API:
  - `contentType`
    - Signature: `public var contentType { get; set; }`
  - `Dispose`
    - Signature: `public var Dispose(...);`

## UnityEngine.Subsystems

### Subsystem

- Namespace/type: `UnityEngine.SubsystemsImplementation.Subsystem`
- Use when: Base subsystem lifecycle.
- Key public API:
  - `running`
    - Signature: `public var running { get; set; }`
  - `Start`
    - Signature: `public var Start(...);`
  - `Stop`
    - Signature: `public var Stop(...);`
  - `Destroy`
    - Signature: `public var Destroy(...);`

### SubsystemDescriptor

- Namespace/type: `UnityEngine.SubsystemsImplementation.SubsystemDescriptor`
- Use when: Descriptor that creates subsystem instances.
- Key public API:
  - `id`
    - Signature: `public var id { get; set; }`
  - `subsystemImplementationType`
    - Signature: `public var subsystemImplementationType { get; set; }`
  - `Create`
    - Signature: `public var Create(...);`

### SubsystemManager

- Namespace/type: `UnityEngine.SubsystemManager`
- Use when: Global subsystem descriptor discovery.
- Key public API:
  - `GetAllSubsystemDescriptors`
    - Signature: `public static var GetAllSubsystemDescriptors(...);`
  - `GetSubsystemDescriptors`
    - Signature: `public static var GetSubsystemDescriptors(...);`
  - `GetInstances`
    - Signature: `public static var GetInstances(...);`
  - `beforeReloadSubsystems`
    - Signature: `public event Action beforeReloadSubsystems;`
  - `afterReloadSubsystems`
    - Signature: `public event Action afterReloadSubsystems;`

### ISubsystemDescriptor

- Namespace/type: `UnityEngine.ISubsystemDescriptor`
- Use when: Descriptor interface.
- Key public API:
  - `id`
    - Signature: `public var id { get; set; }`
  - `Create`
    - Signature: `public var Create(...);`

### ISubsystem

- Namespace/type: `UnityEngine.ISubsystem`
- Use when: Subsystem lifecycle interface.
- Key public API:
  - `running`
    - Signature: `public var running { get; set; }`
  - `Start`
    - Signature: `public var Start(...);`
  - `Stop`
    - Signature: `public var Stop(...);`
  - `Destroy`
    - Signature: `public var Destroy(...);`

## Notable Exclusions

- UGUI legacy reference coverage is intentionally minimal; use `UnityEngine.UI` package docs for Button, Canvas, Graphic, LayoutGroup, and EventSystem details.
- Unity internal, private, and protected implementation code from UnityCsReference is excluded.
- DOTS packages such as Entities, NetCode, Physics DOTS, and Burst are excluded except for package boundary notes.
- Unity.Mathematics is listed as an external package because it is not part of UnityCsReference core.
- Collections package containers such as NativeList, NativeHashMap, and NativeQueue are package APIs and not expanded here.
- SRP package internals for URP/HDRP are excluded; this reference only lists core rendering entry points shared by UnityEngine/UnityEditor.
- Platform-specific native plugins, XR provider internals, console SDK APIs, and proprietary modules are excluded.
- Obsolete APIs are not exhaustively listed; prefer current Unity 6 APIs and compiler warnings.
- Every overload is not listed; grouped signatures use `...` to keep this skill practical under the line budget.

## Cross-Reference with LUX Bridge

LUX bridge adapter code commonly needs these Unity APIs when inspecting or automating a Unity Editor project:
- Asset and script discovery: `AssetDatabase.FindAssets`, `GUIDToAssetPath`, `LoadAssetAtPath`, `GetDependencies`, `ImportAsset`, `Refresh`, `SaveAssets`.
- Editor loop and play state: `EditorApplication.update`, `delayCall`, `isPlaying`, `isCompiling`, `QueuePlayerLoopUpdate`, `playModeStateChanged`.
- Compilation evidence: `CompilationPipeline.GetAssemblies`, `GetCompileErrors`, compilation events.
- Selection and hierarchy context: `Selection.activeObject`, `Selection.gameObjects`, `SceneManager.GetActiveScene`, `Scene.GetRootGameObjects`, `GameObject.GetComponents`.
- Safe editor mutation: `Undo.RecordObject`, `SerializedObject`, `SerializedProperty`, `EditorUtility.SetDirty`, `AssetImporter.SaveAndReimport`.
- Runtime object inspection: `GameObject`, `Component`, `Transform`, `MonoBehaviour`, `Collider`, `Rigidbody`, `Camera`, `Light`, `Mesh`, `Material`.
- UI Toolkit bridge panels: `EditorWindow.rootVisualElement`, `VisualElement`, `PropertyField`, toolbar controls where a bridge UI is required.
- Build and report capture: `BuildPipeline.BuildPlayer`, `BuildReport.summary`, `PlayerSettings` platform settings.
- Network and local HTTP flows: `UnityWebRequest`, download/upload handlers for editor tooling that talks to local services.
- Logging evidence: `Debug.unityLogger`, `Application.logMessageReceived`, and editor console access via public editor APIs where available.

### LUX safety reminders

- Treat the Unity project and `.lux/` workspace as explicit state; do not hide failed editor operations behind fake success.
- Use GUIDs for durable asset identity and paths for user-facing reports.
- Use Undo and SerializedObject for user-visible editor mutations; use AssetDatabase APIs for persistent asset writes.
- Keep editor-only calls out of runtime assemblies.
- Do not depend on Unity internal classes copied from UnityCsReference.
