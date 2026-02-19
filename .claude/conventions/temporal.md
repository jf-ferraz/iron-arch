# Temporal Contamination

Comments must be written from the perspective of a reader encountering the code for the first time, with no knowledge of what came before. The code simply _is_.

**Why this matters**: Change-narrative comments are an LLM artifact — a category error. The change process is ephemeral. Humans writing comments naturally describe what code IS, not what they DID. A novel's narrator never describes the author's typing process.

## 5-Question Detection Heuristic

### 1. Does it describe an action taken rather than what exists?

| Contaminated | Clean |
|-------------|-------|
| `// Added mutex to fix race condition` | `// Mutex serializes cache access from concurrent requests` |
| `// New validation for edge case` | `// Rejects negative values (downstream assumes unsigned)` |
| `// Changed to use batch API` | `// Batch API reduces round-trips from N to 1` |

Signal words: "Added", "Replaced", "Now uses", "Changed to", "Updated", "Refactored"

### 2. Does it compare to something not in the code?

| Contaminated | Clean |
|-------------|-------|
| `// Replaces per-tag logging with summary` | `// Single summary line; per-tag logging would produce 1500+ lines` |
| `// Unlike the old approach, this is thread-safe` | `// Thread-safe: each goroutine gets independent state` |

Signal words: "Instead of", "Previously", "Replaces", "Unlike the old", "No longer"

### 3. Does it describe where to put code rather than what code does?

| Contaminated | Action |
|-------------|--------|
| `// After the SendAsync call` | Delete — diff structure encodes location |
| `// Insert before validation` | Delete — diff structure encodes location |

Always delete. Location is encoded by surrounding code, not comments.

### 4. Does it describe intent rather than behavior?

| Contaminated | Clean |
|-------------|-------|
| `// TODO: add retry logic later` | Delete, or implement now |
| `// Temporary workaround until API v2` | `// API v1 lacks filtering; client-side filter required` |

Signal words: "Will", "TODO", "Planned", "Eventually", "Temporary", "Workaround until"

### 5. Does it describe the author's choice rather than code behavior?

| Contaminated | Clean |
|-------------|-------|
| `// Deliberately using mutex over channel` | `// Mutex serializes access (single-writer pattern)` |
| `// Chose polling for reliability` | `// Polling: 30% webhook delivery failures observed` |

Signal words: "intentionally", "deliberately", "chose", "decided", "we opted"

## The Transformation

> Extract the technical justification, discard the change narrative.

1. What useful information is buried? (problem, behavior, constraint)
2. Reframe as timeless present

`"Added mutex to fix race"` → `"Mutex serializes concurrent access"`

## Catch-All

If a comment only makes sense to someone who knows the code's history, it is temporally contaminated — even if it doesn't match any question above.
