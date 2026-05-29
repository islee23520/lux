# Lux PlayMode Input
Commands for simulating user input during PlayMode.

## lux unity simulate-mouse-input
Simulates low-level mouse input (buttons, movement, scroll).

**Flags:**
- `--action <action>`: `click`, `long-press`, `move-delta`, `smooth-delta`, `scroll`.
- `--button <button>`: `left` (default), `right`, `middle`.
- `--delta-x`, `--delta-y`: Movement delta.
- `--scroll-x`, `--scroll-y`: Scroll delta.
- `--duration-ms`: Duration of the action.

**Usage:**
```bash
lux unity simulate-mouse-input --action click --button left
```

---

## lux unity simulate-keyboard
Simulates keyboard key presses.

**Flags:**
- `--action <action>`: `press`, `key-down`, `key-up`.
- `--key <key>`: Key name (e.g., "w", "space", "escape").
- `--duration-ms`: Duration for `press`.

**Usage:**
```bash
lux unity simulate-keyboard --action press --key space
```

---

## lux unity simulate-mouse-ui
Simulates mouse interaction with UI Toolkit elements using screen coordinates.

**Flags:**
- `--action <action>`: `click`, `long-press`, `drag-start`, `drag-move`, `drag-end`.
- `--x`, `--y`: Screen coordinates.
- `--duration-ms`: Duration of the action.

**Usage:**
```bash
lux unity simulate-mouse-ui --action click --x 100 --y 200
```
