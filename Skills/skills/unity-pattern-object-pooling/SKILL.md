---
name: unity-pattern-object-pooling
description: "Object Pooling"
category: reference
source: unity-design-patterns-skills
source_path: patterns/object-pooling/SKILL.md
---

# Object Pooling

## Purpose
Reuse expensive or high-churn objects instead of allocating and destroying them repeatedly.

## Use When
- Bullets, VFX, UI rows, or enemies spawn frequently.
- GC churn or instantiate/destroy spikes are measurable.
- Objects can be reset predictably.

## Avoid When
- Objects spawn rarely.
- Reset rules are too fragile and stale state bugs are likely.

## Unity Implementation Recipe
1. Define acquire and release rules.
2. Reset transient state on release.
3. Prewarm based on actual usage.
4. Instrument misses, leaks, and growth.

## Common Pitfalls
- Reusing objects that still have listeners or coroutines attached.
- Pooling everything by default without evidence.

## Minimal Unity Sketch
```csharp
public sealed class BulletPool
{
    private readonly Queue<Bullet> _free = new();

    public Bullet Get() => _free.Count > 0 ? _free.Dequeue() : new Bullet();

    public void Release(Bullet bullet)
    {
        bullet.ResetState();
        _free.Enqueue(bullet);
    }
}
```

## Response Shape
- Say whether **Object Pooling** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
