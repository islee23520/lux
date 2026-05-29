# Lux Object and Hierarchy
Commands for searching GameObjects and exporting scene structure.

## lux unity find-game-objects
Searches for GameObjects in the active scene based on various filters.

**Flags:**
- `--search-mode <mode>`: `query` (default) or `Selected`.
- `--name <name>`: Filter by name.
- `--regex <pattern>`: Filter by name regex.
- `--path <path>`: Filter by hierarchy path.
- `--component <type>`: Filter by component type.
- `--tag <tag>`: Filter by tag.
- `--layer <layer>`: Filter by layer.
- `--active-state <state>`: `any` (default), `active`, or `inactive`.

**Usage:**
```bash
lux unity find-game-objects --name "Player"
```

---

## lux unity get-hierarchy
Exports the scene hierarchy to a persistent JSON file.

**Flags:**
- `--all`: Export the entire hierarchy (default).
- `--root-path <path>`: Export hierarchy starting from a specific path.
- `--use-selection`: Export hierarchy under the current selection.

**Usage:**
```bash
lux unity get-hierarchy --all
```

**Expected Output:**
JSON containing `filePath` to the exported hierarchy, `rootCount`, and `nodeCount`.
The hierarchy file is saved under `.lux/outputs/hierarchy/`.
