---
name: unity-pattern-dependency-injection
description: "Dependency Injection"
category: reference
source: unity-design-patterns-skills
source_path: patterns/dependency-injection/SKILL.md
---

# Dependency Injection

## Purpose
Provide collaborators from outside the object so construction, ownership, and substitution stay explicit.

## Use When
- Scene lookups or singletons are hiding dependencies.
- Services should be replaceable in tests.
- Bootstrap code can own wiring decisions.

## Avoid When
- The feature is tiny and extra wiring adds more cost than clarity.
- A service locator is being used as fake DI.

## Unity Implementation Recipe
1. Identify real collaborators.
2. Inject them at construction or initialization boundaries.
3. Keep wiring in a composition root.
4. Use interfaces only where substitution matters.

## Common Pitfalls
- Mixing DI with hidden globals.
- Injecting after runtime logic already started.

## Minimal Unity Sketch
```csharp
public sealed class QuestPresenter
{
    private readonly IQuestRepository _quests;
    private readonly IClock _clock;

    public QuestPresenter(IQuestRepository quests, IClock clock)
    {
        _quests = quests;
        _clock = clock;
    }
}
```

## Response Shape
- Say whether **Dependency Injection** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
