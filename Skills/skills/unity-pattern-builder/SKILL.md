---
name: unity-pattern-builder
description: "Builder"
category: reference
source: unity-design-patterns-skills
source_path: patterns/builder/SKILL.md
---

# Builder

## Purpose
Construct complex objects step by step so call sites stay readable and validation happens before final creation.

## Use When
- Constructors are getting long or ambiguous.
- Optional settings or presets should compose cleanly.
- Creation needs validation before the object becomes usable.

## Avoid When
- A constructor or simple factory is already clear.
- The builder would only mirror trivial fields without adding safety or clarity.

## Unity Implementation Recipe
1. List required vs optional inputs.
2. Expose fluent methods with domain language.
3. Validate in Build().
4. Keep runtime behavior on the built object, not the builder.

## Common Pitfalls
- Allowing half-built objects to leak.
- Using a builder for a tiny type that does not need one.

## Minimal Unity Sketch
```csharp
public sealed class EnemyBuilder
{
    private int _hp = 100;
    private float _speed = 3f;
    private string _name = "Enemy";

    public EnemyBuilder Named(string name) { _name = name; return this; }
    public EnemyBuilder WithHp(int hp) { _hp = hp; return this; }
    public EnemyBuilder WithSpeed(float speed) { _speed = speed; return this; }

    public EnemyConfig Build() => new EnemyConfig(_name, _hp, _speed);
}
```

## Response Shape
- Say whether **Builder** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
