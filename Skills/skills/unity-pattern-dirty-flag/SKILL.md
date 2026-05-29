---
name: unity-pattern-dirty-flag
description: "Dirty Flag"
category: reference
source: unity-design-patterns-skills
source_path: patterns/dirty-flag/SKILL.md
---

# Dirty Flag

## Purpose
Track when source data changes so expensive derived work runs only when needed.

## Use When
- A cached derived value is read often and written rarely.
- Rebuild work is expensive enough to measure.
- Invalidation points are identifiable.

## Avoid When
- The computation is already cheap.
- Too many mutation paths make invalidation fragile.

## Unity Implementation Recipe
1. Separate source from cached derived data.
2. Mark dirty on every relevant mutation.
3. Rebuild lazily or at explicit sync points.
4. Clear the flag only after rebuild succeeds.

## Common Pitfalls
- Missing one invalidation path.
- Adding cache complexity without profiling.

## Minimal Unity Sketch
```csharp
public sealed class StatBlock
{
    private bool _dirty = true;
    private int _cachedPower;
    private readonly List<int> _modifiers = new();

    public void AddModifier(int value) { _modifiers.Add(value); _dirty = true; }
    public int GetPower()
    {
        if (_dirty)
        {
            _cachedPower = _modifiers.Sum();
            _dirty = false;
        }
        return _cachedPower;
    }
}
```

## Response Shape
- Say whether **Dirty Flag** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
