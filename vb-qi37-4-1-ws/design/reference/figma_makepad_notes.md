# velvet-ballastics — Figma-ready tightened UI set

This package contains 8 aligned desktop app screens exported as both SVG and PNG.

## Import into Figma

Recommended flow:
1. Import the SVG files in `/svg` first for editable vector layers.
2. Import the PNG files in `/png` if you need exact raster previews.
3. Use the board PNG as a quick overview for design review.
4. Each screen is 1920×1080 with a consistent app shell, sidebar, top action bar, and content grid.

## Alignment fixes applied

- Consistent 32px outer window margin.
- 246px fixed sidebar.
- 78px fixed top action bar.
- 8px rhythm with 14–22px rounded cards.
- Hairline borders only, no heavy outlines.
- Fixed right inspector widths per screen.
- No overlapping panels, graphs, tables, or text blocks.
- Color only encodes semantic state.

## Makepad animation cues represented visually

- Moving packet dots along workflow edges.
- Soft active-node glow on running steps.
- Timeline scrubber and selected event marker.
- Failure path focus with restrained red outline.
- Taint and replay-safe chips for semantic overlays.
- Journal event timeline suitable for shader-driven motion.

## Suggested Makepad implementation notes

- Implement graph edges as custom draw shaders with animated dash/packet uniforms.
- Use retained node layout coordinates so replay scrubber can drive node states.
- Represent packet animation as a normalized `progress` float per edge.
- Use small semantic color tokens; keep most surfaces white/gray.
- Use shader glow only for selected/running/failure state, never for all cards.
- Keep graph, timeline, and inspector as separate components so Figma frames map cleanly to Makepad widgets.
