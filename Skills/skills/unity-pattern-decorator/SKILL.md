---
name: unity-pattern-decorator
description: "Decorator"
category: reference
source: unity-design-patterns-skills
source_path: patterns/decorator/SKILL.md
---

# Decorator

## Purpose
Add responsibilities by wrapping an object with the same interface instead of creating subclasses for every combination.

## Use When
- Abilities or weapons need stackable runtime modifiers.
- You want composition instead of subclass explosion.
- Wrapper order should remain explicit.

## Avoid When
- Configuration alone solves the variation.
- Debugging wrapper chains would be harder than the original design.

## Unity Implementation Recipe
1. Define one stable component interface.
2. Create focused wrappers that delegate then add one behavior.
3. Build chains explicitly.
4. Test order-sensitive combinations.

## Common Pitfalls
- Undocumented wrapper order effects.
- Decorators depending on hidden global state.

## Minimal Unity Sketch
```csharp
public interface IDamageSource
{
    int GetDamage();
}

public sealed class CriticalHitDecorator : IDamageSource
{
    private readonly IDamageSource _inner;
    public CriticalHitDecorator(IDamageSource inner) => _inner = inner;
    public int GetDamage() => _inner.GetDamage() * 2;
}
```

## Response Shape
- Say whether **Decorator** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
