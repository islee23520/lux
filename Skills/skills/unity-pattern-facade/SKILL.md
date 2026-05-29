---
name: unity-pattern-facade
description: "Facade"
category: reference
source: unity-design-patterns-skills
source_path: patterns/facade/SKILL.md
---

# Facade

## Purpose
Expose a small task-focused API over a complicated subsystem.

## Use When
- A subsystem requires repeated multi-step workflows.
- Most callers only need the common path.
- Cross-cutting validation or logging should be centralized.

## Avoid When
- You are only renaming a couple of methods.
- Callers genuinely need fine-grained control over every subsystem piece.

## Unity Implementation Recipe
1. Identify common workflows.
2. Design methods around those workflows, not subsystem class names.
3. Keep the common path obvious.
4. Document side effects and error handling.

## Common Pitfalls
- A facade that grows into a god service.
- Hiding important failure cases from callers.

## Minimal Unity Sketch
```csharp
public sealed class SaveGameFacade
{
    private readonly SaveSerializer _serializer;
    private readonly SaveStorage _storage;

    public SaveGameFacade(SaveSerializer serializer, SaveStorage storage)
    {
        _serializer = serializer;
        _storage = storage;
    }

    public void Save(PlayerProfile profile)
    {
        var json = _serializer.ToJson(profile);
        _storage.Write("profile.json", json);
    }
}
```

## Response Shape
- Say whether **Facade** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
