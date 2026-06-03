---
name: unity-cs-reference
description: "Lazy-load only when this workflow is explicitly needed. Use as a compact lookup guide for Unity Editor API decisions. Prefer public UnityEditor APIs for LUX bridge automation and keep Unity reference material under references/."
category: unity
source: lux
---

# Unity C# Reference

Developer-facing lookup guide for Unity C# API decisions in LUX bridge and editor automation work. The detailed API reference is stored in `references/unity-cs-api.md` so this skill hub stays small enough for release validation.

## Purpose

Use this skill to orient Unity C# implementation choices without copying Unity implementation code. Prefer public UnityEditor and UnityEngine APIs, and verify exact signatures against Unity documentation when parameter order or overload selection matters.

## When to Use

- Editing or reviewing Unity bridge C# code.
- Choosing public UnityEditor APIs for asset import, build, compilation, editor windows, package requests, serialization, or ScriptableObject persistence.
- Checking whether a runtime API belongs in player code or editor-only assemblies.
- Avoiding internal UnityCsReference implementation details.

## Instructions

1. Read this file first to confirm the request is a Unity C# reference lookup.
2. Open `references/unity-cs-api.md` only for the specific namespace or API area needed.
3. Use public API signatures and behavior summaries as guidance, not copied implementation code.
4. For editor-mutating APIs, pair changes with the appropriate Unity lifecycle operation: Undo, SerializedObject, SetDirty, SaveAssets, SaveAndReimport, or compilation/domain reload handling.
5. For native collections and async requests, verify allocator ownership, disposal, request completion state, and error reporting.
6. Report any version-sensitive assumption explicitly when Unity 6000.0+ behavior is not certain from local references.

## Reference Index

- `references/unity-cs-api.md`: UnityEngine, UnityEditor, UI Toolkit, PackageManager, Native Collections, Mathematics, pooling, networking, subsystem, and LUX bridge cross-reference notes.
