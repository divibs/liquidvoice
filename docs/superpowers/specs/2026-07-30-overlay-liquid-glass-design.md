# LiquidVoice Overlay UI - Design Spec

**Date:** 2026-07-30  
**Status:** Locked (user-approved) · implemented in `Capsule.svelte`  
**Scope:** Replace the existing SVG goo `Capsule.svelte` overlay with the approved CSS liquid-glass pill.

## Goal

When the hotkey triggers listening, show a small frosted-glass pill that morphs in quickly with a startup bounce, displays mic / waveform / timer / red status, then morphs out on done. Glass must read as dark translucent material (no milky white fill, no hard white outline, no fake top specular).

## Locked decisions

| Decision | Choice |
|---|---|
| Style | Dark frost glass (single surface) |
| Morph | Elastic stretch, fast (~650ms open / ~380ms close), startup spring bounce |
| Size | Short pill ~168×34 CSS px at rest |
| Material | `backdrop-filter: blur(16px) saturate(185%) brightness(0.92)` + `rgba(6,6,12,0.45)` tint |
| Outline | None - no border stroke |
| Top specular | None |
| Status dot | Red (`#ef4444` family), not purple |
| Clip | Always `border-radius: 999px`; morph via width/height (+ slight mid-stretch height squash). No peanut `%` radii |

## Visual structure

```
[ mic well ] [ tapered waveform bars ] | [ mm:ss ] [ red · ]
```

- Mic: recessed circular well, white icon
- Waveform: ~10 bars, live mic level, white
- Timer: tabular nums, white
- Status: small pulsing red disc

## Animation

1. **Appear:** scale spring kick (~+22% decaying underdamped), width/height expand
2. **Stretch:** elastic width grow; slight vertical squash mid-way for liquid feel
3. **Settle:** soft end bounce; chrome fades in ~0.38-0.72 of timeline
4. **Collapse:** reverse on `target=0`, then call `onCollapsed`

## Implementation notes (existing app)

- Replace `src/components/Capsule.svelte` (SVG goo filter approach) with CSS/HTML pill matching this spec
- Keep props: `level`, `target`, `elapsed`, `mode`, `onCollapsed`
- Error mode: can tint rim/glow rose; status stays red while listening
- Overlay window is transparent; WebView **cannot** blur the desktop (`backdrop-filter` only sees in-page pixels). Capsule uses layered tint + film grain so frost still reads; `backdrop-filter` helps in `npm run dev` over colorful stage.
- Never use `isolation: isolate` on the frost surface (disables blur).
- Dev preview (`npm run dev`) should still auto-play morph with fake mic

## Out of scope

- Settings window redesign
- Hotkey / state machine changes
- New dependencies

## Acceptance

- [ ] Morph feels fast with startup bounce
- [ ] Settled pill ~168px wide, dark glass, no white outline/specular
- [ ] Red status dot near timer
- [ ] No rectangular tint box leaking outside the capsule
- [ ] Works in Tauri overlay and frontend-only preview
