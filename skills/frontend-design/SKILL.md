---
name: frontend-design
description: Create distinctive, production-grade frontend interfaces with a strong visual point of view and non-generic aesthetics. Use when Codex needs to build or redesign web components, pages, landing screens, dashboards, or full frontend applications in HTML/CSS/JS, React, Vue, or similar UI stacks, especially when the user wants polished design quality instead of boilerplate UI.
---

# Frontend Design

Turn frontend requirements into working interfaces that feel deliberately designed, not auto-generated. Prefer bold, coherent visual decisions and production-ready code over safe defaults.

## Workflow

1. Inspect the existing codebase before designing. Preserve established frameworks, tokens, spacing systems, and component conventions when working inside an existing product.
2. Define the brief in one pass: product purpose, audience, technical constraints, and the single visual idea that should make the interface memorable.
3. Commit to one aesthetic direction before coding. Name it in a short phrase so the implementation stays coherent. If you need prompts, use `references/aesthetic-directions.md`.
4. Match implementation complexity to the concept. Maximalist work should include richer composition, layering, and motion; restrained work should focus on typography, spacing, rhythm, and surface detail.
5. Build real, working code. Do not stop at mockup language, pseudo-components, or placeholder interactions unless the user explicitly asked for a mockup.
6. Verify responsiveness, accessibility, and visual consistency before finishing.

## Design Rules

- Avoid generic AI aesthetics: default centered marketing layouts, interchangeable card grids, timid palettes, and overused font stacks such as Inter, Roboto, Arial, and plain system defaults unless the repo already requires them.
- Avoid the common purple-on-white gradient look unless the product already uses it.
- Prefer a distinctive display font plus a readable body font. Make typography part of the identity, not an afterthought.
- Use CSS variables or project tokens for color, spacing, radius, shadows, and motion values.
- Favor strong composition: asymmetry, overlap, controlled density, dramatic negative space, or other layout moves that support the concept.
- Add atmosphere with backgrounds, textures, borders, gradients, mesh fields, grain, or pattern work that fit the chosen direction.
- Use motion intentionally. One well-directed load sequence or hover behavior is better than scattered animation noise.
- Keep the interface usable. Preserve contrast, keyboard navigation, reduced-motion considerations, and mobile behavior.

## Implementation Guidance

### Existing projects

- Reuse existing components before creating new primitives.
- Translate the chosen aesthetic through the project's conventions instead of fighting them.
- If the product already has a design system, push expression through composition, type, color use, and motion rather than replacing the system wholesale.

### New builds

- Establish a clear visual system early: typography pair, palette, spacing rhythm, border treatment, and motion language.
- Define a signature detail the interface repeats with intent, such as a frame treatment, angled section cut, oversized numeric type, tactile shadow language, or patterned surface.

### Stack notes

- In plain HTML/CSS/JS, prefer CSS variables and CSS-driven animation first.
- In React, follow repo patterns and available libraries. Use richer motion libraries only when they already exist or the user wants that dependency.
- In utility-CSS projects, group repeated decisions into tokens, component classes, or theme layers instead of scattering one-off values.

## Done Criteria

- The result is functional code, not just styling ideas.
- The design has a clear aesthetic point of view that can be described in one sentence.
- The interface does not look interchangeable with generic AI-generated UI.
- The work respects the project's technical constraints and feels cohesive across desktop and mobile layouts.
