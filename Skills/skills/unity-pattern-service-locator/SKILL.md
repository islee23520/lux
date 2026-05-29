---
name: unity-pattern-service-locator
description: "Service Locator"
category: reference
source: unity-design-patterns-skills
source_path: patterns/service-locator/SKILL.md
---

# Service Locator

## Purpose
Resolve shared services through a central registry as a constrained bridge when explicit injection is not yet practical everywhere.

## Use When
- Legacy or callback-heavy code makes immediate DI hard.
- A temporary migration bridge is needed.
- Bootstrap can control registration order.

## Avoid When
- You are designing a new architecture from scratch.
- Hidden dependencies would materially hurt the feature.

## Unity Implementation Recipe
1. Constrain the allowed services.
2. Register at bootstrap.
3. Document lookup points.
4. Provide test reset or override hooks.

## Common Pitfalls
- A global grab-bag of unrelated services.
- Initialization order bugs.

## Minimal Unity Sketch
```csharp
public static class Services
{
    private static readonly Dictionary<Type, object> _map = new();
    public static void Register<T>(T service) where T : class => _map[typeof(T)] = service;
    public static T Get<T>() where T : class => (T)_map[typeof(T)];
}
```

## Response Shape
- Say whether **Service Locator** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
