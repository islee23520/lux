# Lux Skills Canonical Schema

## SKILL.md Format

### Required YAML Frontmatter
```yaml
---
name: kebab-case-skill-name  # MUST match directory name
description: "One-line description (max 200 chars)"
category: reference | workflow | studio  # Skill classification
source: unity-patterns | lux | chronos-adapted  # Origin provenance
---
```

### Required Body Sections
1. **Purpose** - What this skill does
2. **When to Use** - Trigger conditions  
3. **Instructions** - Step-by-step agent guidance
4. **Examples** (optional) - Code/output examples

### Constraints
- Max 500 lines per SKILL.md
- Use references/ subfolder for supplementary content
- English only in v1
