---
description: Open a game domain spec for editing (gdd, mechanics, controls, camera, etc.)
subtask: true
---

Open a Lux game domain markdown spec file for editing. The user should specify which domain to edit.

Available domains: gdd, mechanics, controls, camera, levels, art-style, audio, narrative, ui-ux, technical-architecture, engine, testing, build-release

Usage: `/lux-spec-edit <domain>`

If no domain is specified, list available domains and ask the user which one to edit.

Execute:

```bash
lux spec edit <domain>
```

After the editor closes, summarize the changes made to the domain spec.
