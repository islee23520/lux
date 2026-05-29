---
name: unity-pattern-adapter
description: "Adapter"
category: reference
source: unity-design-patterns-skills
source_path: patterns/adapter/SKILL.md
---

# Adapter

## Purpose
Translate an external or legacy interface into the interface your project actually wants to depend on.

## Use When
- A third-party SDK should not leak through gameplay code.
- Platform-specific services should look identical to the rest of the game.
- You need a stable seam for tests or future vendor swaps.

## Avoid When
- You already control the original interface design.
- The wrapper would become a full subsystem rewrite rather than a translation layer.

## Unity Implementation Recipe
1. Define a project-owned interface first.
2. Wrap the foreign API in a thin adapter.
3. Keep vendor types at the boundary.
4. Inject the adapter where the local interface is expected.

## Common Pitfalls
- Leaking third-party types past the boundary.
- Putting orchestration logic into the adapter.

## Minimal Unity Sketch
```csharp
public interface ICloudSave
{
    void Save(string key, string json);
}

public sealed class VendorCloudSaveAdapter : ICloudSave
{
    private readonly VendorSdk _sdk;

    public VendorCloudSaveAdapter(VendorSdk sdk) => _sdk = sdk;

    public void Save(string key, string json)
    {
        _sdk.Upload(key, json);
    }
}
```

## Response Shape
- Say whether **Adapter** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
