# Prototype reference

An implementation-ready extraction of `prototype/index.html` (shell prototype 0.1, identical to the hosted artifact). The Rust shell must match these values. Line numbers (`L123`) point into `prototype/index.html`.

Where the prototype and the design spec disagree, the difference is listed in [Conflicts](#conflicts-with-the-design-spec). The project rule: the prototype wins unless a doc says otherwise, and every conflict is flagged before code depends on it.

All sizes are logical points (pt). In the browser 1 pt = 1 CSS px.

---

## 1. Canvas and scaling

| Item | Value | Source |
| --- | --- | --- |
| Logical canvas (prototype) | 393 x 852 pt | `.screen` L46, `W, H` L1130 |
| Display corner radius | 46 pt | `.screen` L46 |
| Device panel | 1080 x 2400 px, 60 Hz | Handbook "Scaling" |
| Scale on device | 2.75 (1080 / 393 = 2.748) | Handbook "Scaling" |
| Logical canvas on device | 393 x 872.7 pt (2400 / 2.75) | Design spec: "Designs scale by width; heights reflow" |
| Status area | 54 pt | `.status` L76, spec safe areas |
| Gesture area | 34 pt | Spec safe areas |
| Screen fallback background | `#16204A` | L46 |

**Height rule.** The device canvas is 20.7 pt taller than the prototype. Elements the prototype positions with `top` keep their top offset. Elements positioned with `bottom` keep their bottom offset. Full-bleed art (wallpapers, map, camera scene) uses `xMidYMid slice` against the 393 x 852 viewBox. On the device that scales by 872.7 / 852 = 1.0243 and crops about 4.8 pt from each side.

**Specular light position** (L2351): `--lx = clamp(x / W) * 100%`, `--ly = clamp(y / H) * 60%`. Defaults: `--lx: 30%`, `--ly: 0%`. On the device these come from the accelerometer (L2460): `lx = clamp((gamma + 45) / 90)`, `ly = clamp((beta - 20) / 70) * 0.6`.

---

## 2. Colour tokens

### 2.1 In-screen colours

| Role | Value | Where used |
| --- | --- | --- |
| Ink (brand) | `#16142A` | Light-surface text, `.glass-light` text, chips on |
| Ink 2 | `#1B1833` | Status bar over apps, app window text |
| Ink 3 | `#1F1B3A` | Selected timer preset, running start button |
| Violet (links, active) | `#6546F2` | `.h2 a`, `.back`, unread dot, outlines |
| Violet light | `#A58BFF` | FAB and bubble gradient start |
| Muted 1 | `#8A84A6` | Secondary text in apps |
| Muted 2 | `#7A7497` | Settings values |
| Muted 3 | `#6E6890` | Empty-state body |
| Muted 4 | `#4A4468` | Tab bar labels, seg buttons |
| Muted 5 | `#B3AEC7` | Chevrons |
| Crystal Cyan | `#8AF0F0` | Accent gradient start, shelf badge |
| Accent (prototype `--accent`) | `#7FE3F0` | Halo equaliser bars, focus ring |
| Amethyst | `#A78BFA` | Accent gradient end |
| Accent gradient | `linear-gradient(135deg, #8AF0F0, #A78BFA)` | CC tiles and buttons "on"; text on it `#14123A` |
| Success | `#34D17A` (call), toggle on `linear-gradient(135deg, #34D17A, #1FB5A0)` | Halo call, toggles |
| Privacy dot (camera) | `#3EE08F`, glow `0 0 8px #3EE08F` | L264 |
| Timer amber | `#FFB547` | Halo timer |
| Danger | `#FF4D5E` (buttons), `#FF3B4E` (text, `.red`) | End call, destructive |
| Battery ring | `#9FF2C8` | Lock-screen battery complication |
| Mist (light app surface) | `linear-gradient(180deg, #F7F7FA, #EEEEF4)` | `.appwin`, `.pview` |
| Dark app surface | `radial-gradient(120% 60% at 50% 0%, #2A2552, #0C0A18 70%)` | Camera, calculator, terminal |
| Grouped card | `rgba(255,255,255,.92)`, radius 22 | `.group` L183 |
| Row divider | `1px solid rgba(30,20,80,.07)` | L185 |
| Panel backdrop tint | `rgba(22,18,50,.35)` | `.pbg` L200 |
| Library backdrop tint | `rgba(10,8,28,.3)` | `.libbg` L730 |

White text on glass uses these opacities: 0.94 (icon labels), 0.93 (clock), 0.88 (date, card text), 0.85 (CC header), 0.72 (complication captions), 0.7 (hint), 0.62 (card time), 0.6 (widget kicker).

### 2.2 App icon gradients

Each icon is a 60 x 60 SVG square filled with `linearGradient x1=0 y1=0 x2=.35 y2=1`, then a glyph, then the highlight `radialGradient cx=.2 cy=0 r=.9` from white 0.28 to white 0 (L1232–1240).

| App | Top | Bottom |
| --- | --- | --- |
| phone | `#43E3A8` | `#0E9E86` |
| messages | `#A58BFF` | `#5B3FE0` |
| browser | `#72D4FF` | `#2A62E6` |
| settings | `#B4BACF` | `#596179` |
| calendar | `#FFFFFF` | `#E6E8F3` |
| clock | `#2A2650` | `#0D0B20` |
| weather | `#72CBFF` | `#2E78E6` |
| maps | `#8DE8C0` | `#1F9E8A` |
| gallery | `#FFB28C` | `#F0507C` |
| camera | `#5A586F` | `#1F1E30` |
| notes | `#FFD76A` | `#F29B1D` |
| files | `#80BAFF` | `#3A63EE` |
| calculator | `#3D3958` | `#16142A` |
| store | `#CB8EFF` | `#6546F2` |
| music | `#FF84AA` | `#DD2E68` |
| mail | `#56DCCB` | `#2386B5` |
| terminal | `#2A2650` | `#07060D` |

Glyphs are in `GLY` (L1250–1274). The glyph fill is `url(#gw)`, a vertical gradient from `#fff` to `#E4E8F7`, with a drop shadow `dy 1.4, stdDev 1.3, #0A0830 @ .28`.

Icon shell (L163–168): radius = `0.3 x size` (62 pt → 18.6). Shadow: `0 12px 22px -12px rgba(10,8,40,.65), 0 2px 5px -2px rgba(10,8,40,.3)`. Overlay: `radial-gradient(90% 55% at var(--lx) 0%, rgba(255,255,255,.22), transparent 60%)` plus `inset 0 1px 0 rgba(255,255,255,.45), inset 0 0 0 1px rgba(255,255,255,.10)`. Pressed: scale 0.9 over 0.12 s. Release: `transform .42s cubic-bezier(.34,1.56,.64,1)`.

---

## 3. Glass materials

The prototype has four CSS materials. All four share the `::before` sheen and `::after` rim below, plus `isolation:isolate; overflow:hidden`.

| Class | Role | Fill | Backdrop | Shadow (outer) |
| --- | --- | --- | --- | --- |
| `.clear` | Clear | `linear-gradient(165deg, rgba(255,255,255,.08), rgba(255,255,255,.01) 45%, rgba(255,255,255,.05))` | `blur(5px) saturate(190%) brightness(1.06)` | `0 10px 26px -14px rgba(4,3,18,.6)` |
| `.glass` | Regular | `linear-gradient(165deg, rgba(255,255,255,.13), rgba(255,255,255,.03) 42%, rgba(255,255,255,.07))` | `blur(16px) saturate(190%) brightness(1.04)` | `0 18px 40px -20px rgba(4,3,18,.7)` |
| `.thick` | Thick | `linear-gradient(165deg, rgba(40,38,64,.40), rgba(18,16,34,.36))` | `blur(32px) saturate(180%)` | `0 18px 40px -18px rgba(4,3,18,.7)` |
| `.glass-light` | Light sheet (tab bars, sheets inside light apps) | `linear-gradient(165deg, rgba(255,255,255,.80), rgba(255,255,255,.56))`; text `#16142A` | `blur(20px) saturate(180%)` | `0 10px 30px -14px rgba(20,16,50,.35)` |
| `.pbg` | Frosted backdrop (NC, CC, Intent, switcher) | `rgba(22,18,50,.35)` | `blur(34px) saturate(150%)` | none |
| `.libbg` | App Library backdrop | `rgba(10,8,28,.3)` | `blur(30px) saturate(160%)` | none |

CSS `blur(Npx)` is a Gaussian with standard deviation N.

**Inset rim and highlight shadows** (all inside the panel, applied in this order):

| Inset | `.clear` | `.glass` | `.thick` | `.glass-light` |
| --- | --- | --- | --- | --- |
| Top hairline `0 1px 0` | `rgba(255,255,255,.75)` | `.7` | `.4` | `#fff` |
| Bottom hairline `0 -1px 0` | `.25` | `.2` | `.1` | none |
| Left hairline `1px 0 0` | `.22` | `.22` | `.22` | none |
| Right hairline `-1px 0 0` | `.10` | `.10` | `.10` | none |
| Top glow `0 9px 16px -10px` | `.5` | `.45` | `.22` | none |
| Bottom glow `0 -12px 18px -12px` | `.32` | `.28` | `.12` | `0 -8px 14px -10px rgba(255,255,255,.9)` |
| Inner fill `0 0 16px` | `.07` | `.07` | `.07` | none |

**Specular sheen `::before`** (L66–68), drawn below the content:

- `radial-gradient(140% 100% at var(--lx) var(--ly), rgba(255,255,255,.22), rgba(255,255,255,0) 50%)`
- plus `linear-gradient(115deg, transparent 30%, rgba(255,255,255,.07) 42%, transparent 55%)`

**Rim `::after`** (L70–73): a 1 px ring (content box masked out) filled with `linear-gradient(155deg, rgba(255,255,255,.9) 0%, rgba(255,255,255,.12) 30%, rgba(255,255,255,.04) 55%, rgba(170,235,255,.35) 80%, rgba(255,200,235,.6) 100%)`. The cyan and pink stops are the "prism" dispersion hint.

**What the native renderer adds** (design spec and handbook pipeline, not in the prototype): quarter-resolution dual Kawase blur, SDF edge refraction of up to 6 px, adaptive tint with the legibility guard (4.5:1), a 0.5 px red/blue split on the outer rim, and a blur cache. The prototype comment at L1135 says: "GPU refraction lives in the native compositor".

---

## 4. Corner radii used

| Element | Radius |
| --- | --- |
| Display | 46 |
| Halo expanded | 36 |
| Dock | 36 |
| Home widgets `.hwx` | 30 |
| CC connectivity and media blocks | 30 |
| Switcher card body | 30 |
| App Library tiles | 30 |
| Tab bar | 30 (buttons 24) |
| CC sliders | 28 |
| Sheets (`.wgsheet`, `.lesheet`, `.msheet`) | 32 |
| Intent Bar | 26 |
| Notification card, CC tile | 24 |
| Grouped list card | 22 |
| Lock complications | 32 (fully round, 64 pt) |
| Search pill, NC pill | 20 |
| App icon | 0.3 x size |
| Lock round quick action | 50% (54 pt circle) |

The spec's standard radii are 12, 20, 28 and 36. The spec also asks for continuous-curvature (squircle) corners. The prototype uses circular CSS radii because browsers cannot draw squircles; the renderer should draw squircles with these radii.

---

## 5. Type

Font: Outfit (Google Fonts, 100–900) standing in for Sphatik Sans. Fraunces and Nunito are used only for the Serif and Rounded lock-clock styles. Numbers use `font-variant-numeric: tabular-nums` everywhere they tick.

| Use | Size / weight | Other |
| --- | --- | --- |
| Lock clock | 118 / 200 | line-height 1, tracking -0.03em, `rgba(255,255,255,.93)`, shadow `0 2px 30px rgba(20,16,60,.25)` |
| Home widget big number | 54 / 200 | line-height 1, tracking -0.03em |
| Page title `.atitle` | 34 / 700 | tracking -0.02em, padding 66 22 10 |
| NC day | 34 / 300 | tracking -0.02em |
| Lock date | 18 / 500 | tracking 0.01em, `rgba(255,255,255,.88)` |
| Intent Bar input | 17 | |
| Status bar | 15 / 600 | tabular |
| Search pill, CC header | 15 / 500 | |
| Notification card title / body | 14 / 600, 14 / 400 (line-height 1.35) | |
| Icon label | 11.5 / 500 | tracking 0.01em, shadow `0 1px 3px rgba(0,0,0,.28)` |
| Widget kicker | 11 / 600 | uppercase, tracking 0.08em, `rgba(255,255,255,.6)` |
| SKY wordmark | 80 / 250 | tracking 0.38em |

The spec's text styles (Display 96/96, Title 1 34/41 700, Title 2 22/28 600, Headline 17/22 600, Body 17/22 400, Callout 15/20 400, Caption 12/16 500) are what `sphatik-ui` exposes. Shell surfaces use the prototype sizes above.

---

## 6. Motion

### 6.1 Spring solver (`run()`, L1300–1314)

```
state: x (animated value, usually 0..1 progress), v (units per second)
each display frame:
  dt = min(0.034, (now - last) / 1000)            // seconds, capped at 34 ms
  repeat 4 times:                                  // semi-implicit Euler, 4 substeps
    a = -k * (x - to) - c * v                      // unit mass
    v += a * dt / 4
    x += v * dt / 4
  if |v| < 0.02 and |x - to| < 0.002: snap x = to, finish
starting a new run on the same key cancels the old one (interruptible);
v0 is carried in from the gesture
Reduce Motion: jump straight to `to`
```

| Preset | Prototype k / c | Implied response / damping ratio | Design spec |
| --- | --- | --- | --- |
| Snappy | 420 / 41 | 0.307 s / 1.00 | 0.25 s / 0.90 (k 631.7, c 45.2) |
| Smooth | 210 / 29 | 0.434 s / 1.00 | 0.40 s / 1.00 (k 246.7, c 31.4) |
| Bouncy | 190 / 18 | 0.456 s / 0.65 | 0.45 s / 0.72 (k 195.0, c 20.1) |
| Gentle | not in prototype | | 0.60 s / 1.00 (k 109.7, c 20.9) |

Conversion with mass 1: `omega = 2 * pi / response`, `k = omega^2`, `c = 2 * zeta * omega`.

**Velocity hand-off.** Gesture velocities are measured in px/ms (see 7.1) and passed into the spring on the progress scale. The multipliers are `unlock(-vy * 2)`, `closeApp(vy * 2)`, `showPanel(name, vy * 2)`, Intent `vy * 3`, and page `-vx * 1000 / W`.

**Which spring where** (prototype): unlock and lock use Smooth. App open and close use Smooth. Panels open with Bouncy and close with Smooth. The switcher uses Smooth. Library paging uses Smooth.

### 6.2 CSS-timed motion to port as springs

| Element | Prototype timing |
| --- | --- |
| Halo size | `.5s cubic-bezier(.34,1.45,.64,1)` (overshoot, so Bouncy) |
| Halo ping | scale 1.35 at 40%, 0.6 s |
| Halo content fade | 0.25 s; compact delay 0.15 s, expanded delay 0.18 s |
| Banner drop | `.55s cubic-bezier(.34,1.3,.64,1)`, from `translateY(-160%)`, auto-hide 4.2 s |
| Sheets (`.wgsheet`, `.lesheet`) | `.5s cubic-bezier(.34,1.2,.64,1)` from `translateY(110%)` |
| Pushed view | `.42s cubic-bezier(.22,1,.36,1)` from `translateX(100%)` |
| Toggle knob | `.32s cubic-bezier(.34,1.4,.64,1)`, 20 pt travel |
| Lock customise | inner `scale(.84) translateY(40px)`, `.5s cubic-bezier(.34,1.25,.64,1)` |
| Tab content in | `.34s cubic-bezier(.2,.9,.25,1)`, from opacity 0, `translateY(10px)` |
| Edit-mode wobble | ±1.4°, 0.26 s alternate (every second item −0.13 s delay, 0.3 s) |

---

## 7. Gestures (gesture manager, L2329–2457)

### 7.1 Recogniser

| Parameter | Value |
| --- | --- |
| Touch slop before classification | 8 pt (`Math.hypot(dx, dy) < 8`) |
| Velocity | EMA per move: `v = 0.7 * v + 0.3 * instantaneous`, px/ms |
| "Paused" (still) | no movement > 3 pt for > 140 ms |
| Apply rate | moves are coalesced and applied once per frame |
| Long-press (home edit, lock customise) | 520 ms |
| Long-press to lift an item for the Halo Shelf | 450 ms |
| Two-finger long-press (Glass Lens, v2) | 450 ms, fingers move < 14 px |
| Rubber band past 1.0 | `1 + (v - 1) * 0.18` |

### 7.2 Classification order (first match wins)

1. Finger on a CC slider → `slider`.
2. Edit mode → drag an icon (`icon`), else ignore.
3. Switcher open → top-edge pull (`nc`/`cc`), card flick up (`scard`), horizontal scroll (`strack`).
4. NC open and horizontal on a card → `card` (swipe to dismiss).
5. `y0 < 50`, vertical, downward, no panel open, Halo not expanded → `nc` if `x0 < W/2`, otherwise `cc`.
6. A panel is open and the drag is vertical upward → `closePanel`.
7. Horizontal, unlocked, no app, no panel → `page` (home ⇄ App Library).
8. Locked, vertical, upward → `unlock`.
9. App open, `y0 > H - 56`, vertical, upward → `home` (app close or switcher).
10. Home, `y0 > H - 56`, upward → `homeSwitch` (switcher from home).
11. Home, vertical, downward → `intent`.

### 7.3 Mapping and release rules

| Gesture | Progress while dragging | Commit on release |
| --- | --- | --- |
| unlock | `p = clamp(-dy / (0.5 H))` | `p > 0.25` or `vy < -0.45` → Smooth to 1; else Smooth to 0 |
| home (from an app) | `appT = 1 - clamp(-dy / (0.55 H))`, `k = 1 - appT` | if `k > 0.14` and (paused, or `|vy| < 0.25` and `k < 0.6`) → switcher; else if `k > 0.18` or `vy < -0.4` → close; else spring back to 1 |
| homeSwitch | home `translateY(max(dy, -120) * 0.25)`, `scale(1 - clamp(-dy/600) * 0.08)` | `-dy > 70` and there are recents → switcher |
| nc / cc | `p = rubber(dy / (0.42 H))` | `p > 0.3` or `vy > 0.4` → open (Bouncy) |
| intent | `p = rubber(dy / (0.28 H))` | `p > 0.35` or `vy > 0.4` → open |
| closePanel | `p = 1 - clamp(-dy / (0.42 H))` | `p < 0.75` or `vy < -0.4` → close |
| card (NC) | `translateX(dx)`, opacity `1 - |dx|/320` | `|dx| > 110` or `|vx| > 0.6` → fly out 420 pt in 0.25 s |
| scard (switcher) | `translateY(min(0, dy))`, opacity `1 + dy/500` | `-dy > 140` or `vy < -0.6` → remove |
| strack | scroll by `-dx` | snap to 268 pt steps, momentum `vx * 180` |
| page | `p = clamp(p0 - dx / W)` | `vx < -0.35` → 1, `vx > 0.35` → 0, else nearest |
| shelf drop | ghost follows the finger | drop zone `y < 96` and `|x - W/2| < 150` |

Starting a drag cancels every running spring for `lock`, `app`, `p_nc`, `p_cc`, `p_intent`, `sw` and `page`. The surface stops where it is and the finger takes over.

`Esc` (L2467): close panel, then collapse the Halo, then close the switcher, then close the app.

---

## 8. Lock screen (L113–134, L989–1013, L1326–1337)

| Element | Frame / style |
| --- | --- |
| Date | top 84, full width centred; 18/500; "Sunday, 28 September" |
| Clock | top 104, centred row, gap 10; hours unpadded 12-hour, minutes padded |
| Colon | two 10 x 10 squares rotated 45° (radius 1), column gap 26, padding-top 6 |
| Complications row | top 262, left and right 24, centred, gap 12: weather round 64 x 64 (r 32, "31°" 19/400), battery round 64 x 64 with a ring, event capsule 178 x 64 (r 32, padding 0 22, title 15/600, caption 12/500 `rgba(255,255,255,.72)`), all Regular glass. Row width 330, so x starts at 31.5 |
| Battery ring | SVG at inset 5 (54 x 54), `r = 24`, stroke 3, track `rgba(255,255,255,.18)`, fill `#9FF2C8`, dasharray 150.8, offset `150.8 * (1 - level)`, starting at −90° |
| Notification pill | container left and right 18, bottom 132, column gap 8; pill Clear glass, padding 9 x 18, r 20, 14/500, "5 notifications"; tap expands to a stack of 3 locked cards ("Unlock to view") |
| Quick actions | 54 x 54 circles, Clear glass, bottom 46, left 40 (torch) and right 40 (camera); on = `rgba(255,255,255,.9)` with `#1B1833` icon |
| Hint | bottom 28, 13/500 `rgba(255,255,255,.7)`, "Swipe up to open" |
| Depth crystal (crystal wallpapers) | a polygon `196,186 268,420 212,690 142,440` filled `url(#f1)`, stroke white 0.32 / 0.7, plus a line `196,186 → 212,690`, drawn above the clock |

**Unlock animation, progress p from 0 to 1:**

- Lock layer: `translateY(-p * H * 0.55)`, inner opacity `clamp(1 - 1.5p)`, hidden at `p ≥ 0.999`.
- Home: opacity factor `--hf = clamp(1.2p)` and `scale(0.9 + 0.1p)`, transform-origin `50% 60%`.
- Wallpaper crystal tip (`#tip`, `#tipline`): opacity `clamp(1.5p)`. The depth copy on the lock layer leaves with it.

Customisation (L2637–2676): clock font Sphatik / Serif / Rounded; weight 100–800 in steps of 50 (default 200); colours `#FFFFFF #BFF6FF #FFC6E6 #FFE3A0 #B8FFD9 #D6C6FF`; finish Solid (colour at 0.95) or Glass (fill at 0.16, 1.3 px stroke at 0.9, glow `0 0 30px` at 0.35); widget toggles; wallpaper. Persisted.

---

## 9. Home (L136–174, L2553–2596)

| Element | Frame / style |
| --- | --- |
| Widgets `.hw2` | top 66, left and right 18, 2 columns, gap 14, so each is 171.5 x 162; r 30, padding 16 16 15, Regular glass |
| Weather widget | kicker; big "31°" 54/200 at margin-top 10; sun disc 30 x 30 at right 16, top 42, `radial-gradient(circle at 35% 35%, #FFF3C4, #FFC24D 60%, #FF9F43)`, glow `0 0 24px rgba(255,196,77,.55)`; condition 14/500 at the bottom; H/L 12 at `.6` |
| Up next widget | kicker; two events, each a 3 pt bar (`#8AF0F0→#6FB8FF`, `#D6C6FF→#A78BFA`), title 14/600, time 12 at `.6` |
| Icon grid | top 246, left and right 16 (361 wide), 4 columns of 90.25, row gap 18; cell = icon 62 + gap 7 + label (about 14.5 line height) |
| Default grid order | calendar, clock, weather, maps / gallery, camera, notes, files / calculator, store, music, mail |
| Page dots | top 612, 6 x 6, gap 7, `rgba(255,255,255,.4)`, current `#fff` |
| Search pill | top 650, centred, height 40, padding 0 20 0 14, r 20, Clear glass, gap 8, 15/500, search icon 22 + "Search or ask" |
| Dock | left and right 16, bottom 26, height 94 (y 732–826), r 36, Regular glass, `space-around`, padding 0 8; phone, messages, browser, settings |
| Edit bar | top 648, centred, gap 12; buttons height 42, padding 0 20, r 21, 15/600, Regular glass ("＋ Add widget", "Done") |
| Remove badge | 24 x 24 circle at top −7, left 4; `rgba(30,28,48,.78)` with blur 8, ring `rgba(255,255,255,.28)` |

App Library (L728–743, L2598–2635): slides in with `translateX((1 - p) * 100%)` while home content moves `translate3d(-p * 60, 0, 0)` and fades. Padding 62 18 120, search 44 tall (r 22, Regular glass), grid 2 columns (gap 18 x 14), tiles r 30 padding 14 laid out 2 x 2 (gap 12), cluster 62 x 62 of four 28 pt icons (gap 6, r 8.5), category name 12/600.

---

## 10. Status bar and Halo

Status bar (L76–80, L1069–1079): height 54, padding 0 30 0 34, 15/600 tabular, white; `#1B1833` when a light app is open. Right side, gap 6: privacy dot 7 x 7, Focus moon 15, airplane 15, signal (17 x 12, four bars 3 wide, heights 4 / 6.5 / 9 / 12), Wi-Fi 16, battery (26 x 12, body 22 x 11 r 3.5 at 0.45 stroke, fill 16 x 8 r 2, cap 2 x 4).

Home handle (L247): bottom 8, 136 x 5, r 3, `rgba(255,255,255,.85)`; over a light app `rgba(20,16,50,.8)`.

### Halo (L84–111, L255–261, L683–690, L1436–1468)

| State | Size (w x h) | Radius | Top | Lens centre y (from Halo top) |
| --- | --- | --- | --- | --- |
| Idle | 28 x 28 | 14 | 11 | 14 |
| Compact | 170 x 34 | 17 | 11 | 17 |
| Expanded | 369 x 188 | 36 | 11 | 18 |

- Horizontally centred. Fill `#000`, outline `0 0 0 1px rgba(255,255,255,.06)`.
- Lens: 10 x 10 circle, `radial-gradient(circle at 35% 35%, #3b3f5c, #0c0d16 70%)`.
- Mode priority: call > timer > music. Tap idle → ping. Tap active → expand. Tap again or outside → compact.
- **Compact music:** art 20 x 20 at 8, 7 (r 6, `linear-gradient(135deg, #FFB7C9, #8C7CF0 60%, #5FC7C1)`); equaliser at right 14, top 10, 4 bars 3 wide, gap 2.5, height 14, colour `#7FE3F0`, 1 s cycle.
- **Compact timer:** 20 pt ring (r 8, stroke 2.4, `#FFB547`, dasharray 50.27); time at right 14, top 8, 14/600 `#FFB547`.
- **Compact call:** 22 pt green dot `#34D17A` with a phone glyph; duration 14/600 `#34D17A`.
- **Expanded:** padding 44 22 18. Music: 58 x 58 art (r 16), title 17/600, artist 14 at `.6`, progress bar 4 tall, controls gap 44. Timer and call: label 14 at `.6`, big 52/250 in the mode colour, two pills (flex 1, padding 10, r 18, `rgba(255,255,255,.14)`, 15/600; End `#FF4D5E`; on = white with `#16142A`).
- **Shelf droplet** (Halo Shelf, a v1 signature feature): 34 x 34 black circle at top 11, `left: 50% + 22`. It hangs below at top 50, centred, while an activity is compact, and hides while expanded. Count badge 16 tall, `#8AF0F0` on `#10283A`, 10/700.
- **Drop-target glow:** ready `0 0 0 2px rgba(138,240,240,.45)`; hot scale 1.25 with `0 0 0 3px rgba(138,240,240,.9), 0 0 28px rgba(138,240,240,.7)`.

Banner (L262): top 8, left and right 10, Thick glass card.

---

## 11. Panels

Shared (L1395–1403):
- Backdrop `.pbg` opacity = p.
- Content `translate3d(0, (p - 1) * 60, 0) scale(0.96 + 0.04p)`, content opacity p.
- Status bar turns white over panels. Interactive when p > 0.5. A tap on the backdrop closes the panel.

### Notification Center (L202–213)

- Header: top 66, left and right 22. Day 34/300; date 15/500 at `.72`; "Clear all" Regular glass, padding 8 x 14, r 18, 13/600.
- List: top 140, left and right 14, gap 10.
- Card: Regular glass, r 24, padding 13 x 14, gap 12, icon 38; title 14/600 and time 13/500 at `.62`; body 14 at `.88`, line-height 1.35.

### Control Center (L214–229, L1027–1054)

- Header: top 70, left and right 24, 15/500 at `.85` ("82% battery" / "Swipe up to close").
- Grid: top 112, left and right 22 (349 wide), 4 columns of 78.25, rows 78, gap 12.
- Column x: 22, 112.25, 202.5, 292.75. Row y: 112, 202, 292, 382, 472.

| Item | Cells | Frame (x, y, w, h) |
| --- | --- | --- |
| Connectivity (Wi-Fi, data, Bluetooth, airplane), Regular glass r 30, padding 10, 2 x 2 buttons 58 x 58 (`rgba(255,255,255,.14)`) | cols 1–2, rows 1–2 | 22, 112, 168.5, 168 |
| Media, Regular glass r 30, padding 16, art 48 (r 14), play 42 on white `.9` | cols 3–4, rows 1–2 | 202.5, 112, 168.5, 168 |
| Brightness slider, r 28 | col 1, rows 3–4 | 22, 292, 78.25, 168 |
| Volume slider, r 28 | col 2, rows 3–4 | 112.25, 292, 78.25, 168 |
| Torch, Focus | cols 3, 4, row 3 | 202.5 / 292.75, 292, 78.25, 78 |
| Hotspot, Screen record | cols 3, 4, row 4 | 202.5 / 292.75, 382, 78.25, 78 |
| Nearby, Lens, Desktop (v2, not built in v1) | cols 1–3, row 5 | y 472 |

- Slider fill: `linear-gradient(180deg, rgba(255,255,255,.95), rgba(235,232,255,.85))`, rising from the bottom; radius `0 0 28 28`, becoming 28 all round above 94%. Icon at bottom 16 in `#1B1833`. Brightness is floored at 0.15 and dims the screen by `(1 - b) * 0.6` black.
- On state for tiles and buttons: the accent gradient with `#14123A`. Tile radius 24.

### Intent Bar (L230–244)

- Bar: top 62, left and right 16, height 52, r 26, Regular glass, padding 0 16, gap 10; search icon, 17 pt input ("Search or ask"), mic icon.
- Results: from top 130 to bottom 40.

### App switcher (L336–348, L2263–2292)

- Backdrop `.pbg`. Track: top 124, height 600, side padding 71.5, gap 18. Card 250 wide: head 42 (icon 28, 15/600), body 250 x 542, r 30, content scaled 0.6361.
- Open progress: track `scale(0.86 + 0.14p)`, opacity p. "Clear all" at bottom 52.

---

## 12. App window open and close (L2225–2259)

Given the source rectangle `r` (the tapped icon, radius 19) and progress `t` (clamped to 1.02 for position, 1 for radius):

```
sc   = max(0.01, lerp(r.w / W, 1, t))
x, y = lerp(r.x, 0, t), lerp(r.y, 0, t)                 // window scaled from its top-left
vis  = lerp(r.h, H, t) / sc                             // visible height (bottom clip)
rad  = lerp(r.rad or 19, 46, t) / sc
content opacity = clamp((t - 0.3) / 0.45)               // 0 when opening from the switcher
icon overlay opacity = clamp(1 - 2.2 t), centred in the visible area, scaled r.w / 62 / sc
home: scale(1 - 0.07 t), --hf = 1 - 0.55 t
status bar and handle switch to dark once t > 0.6 (light apps)
```

Closing re-targets to the app's icon on the current page, or to a 62 pt square at the screen centre if the icon isn't visible.

---

## 13. Wallpapers (L2475–2551)

Vector art on a 393 x 852 viewBox, `preserveAspectRatio="xMidYMid slice"`. Gradient coordinates are SVG defaults (`objectBoundingBox`): every ribbon gradient spans that ribbon's own bounding box. The Rust port must reproduce this.

### 13.1 Ribbon type: Liquid Aurora (default) and Obsidian

| | Liquid Aurora | Obsidian (`flip`: mirrored with `translate(393 0) scale(-1 1)`) |
| --- | --- | --- |
| `bg` (0, 0.6, 1) | `#0B0A1C #06050E #15113A` | `#0C0C12 #040406 #15151D` |
| `rib[0]` | `#231C6E #3B2C9E #5B47D6` | `#0E0E14 #1C1C26 #2A2A36` |
| `rib[1]` | `#6D5BFF #43C6F5 #A6F4E8` | `#1A1A24 #55556A #22222C` |
| `rib[2]` | `#FF6FA8 #FFA982 #FFD9A0` | `#24242F #9A9AB2 #2C2C38` |
| `rib[3]` | `#8BE9FF #C2B2FF #FFFFFF` | `#3A3A4A #E4E4F0 #5A5A6A` |
| `hl` | `#FFFFFF #BFF6FF #FFC6E6` | `#FFFFFF #9FF0F5 #D6C6FF` |

Ribbon paths (`RIBBONS`, L2488):

```
0: M-40 900 L-40 830 C 110 730, 250 905, 440 700 L 440 900 Z
1: M-40 720 C 90 580, 215 840, 440 540 L 440 640 C 235 930, 80 690, -40 820 Z      top: M-40 720 C 90 580, 215 840, 440 540
2: M-40 575 C 120 480, 195 715, 440 415 L 440 470 C 215 780, 110 555, -40 640 Z    top: M-40 575 C 120 480, 195 715, 440 415
3: M-40 485 C 140 395, 205 615, 440 318 L 440 336 C 220 640, 130 432, -40 510 Z    top: M-40 485 C 140 395, 205 615, 440 318
```

Gradients:

- `bg`: linear (0, 0) → (0.3, 1), stops at 0, 0.6 and 1.
- `rg_i`: linear (0, 0.2) → (1, 0), stops `rib[i]` at 0, 0.55 and 1.
- `sh` (satin shade): vertical, `#fff@.30` at 0, `#fff@0` at 0.3, `#000@0` at 0.6, `#000@.5` at 1.
- `hl` (mirror edge): horizontal, `hl0@0` at 0, `hl0@.9` at 0.35, `hl1@.8` at 0.65, `hl2@.2` at 1.

Paint order:

1. Rect filled with `bg`.
2. Ellipse at (200, 700), rx 280, ry 220, filled `rib[1][0]` at opacity 0.35, Gaussian σ 28.
3. Group at opacity 0.55, Gaussian σ 28: ribbon 1 in `rg1`, ribbon 2 in `rg2` (the glow).
4. For i = 0..3: ribbon i in `rg_i`, then ribbon i again in `sh`, then for i ≥ 1 its `top` edge stroked in `hl` at width 1.4 (1.0 for i = 3), opacity 0.85.
5. Time-of-day tint rect (13.3).
6. Grain: `feTurbulence fractalNoise`, baseFrequency 0.9, 2 octaves, desaturated, opacity 0.05, `mix-blend-mode: overlay`.

Steps 2–4 sit inside the parallax group.

### 13.2 Crystal type: Quartz Dawn and Amethyst

| | Quartz Dawn | Amethyst |
| --- | --- | --- |
| `sky` (0, 0.4, 0.76, 1) | `#0F1640 #34307A #A8739E #F4C2B0` | `#06061A #1B1450 #4A2A8C #A064CF` |
| `b` | `#4FD8D0 #FF8FB7 #8B6CFF` | `#6C4BFF #FF6FB5 #3FC8FF` |

Paint order (L2509–2538):

1. `sky` linear (0, 0) → (0.25, 1).
2. Parallax group:
   - Group at opacity 0.8, Gaussian σ 42: ellipses (40, 250, 160 x 120, `b0` @ .7), (380, 590, 150 x 180, `b1` @ .6), (210, 430, 120 x 100, `b2` @ .55).
   - Inner glow ellipse (206, 420, 110 x 190), radial (0.5, 0.35, r 0.5) from white 0.5 to 0.
   - Facets, stroke white 0.34, width 0.7, round joins:
     - `150,440 142,442 86,560 118,410` in f2
     - `142,440 212,690 136,770 88,562` in f4
     - `268,420 322,560 270,735 212,690` in f3
     - `268,420 306,352 342,520 322,560` in f2
     - tip `196,186 268,420 212,690 142,440` in f1
   - Tip line `196,186 → 212,690`, white 0.5, width 0.6.
   - Sparkles: 4-point star path `M0-6 1.1-1.1 6 0 1.1 1.1 0 6-1.1 1.1-6 0-1.1-1.1Z` at (268, 420, scale 0.9, opacity 0.9), (142, 440, .6, .7), (306, 352, .5, .7), (70, 150, .45, .5), (330, 250, .35, .45), (212, 690, .7, .8).
   - Battery glow ellipse (205, 440, 70 x 150), white 0.22, screen blend.
3. Tint rect.
4. Grain at 0.06.

Facet gradients:

- f1 (0, 0) → (1, 1): white .6, `b0`@.22 at 0.5, white .06.
- f2 (1, 0) → (0, 1): `b2`@.45, `sky1`@.12.
- f3 (0, 0) → (1, 1): `b1`@.45, white .05.
- f4 (0, 1) → (1, 0): `b0`@.4, white .1.

### 13.3 Living wallpaper

- **Parallax:** `translate(-nx * 14, -ny * 10)`, with nx and ny in −0.5..0.5 from pointer or tilt. Applied to the wallpaper group and the lock depth crystal.
- **Time tint:** 05–08 `#FFB38A` @ 0.14, 08–17 `#FFF3D6` @ 0.05, 17–20 `#FF8A5C` @ 0.16, otherwise `#0A1240` @ 0.26.
- **Update policy:** motion updates only when the phone moves or the time changes; nothing animates constantly (spec).

---

## 14. SKY boot splash (L928–945, L3061–3073)

- Black background. "SKY" at 80/250, tracking 0.38em (padding-left 0.38em keeps it optically centred), centred on screen.
- Fill: `linear-gradient(100deg, #8E94B6 0%, #fff 36%, #A6F2F6 46%, #D6C6FF 53%, #fff 62%, #8E94B6 100%)`, background-size 280%, clipped to the text.
- `skyin`: 1.2 s `cubic-bezier(.2,.8,.2,1)`, delay 0.25 s, from opacity 0, blur 12, tracking 0.9em to opacity 1, blur 0, tracking 0.38em.
- `shimmer`: 1.7 s ease-in-out, delay 1.1 s, background-position 100% → 0%.
- Footer "SPHATIK OS": bottom 70, 14/400, tracking 0.24em, `rgba(255,255,255,.5)`, fades in over 1 s after 0.9 s.
- Hand-off: the splash fades out over 0.8 s at t = 3.6 s (0.9 s under Reduce Motion), revealing the lock screen.
- The `.bd` diamond is defined in CSS but not placed. The brand rules say no symbol, so don't draw it.

---

## 15. Shared app components (light apps)

| Component | Key values |
| --- | --- |
| Page | title 34/700 at padding 66 22 10; scroll area from top 120, padding 0 16 130 |
| Floating tab bar | `.glass-light`, centred, bottom 34, padding 5, r 30, gap 2; buttons padding 10 x 16, r 24, 14/500 `#4A4468`; on = `rgba(167,139,250,.24)` with `#15123A` 600 |
| Grouped card and row | `rgba(255,255,255,.92)` r 22; row min-height 56, padding 10 x 14, gap 12; title 16/600, subtitle 13 `#8A84A6` |
| Search field | height 38, r 12, `rgba(118,118,140,.12)`, 16 pt |
| Segmented control | track `rgba(118,118,140,.12)` r 10 padding 2; buttons 13/600, on = white with a shadow |
| Toggle | 51 x 31 r 16, off `rgba(120,120,140,.28)`, knob 27, on = green gradient |
| FAB | 58 x 58 r 29, right 20, bottom 104, `linear-gradient(135deg, #A58BFF, #6546F2)`, shadow `0 14px 26px -10px rgba(101,70,242,.7)` |
| Pushed view nav bar | height 100 (padding-top 52), `rgba(247,247,250,.8)` with blur 20; back in `#6546F2` 16/500 |
| Toast | Thick glass, bottom 118, padding 10 x 18, r 20, 14/500 |

The 16 stock apps and their data live in the `UI` registry (L1686–2026) and are Stage 5–6 work. Consult them directly when those work packages start.

---

## 16. Out of scope until after the public beta

These appear in the prototype but are v2 features per the build plan (rule 5). Don't port them: Desktop mode (L2751), Glass Lens (L2840), Offline Nearby (L2918), Private Memory (L2942), decoy data and the domain firewall (L2965 onward, per-server toggles), and Private Space. The Control Center row Nearby / Lens / Desktop goes with them.

The Terminal app, Halo Shelf, living wallpapers, Hindi for the whole phone, the tracker firewall v1 and Longevity are v1 (build plan 6.6 and 6.7).

---

## Conflicts with the design spec

Decided 2026-09-28. The owner delegated the calls, and these rules come from the docs themselves:

- **Visual layout, sizes and gesture feel: prototype.** The handbook says the prototype is "the visual reference" for every component, and the Glass on laptop gate requires swipes that "feel as good as the prototype".
- **Spring constants and which preset each transition uses: design spec.** Build plan 2.6 says the tests must match the spec values.
- **UX rules the prototype doesn't implement: design spec.** This covers the Back gesture and thumb reach (HIG principle 2).
- **Renderer: handbook** (GLES 3.2 first).

| # | Topic | Prototype | Design spec / handbook | Decision |
| --- | --- | --- | --- | --- |
| C1 | Spring constants | Snappy 420/41, Smooth 210/29, Bouncy 190/18, no Gentle | Snappy 0.90 / 0.25 s, Smooth 1.0 / 0.40 s, Bouncy 0.72 / 0.45 s, Gentle 1.0 / 0.60 s; build plan 2.6 says "unit tests match the design spec values" | **Spec.** Snappy k 631.7 c 45.2; Smooth k 246.7 c 31.4; Bouncy k 195.0 c 20.1; Gentle k 109.7 c 20.9. Keep the prototype's 34 ms frame cap, retargeting and rest thresholds. **Integration (WP 2.6):** the prototype's 4-substep Euler drifts from the true spring (about 0.02 on Snappy's first frame; Bouncy overshoots 3.3% instead of 3.8%), so `sphatik-motion` advances each frame with the exact closed-form solution, which also makes motion identical at 60 and 120 Hz |
| C2 | Glass blur | clear 5, regular 16, thick 32, frosted 34 (CSS Gaussian σ) | Blur radius clear 8, regular 24, thick 40, frosted 64 (Kawase); tint 5 / 18 / 35 / 55%; Z 3 / 2 / 4 / 1 | **Prototype look.** Choose Kawase pass count and offsets per material so the result matches a Gaussian with the prototype σ (5 / 16 / 32 / 34); verify with golden images. The spec's radii remain the names of the tiers |
| C3 | Halo compact | 170 x 34 | 36 pt tall capsule | **Prototype** (170 x 34) |
| C4 | Halo expanded | 369 x 188 | Up to 393 x 200 | **Prototype** (369 x 188, within the spec's maximum) |
| C5 | Dock | height 94, r 36, bottom 26 | 88 pt tall, radius concentric (46 − 16 = 30) | **Prototype** |
| C6 | Home icon size | 62 | 60 | **Prototype** (62) |
| C7 | App open and close spring | Smooth | Bouncy | **Spec: Bouncy** (spring assignment is a motion-system rule). By the same rule, **unlock** commits with Bouncy (spec lock-screen states: "Swipe up; Bouncy spring"); falling back to locked stays Smooth. Panels keep the prototype's Bouncy open and Smooth close (the spec assigns them no preset) |
| C8 | CC open threshold | p > 0.3 of a 0.42 H drag (about 12.6% of screen) | "past 30% of the screen height" | **Prototype** (gesture feel is the gate criterion) |
| C9 | Top-edge panel zone | `y0 < 50` | Status area 54; edge zones 20 | **Prototype** (50) |
| C10 | Back gesture | not implemented | Inward swipe from the left or right 20 pt edge | **Spec:** added in 2.7 (`sphatik_shell::gesture`): only inside an app with no panel open, horizontal, moving inward. The spec gives no commit rule, so these are **provisional**: progress = \|dx\| / (0.5 W); commit when progress > 0.35 or \|vx\| > 0.35 pt/ms |
| C11 | Control Center material and layout | Frosted backdrop + Regular-glass tiles; media 2 x 2; sliders 1 x 2 | One Thick sheet; media 4 x 2; sliders 1 x 3 | **Prototype** (revisit at 5.5) |
| C12 | App Library search | at the top | pinned at the bottom (thumb reach) | **Spec:** bottom, per HIG principle 2 "Reachable" (applies at 5.3) |
| C13 | Switcher card scale | 0.6361 | 70% | **Prototype** |
| C14 | Renderer | n/a | Spec hardware table says "glass renders via Vulkan"; handbook and build plan say GLES 3.2 first | **GLES 3.2** (handbook, build plan) |
