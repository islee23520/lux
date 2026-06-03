# Lux Input Record and Replay
Commands for capturing and replaying PlayMode input sequences.

## lux unity record-input
Starts or stops recording Input System device-state changes.

**Flags:**
- `--action <action>`: `start` or `stop`.

**Usage:**
```bash
# Start recording
lux unity record-input --action start

# Stop recording
lux unity record-input --action stop
```

**Expected Output:**
JSON containing `filePath` to the recorded JSON file under `.lux/outputs/input-recordings/`.

---

## lux unity replay-input
Replays a previously recorded input sequence.

**Flags:**
- `--action <action>`: `start`, `stop`, or `status`.
- `--file <path>`: Path to the recording JSON file (required for `start`).

**Usage:**
```bash
# Start replay
lux unity replay-input --action start --file .lux/outputs/input-recordings/rec_123.json

# Check status
lux unity replay-input --action status
```

**Expected Output:**
JSON containing `active` status, `frameCount`, and `replayedFrameCount`.
