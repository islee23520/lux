# Lux Screenshots
Commands for capturing visual state from the Unity Editor.

## lux unity screenshot
Captures a screenshot of the Unity Editor.

**Flags:**
- `--capture-mode <mode>`: `rendering` (default) or `game`.
- `--annotate-elements`: If true, includes UI element bounding boxes in the metadata.
- `--elements-only`: If true, returns only element metadata without saving the PNG (requires `--annotate-elements`).

**Usage:**
```bash
# Capture full rendering
lux unity screenshot

# Capture with UI annotations
lux unity screenshot --annotate-elements
```

**Expected Output:**
JSON containing `filePath` to the PNG, `mediaType`, and `annotatedElements` array if requested.
Screenshots are saved ephemerally under `Temp/Lux/Screenshots/`.

## Coordinate Workflow for UI Interaction

Annotated screenshots provide `simX`/`simY` coordinates for each UI element.
These are normalized screen coordinates ready for use with `simulate-mouse-ui`:

1. Capture with annotations:
   ```bash
   lux unity screenshot --annotate-elements
   ```
2. Extract `simX`/`simY` from the `annotatedElements` array in the JSON output.
3. Feed coordinates to UI interaction:
   ```bash
   lux unity simulate-mouse-ui --action click --x <simX> --y <simY>
   ```

Coordinates are in Unity screen space (origin bottom-left).
The annotation pipeline converts element bounding boxes to center-point screen coordinates.
No manual coordinate transformation is needed.
