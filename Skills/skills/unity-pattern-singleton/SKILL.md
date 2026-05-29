---
name: unity-pattern-singleton
description: "Singleton"
category: reference
source: unity-design-patterns-skills
source_path: patterns/singleton/SKILL.md
---

# Singleton

## Purpose
Guarantee that one meaningful instance exists and provide one controlled access path to it.

## Use When
- Uniqueness is a real domain rule.
- A cross-scene service needs one lifetime owner.
- Bootstrap and teardown rules can be made explicit.

## Avoid When
- Global access is only being used for convenience.
- Multiple instances may be valid later.

## Unity Implementation Recipe
1. Document why uniqueness is required.
2. Handle duplicates and scene reloads explicitly.
3. Keep the API narrow.
4. Prefer composition over singleton if uniqueness is weak.

## Common Pitfalls
- Hidden global dependencies.
- Duplicate instances during startup or play-mode reload.

## Minimal Unity Sketch
```csharp
public sealed class GameConfig : MonoBehaviour
{
    public static GameConfig Instance { get; private set; } = null!;

    private void Awake()
    {
        if (Instance != null && Instance != this) { Destroy(gameObject); return; }
        Instance = this;
        DontDestroyOnLoad(gameObject);
    }
}
```

## Response Shape
- Say whether **Singleton** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
