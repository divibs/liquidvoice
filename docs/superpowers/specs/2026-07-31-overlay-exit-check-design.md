# Overlay exit sequence + quality gaps

Date: 2026-07-31  
Status: approved (Approach A)

## Goal

Replace the abrupt / hard-to-see done path with a clear exit sequence, and close the remaining quality gaps.

## Overlay sequence

1. **Listening** — full liquid-glass pill (unchanged).
2. **Stop talking** — morph out into a small circle (~22px). Chrome (mic / wave / timer) fades during shrink.
3. **Processing** — on the circle, a thin circling spinner while transcription + inject run.
4. **Success** — spinner swaps to a small check mark (~0.45s hold).
5. **Exit** — circle scales to 0 and unmounts; then Rust may hide the window.

Silent / skipped takes: brief dim circle (no check) then shrink away.  
Errors: keep shake + caption; no success check; collapse after caption window.

## Backend contract

- Emit `processing` when leaving listening (already).
- Do **not** hide the overlay immediately on success.
- After inject succeeds: emit `done` (or `success`).
- After inject fails: emit `error` + `error-msg` (surface to UI).
- Frontend drives hide timing via morph complete; Rust delays `hide()` until ~1.2–1.6s after `done`, or listens for a future `overlay-ready-hide` if needed. Prefer delayed hide long enough for check + shrink (~1.4s after done).
- Silent skip: emit a distinct state or reuse `done` with no text; frontend shows no check.

## Gaps in same pass

- Settings: expose `max_recording_sec` (slider or number, honor sanitize 5–300); stop hardcoding 60 on save.
- Inject errors: overlay error path (not stderr-only).
- Silent skip: no fake check; quiet shrink exit.

## Non-goals

- No change to listening-in morph feel beyond exit path.
- No clipboard injection.
- No CI restore.
