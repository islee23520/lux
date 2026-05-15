---
description: Open a domain spec for editing (design, architecture, audio, etc.)
subtask: true
---

Open a Lux domain markdown spec file for editing. The user should specify which domain to edit.

Available domains: design, architecture, art-style, audio, narrative, levels, ui-ux, testing, packages

Usage: `/lux-spec-edit <domain>`

If no domain is specified, list available domains and ask the user which one to edit.

Execute:

```bash
lux spec edit <domain>
```

After the editor closes, summarize the changes made to the domain spec.