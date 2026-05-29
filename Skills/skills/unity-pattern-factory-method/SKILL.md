---
name: unity-pattern-factory-method
description: "Factory Method"
category: reference
source: unity-design-patterns-skills
source_path: patterns/factory-method/SKILL.md
---

# Factory Method

## Purpose
Delegate concrete object creation to a factory method so callers depend on a product contract instead of product selection logic.

## Use When
- Callers should not know which concrete product variant to instantiate.
- Creation logic includes setup or wiring.
- Product variants are expected to grow.

## Avoid When
- There is only one trivial concrete type.
- The main issue is staged assembly, which fits Builder better.

## Unity Implementation Recipe
1. Define a product contract.
2. Move selection logic into a factory method or type.
3. Return ready-to-use products or document remaining initialization.
4. Test input-to-product mapping.

## Common Pitfalls
- A mega-factory with unrelated domain logic.
- Hidden side effects during creation.

## Minimal Unity Sketch
```csharp
public interface IProjectile { }

public sealed class ProjectileFactory
{
    public IProjectile Create(WeaponType type)
    {
        return type switch
        {
            WeaponType.Arrow => new ArrowProjectile(),
            WeaponType.Fireball => new FireballProjectile(),
            _ => throw new ArgumentOutOfRangeException(nameof(type))
        };
    }
}
```

## Response Shape
- Say whether **Factory Method** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
