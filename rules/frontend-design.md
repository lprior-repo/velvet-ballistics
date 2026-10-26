# Frontend design and UI craft

Read this when building, styling, or reviewing web UI: pages, components, dashboards. The craft and accessibility numbers hold for any UI; the CSS and DOM APIs are web-only.

## Numbers agents get wrong

- Contrast: 4.5:1 body text, 3:1 large text and UI components against adjacent colors. Test dark mode separately — desaturated tonal variants, never inverted colors.
- Touch targets ≥44×44px with ≥8px gaps; extend the hit area beyond a smaller visual icon.
- Body text ≥16px (avoids iOS auto-zoom), line-height ~1.5, 65–75 characters per line.
- Spacing on a 4/8px scale; z-index from a defined scale (0/10/20/40/100); no arbitrary values.
- Motion: 150–300ms micro-interactions, ≤400ms transitions; ease-out on enter, ease-in on exit (exit ~70% of enter); stagger lists 30–50ms per item; animate `transform`/`opacity` by default (exceptions: the cost ladder under Animation performance); honor `prefers-reduced-motion`; animations interruptible, never input-blocking; no `transition: all`.
- Icons: SVG from one family with one stroke width (e.g. Lucide, Heroicons) — never emoji; don't mix filled and outline at the same hierarchy level.
- `tabular-nums` for numbers in columns (prices, timers, data).

## Interaction correctness

- When a design system exists, use its tokens and components — no hardcoded hex/pixel values, no ad-hoc restyling, which drift from the system on its next redesign; flag deviations instead of silently inventing them.
- `<button>` for actions, `<a>` for navigation — never `<div onClick>`, which skips keyboard focus and screen-reader semantics; icon-only buttons need `aria-label`; semantic HTML before ARIA.
- Never remove focus outline without a `:focus-visible` replacement — keyboard users lose all sight of where focus is.
- No `tabindex` greater than 0, which breaks the page's natural tab order.
- Core task completable in ≤3 interactions; one primary action per view — no competing primary buttons.
- Dialogs: trap focus inside, set initial focus, Escape closes, focus returns to the trigger on close.
- Link field errors to inputs with `aria-describedby` + `aria-invalid`; toasts are never the only notification (pair with `aria-live`).
- `dvh` over `vh` for full-height layouts — `vh` ignores the mobile browser's address bar and clips content.
- Forms: never block paste; correct `type`/`inputmode` per field; labels clickable; validate on blur, not keystroke; errors inline naming the fix, focus the first invalid field; submit stays enabled until the request starts, then shows progress; warn before navigating away with unsaved changes.
- Destructive actions need confirm or undo — never immediate.
- URL reflects state: filters, tabs, pagination deep-linkable; back restores scroll and state.
- Content resilience: design for short, average, and very long content; handle overflow with the CSS properties (`text-overflow`, `line-clamp`, `overflow-wrap`); flex children need `min-width: 0`; handle empty states.
- Images get explicit width/height; reserve layout space for any injected UI (banners, consent bars, embeds) — nothing shifts content when it loads.
- Lazy-load below the fold and virtualize lists >50 items, so content off screen costs no network or layout until it is needed.
- Never trigger browser permission prompts (notifications, geolocation) on page load — request after a user action that shows the value.
- Locale: `Intl.DateTimeFormat`/`Intl.NumberFormat`, never hand-formatted dates/numbers, which bakes in one locale's conventions.
- Copy: active voice; specific button labels ("Save API key", not "Continue"); error messages state the fix.
- A persisted preference (theme, locale, density) is applied before the first paint, through a blocking inline read in the document head or a server render from a cookie. Applied after the app starts, it paints the default first and flashes to the real value; a transition on the themed properties animates the same flash.
- When a feature is explorable before signup and only saving is gated, keep what the visitor built (draft, input, selections) across the signup and attach it to the new account; a signup that discards it turns the wall into the exit.
- Back the numeric rules with an automated accessibility check in the test suite so they hold on the next edit.
- Automated rules catch only part of the issues, so keyboard and screen-reader passes stay manual.
- Flag on sight: `user-scalable=no`, `maximum-scale=1`, paste blocking, unlabeled icon buttons, images without dimensions, `outline: none`.

## Microcopy

Applies to every user-visible string a product ships: page copy, headings, button labels, `title`/`description`, `alt` text, and the plain-text twin of any structured data. Prose documentation has its own register in `rules/markdown.md`.

- Digits for anything countable and checkable — items, permissions, steps, seconds, clicks, storefronts, years, questions — including 0 through 9 and at the start of a sentence. Digits survive scanning; a spelled number dissolves into the words around it ([NN/g eyetracking](https://www.nngroup.com/articles/web-writing-show-numbers-as-numerals/)).
- Words stay for `one` as a pronoun or idiom ("one person, one vote", "which one", "one-time code"), for a bare pronoun pair ("the two agree"), and for vague scale ("a thousand throwaway profiles").
- Never mix the two forms in one sentence. "600 emoji, not six defaults" is the bug: recast the sentence, or keep both as words when a rhetorical `one` sits next to the count.
- Thousands take a comma (`50,000`), a compound modifier takes a hyphen (`6-digit code`), comparison tables always take digits, and a count that also lives in data is interpolated from it rather than typed — the hardcoded copy is the one that goes stale.
- One pair of em-dashes per sentence, never two: an aside inside an aside is where a sentence stops being readable. Recast the second — a colon when a list follows a noun, parentheses for a short qualifier, its own sentence when the aside carries a verb. Adjacent table cells, list items and a `title`/`description` pair are separate sentences, each with its own budget.
- One term per concept, and the internal name is never the user-facing one. Pick the word the interface itself shows the user and use it everywhere; where a product surface quotes a control's exact label, quote it verbatim and mark it as a label rather than paraphrasing. A field name, a feature flag or a class name leaking into copy reads as an implementation detail because it is one.
- Both halves of a string pair change together: the visible HTML and the plain text that feeds structured data, the label and its `aria-label`, the heading and the `title`. Editing one leaves the other contradicting it in the exact place nobody rereads.

## Animation performance

- Cost ladder: composite (`transform`/`opacity`) < paint (color/shadow/filter) < layout (size/position). Pick the cheapest that matches the intent; animate paint/layout only on small isolated surfaces.
- Never interleave layout reads and writes in the same frame: measure once, then animate via transform (FLIP); batch reads before writes.
- Don't drive animation from scroll events or `scrollTop` — use Scroll/View Timelines or IntersectionObserver; pause animations off-screen; no rAF loops without a stop condition.
- `will-change` is temporary and surgical — never applied outside an active animation.
- Animate `transform` and `opacity` instead of CSS variables: a changed custom property restyles every element that reads it, on every frame.
- Blur ≤8px, one-shot, never continuous or on large surfaces.
- View transitions for navigation-level changes only: each one snapshots and animates the page, too heavy for an in-page toggle.
- Never migrate or mix animation libraries unless asked (core Boundaries rule).

## Review output

When reviewing UI against these rules, use the comment format from `rules/code-review.md` (`file:line: <severity>: problem. fix.`); "✓ pass" when clean. Each finding names the violated rule and proof it applies to that surface; try to falsify each before reporting; order by impact.
