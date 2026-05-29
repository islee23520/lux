---
name: unity-pattern-proxy
description: "Proxy"
category: reference
source: unity-design-patterns-skills
source_path: patterns/proxy/SKILL.md
---

# Proxy

## Purpose
Stand in for another object and control when or how the real object is reached.

## Use When
- Creation should be lazy or deferred.
- Access needs permissions, caching, or remoting behavior.
- Callers should still use the same contract.

## Avoid When
- You only need a simplified API, not controlled access semantics.
- The main issue is interface translation rather than access control.

## Unity Implementation Recipe
1. Keep the subject interface stable.
2. Choose the proxy role: lazy, protection, remote, or cache.
3. Make latency and failure behavior explicit.
4. Test caller expectations against both proxy and real subject.

## Common Pitfalls
- Hiding slow remote or load behavior behind an instant-looking API.
- Drifting semantics between proxy and real subject.

## Minimal Unity Sketch
```csharp
public sealed class LazyAudioBank : IAudioBank
{
    private AudioBank? _loaded;
    public AudioClip Get(string id)
    {
        _loaded ??= Resources.Load<AudioBank>("AudioBank");
        return _loaded.Get(id);
    }
}
```

## Response Shape
- Say whether **Proxy** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
