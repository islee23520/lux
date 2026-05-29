---
name: unity-pattern-mvp
description: "MVP"
category: reference
source: unity-design-patterns-skills
source_path: patterns/mvp/SKILL.md
---

# MVP

## Purpose
Keep UI presentation logic out of the view by routing it through a presenter that talks to a passive view contract.

## Use When
- A panel or HUD is accumulating too much logic.
- You want to test UI behavior without a live scene.
- View rendering and interaction rules should be separated.

## Avoid When
- The widget is tiny and short-lived.
- The view would still keep all domain logic.

## Unity Implementation Recipe
1. Define a passive view interface.
2. Move interaction logic into a presenter.
3. Keep model or service dependencies out of the view.
4. Manage subscriptions with scene lifetime.

## Common Pitfalls
- A presenter that becomes a pass-through because the view still owns the logic.
- Presenter code coupled directly to too many Unity APIs.

## Minimal Unity Sketch
```csharp
public interface IHealthView
{
    void SetValue(float normalized);
}

public sealed class HealthPresenter
{
    private readonly IHealthView _view;
    public HealthPresenter(IHealthView view) => _view = view;
    public void OnHealthChanged(int current, int max) => _view.SetValue((float)current / max);
}
```

## Response Shape
- Say whether **MVP** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
