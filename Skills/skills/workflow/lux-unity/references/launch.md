# Lux Launch and Lifecycle
Commands for managing the Unity Editor process and PlayMode state.

## lux unity launch
Launches the Unity Editor for the current project.

**Flags:**
- `--no-wait`: If true, returns immediately after spawning the process. Otherwise, waits up to 60s for the Lux backend to become ready.

**Usage:**
```bash
lux unity launch
```

---

## lux unity control-play-mode
Controls the PlayMode state of the open Unity Editor.

**Flags:**
- `--action <action>`: `play`, `stop`, `pause`, `resume`, `status`.
- `--wait`: If true, waits for the state transition to complete (up to 15s).

**Usage:**
```bash
# Enter PlayMode and wait
lux unity control-play-mode --action play --wait
```

---

## lux unity context
Fetches or refreshes the shared Unity context snapshot.

**Flags:**
- `--refresh`: Forces a refresh of the context by running Unity in batch mode (use sparingly).

**Usage:**
```bash
lux unity context
```

---

## lux unity status
Reports Lux bridge metadata and settings.

**Usage:**
```bash
lux unity status
```

---

## lux unity focus-window
Brings the Unity Editor window to the foreground.

**Usage:**
```bash
lux unity focus-window
```
