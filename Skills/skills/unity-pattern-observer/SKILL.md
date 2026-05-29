---
name: unity-pattern-observer
description: "Observer"
category: reference
source: unity-design-patterns-skills
source_path: patterns/observer/SKILL.md
---

# Observer

## Purpose
Notify multiple listeners when state changes while keeping the source independent from the listeners.

## Use When
- UI, audio, and achievements react to the same source data.
- One source should fan out to multiple subscribers.
- Loose coupling matters more than strict call order.

## Avoid When
- Execution order is critical.
- One direct call is simpler and enough.

## Unity Implementation Recipe
1. Define explicit event payloads.
2. Subscribe and unsubscribe with lifecycle hooks.
3. Separate ongoing state change events from one-shot events.
4. Test unload and no-listener cases.

## Common Pitfalls
- Forgetting to unsubscribe.
- Broadcasting vague events that force listeners to re-query everything.

## Minimal Unity Sketch
```csharp
public sealed class PlayerHealth
{
    public event Action<int, int>? Changed;
    private int _current = 100;
    private int _max = 100;

    public void TakeDamage(int damage)
    {
        _current = Math.Max(0, _current - damage);
        Changed?.Invoke(_current, _max);
    }
}
```

## Response Shape
- Say whether **Observer** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
