# A11Y Decisions Log (Pattern Memory)

> **Purpose:** cross-turn memory of choices between equally conformant alternatives. Keep entries indexed by pattern and reuse them consistently.

## Decisions

- **Terminal panel focus** → active title uses a `>` marker, active row uses a `▶` marker, and color reinforces both — focus remains identifiable without color. *(2026-09-18)*
- **Arabic terminal rendering** → `auto` selects logical order for VTE and visual order elsewhere, with config and environment overrides — preserves Quran marks where native BiDi is reliable while keeping fixed-cell terminals stable. *(2026-09-18)*
- **Startup motion** → first-launch animation is skippable and `reduced_motion = true` removes the timed sequence — users can avoid motion without losing loading status or content. *(2026-09-18)*
- **Transient status feedback** → non-actionable confirmations dismiss after two seconds; errors include `!`, text, and semantic danger color — feedback is redundant and does not permanently replace help. *(2026-09-18)*
