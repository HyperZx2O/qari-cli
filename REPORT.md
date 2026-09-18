# A11y Verification Report

## Validation Context

- **Feature/Epic:** qari-cli terminal reader audit remediation
- **Test Date:** 2026-09-18
- **Covers interface as of:** working tree after Hallmark and Impeccable remediation
- **Compliance Status:** ⚠️ CONDITIONAL — terminal assistive-technology validation remains manual

## 1. Technical Verification

- [x] **Rust quality gates:** `cargo test --all-targets` and strict Clippy pass.
- [x] **Terminal semantics:** Every action is keyboard-operated; no pointer-only behavior exists.
- [~] **Automated accessibility scanner:** The bundled detector returned no findings but does not parse Ratatui; manual source and buffer tests were used instead.

## 2. Focus and Keyboard

- [x] **Focus indicator:** Active panels use text markers plus contrast-safe color.
- [x] **Logical navigation:** Vim and arrow bindings cover panels, lists, scrolling, search, help, and overlays.
- [x] **Overlays:** Search and help close with Escape; help remains scrollable on small terminals.

## 3. Behavior and Task Completion

- [ ] **Screen reader test:** Requires human validation with a terminal/screen-reader pair such as NVDA + Windows Terminal.
- [x] **Status changes:** Loading, offline, success, and error states have visible text; errors also include a `!` marker.
- [x] **Long content:** Buffer tests verify long Scripture scrolls through metadata without clipping the task.

## 4. Visual Perception

- [x] **Contrast:** Automated theme tests enforce 4.5:1 text and 3:1 meaningful-border floors.
- [x] **Redundancy:** Panel focus and errors use marker + text + color.
- [x] **Reflow:** TestBackend coverage verifies three-panel wide mode and one-panel compact mode.

## 5. Motion

- [x] **Control:** Intro motion is first-launch-only, skippable with any key, and bypassed by `reduced_motion = true`.
- [x] **Flashing:** No content flashes more than three times per second.
- [ ] **Human motion review:** A motion-sensitive user should validate the animated and reduced-motion startup paths.

## 6. Cognitive Load

- [x] **Help consistency:** `?` remains available in the footer and Escape closes overlays.
- [x] **Timing:** The only timed content is non-actionable status feedback; actionable content does not auto-dismiss.
- [x] **Conflicting needs:** RTL fidelity and fixed-cell stability are recorded in `A11Y-DECISIONS.md`.

## Known Blockers

- Human screen-reader and motion-sensitivity validation are still required before claiming certification readiness.
