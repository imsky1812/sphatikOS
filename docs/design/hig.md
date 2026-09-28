# Sphatik Human Interface Guidelines

Rules for anyone designing or building a Sphatik app. The design specification explains the system itself; this page is what an app must do to feel native.

## 1. Principles

1. **Content first.** Controls sit on glass above content, never on opaque bars.
2. **Reachable.** Primary actions live in the bottom 60% of the screen.
3. **Physical.** Everything moves on springs and can be interrupted.
4. **Calm.** No badges shouting for attention; notifications are for things the user wants to know now.
5. **Honest.** Ask permission at the moment of need and say why.

## 2. Layout

| Rule | Value |
| --- | --- |
| Base unit | 4 pt; spacing from 4, 8, 12, 16, 20, 24, 32, 40, 56, 72 |
| Side margins | 16 pt on phones |
| Minimum touch target | 44 x 44 pt, 8 pt apart |
| Safe areas | 54 pt status area at the top, 34 pt gesture area at the bottom, 20 pt edge zones for Back |
| Body text line length | 40 to 75 characters |

**Standard page:** large title (34 pt, collapses to a glass title bar on scroll), content scrolling under all glass, floating glass tab bar with 2 to 5 tabs, optional floating action button above it on the right.

## 3. Glass materials

| Material | Use it for |
| --- | --- |
| Clear | Buttons over photos and video, floating toolbars |
| Regular | Tab bars, cards, widgets |
| Thick | Sheets, menus, dialogs, keyboard |
| Frosted | Backdrops behind modal content |

Never stack more than two glass layers. Never put body text on Clear glass over busy content; use Regular or Thick.

## 4. Typography

Use the text styles, not raw sizes: Display 96, Title 1 34/700, Title 2 22/600, Headline 17/600, Body 17/400, Callout 15/400, Caption 12/500. All styles scale with the system text size (80% to 310%); layouts must reflow, not truncate.

## 5. Colour

Use semantic tokens only: `surface`, `on-surface`, `accent`, `on-accent`, `success`, `warning`, `danger`, `glass-tint`, `glass-rim`. Never hard-code colours; tokens adapt to the wallpaper and to Dark mode. Text contrast must be at least 4.5:1.

## 6. Motion

| Preset | Use |
| --- | --- |
| Snappy | Toggles, small controls |
| Smooth | Sheets, page pushes, cards |
| Bouncy | App open and close, Halo, playful moments (sparingly) |
| Gentle | Large background changes |

Pushed pages slide in from the right; sheets rise from the bottom; nothing fades in from nowhere. Under Reduce Motion, replace movement with cross-fades.

## 7. Navigation and gestures

- Do not override the system edge gestures (home, back from either edge, panels).
- Back always pops one level. Tab bars switch sections, they never push.
- Apps that truly need edge swipes (drawing, games) may request the one-time confirm mode.

## 8. Components

Prefer system components: title bar, tab bar, search field, segmented control, toggle, slider, list row, grouped card, avatar, floating action button, bottom sheet, menu, toast, banner, text field, date picker, empty state. A custom control must match their size, contrast and accessibility.

## 9. Notifications and live activities

- Declare channels with honest importance. Marketing is never Urgent.
- A live activity is for something happening right now with a clear end (a call, a ride, a timer, a delivery). End it as soon as it is over.
- Update at most once per second.

## 10. Permissions

Ask at the moment of use, with one sentence of reason. If the user declines, keep the app usable and offer the feature again later only when they try it.

## 11. Accessibility checklist

- [ ] Every control has a label and role in the accessibility tree.
- [ ] Works at 310% text size.
- [ ] Works with Reduce Transparency (glass becomes solid).
- [ ] Works with Reduce Motion.
- [ ] Colour is never the only signal.
- [ ] All actions reachable with the screen reader and with 3-button navigation.

## 12. Writing

Short, plain, friendly. Sentence case for everything. Say what happened and what to do: "Couldn't send. Check your connection and try again." Never blame the user. Hindi and English strings are both first-class; leave room for 40% longer text.
