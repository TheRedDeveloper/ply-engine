# Text Styling Documentation

The macroquad renderer provides the `macroquad-text-styling` feature flag, which enables the use of a custom styling sytax.

## Syntax & Formatting
Styles are applied using tags enclosed in `{` and `|` and closed with `}`.
* **Format:** `{tag_param=value|Styled Text}`
* **Important:** Do not use spaces between parameters. Use **underscores** `_` as separators (e.g., `{wave_f=0.5_a=0.3|...}`) to avoid bad word breaks.
* **Stacking:** Multiple tags can be stacked. Inner tags overwrite outer tags if they conflict.
* **Escaping:** Use `\` to escape `{`, `|`, `}`, or `\` itself.

---

## Categories

**Properties**

Static attributes applied to the text block. These set specific values like **color** or **opacity**.

**Effects**

Effects applied **individually to each character**. These create movement or visual effects.

**Animations**

Time-based transitions (entry or exit) tracked via a unique `id`. These cause text to appear or disappear.

---

## Processing Order & Priority
When multiple tags are active, they are processed in this strict order:

1.  **Hide** (Effect)
2.  **Type** (Animation)
3.  **Fade** (Animation)
4.  **Scale** (Animation)
5.  **Transform** (Effect)
6.  **Wave** (Effect)
7.  **Pulse** (Effect)
8.  **Swing** (Effect)
9.  **Jitter** (Effect)
10. **Gradient** (Effect)
11. **Opacity** (Property)
12. **Color** (Property)
13. **Shadow** (Effect)

---

## Properties Reference

**color**

Sets the fill color of the text.
* **Parameter:** `color=VALUE` (Hex code `#RRGGBB`, `(r,g,b)`, or named color)
* **Supported Names:** White, Black, LightGray, DarkGray, Red, Orange, Yellow, Lime, Green, Cyan, LightBlue, Blue, Purple, Magenta, Brown, Pink. Case insensitive.
Example: `{color=red|Red Text}`

**opacity**

Sets the transparency of the text.
* **Parameter:** `opacity=FLOAT` (0.0 to 1.0)
Example: `{opacity=0.5|50% Transparent Text}`

---

## Effects Reference

**transform**

Applies static geometric transformations to characters.
* `translate=X,Y`: Pixel offset ratio relative to font size. **Def:** 0,0
* `scale=X,Y`: Size multiplier. One value applies to both axes. **Def:** 1.0
* `rotate=DEGREES`: Clockwise rotation. **Def:** 0

**wave**

Vertical sine wave movement.
* `w=FLOAT`: Wavelength in characters. **Def:** 3
* `f=FLOAT` **OR** `s=FLOAT`: Frequency (bobs/sec) or Speed (chars/sec). **Def:** f=0.5
* `a=FLOAT`: Amplitude (pixel ratio to font size). **Def:** 0.3
* `p=FLOAT`: Phase offset (0-1). **Def:** 0
* `r=FLOAT`: Rotation of the movement vector in degrees. **Def:** 0

**swing**

Rotational sine wave (pendulum motion).
* `w=FLOAT`: Wavelength in characters. **Def:** 3
* `f=FLOAT` **OR** `s=FLOAT`: Frequency (swings/sec) or Speed (chars/sec). **Def:** f=0.5
* `a=FLOAT`: Amplitude in degrees. **Def:** 8
* `p=FLOAT`: Phase offset (0-1). **Def:** 0

**pulse**

Sine wave scaling (growing and shrinking).
* `w=FLOAT`: Wavelength in characters. **Def:** 2
* `f=FLOAT` **OR** `s=FLOAT`: Frequency (cycles/sec) or Speed (chars/sec). **Def:** f=0.6
* `a=FLOAT`: Amplitude size multiplier. **Def:** 0.15
* `p=FLOAT`: Phase offset (0-1). **Def:** 0

**jitter**

Randomized character offset within an ellipse.
* `radii=X,Y`: Maximum offset distance (ratio to font size). **Def:** 0.5,0.5
* `rotation=DEGREES`: Rotation of the jitter ellipse.

**shadow**

Renders a duplicate character behind the text.
* `color=COLOR`: Shadow color. **Def:** Black
* `offset=X,Y`: Pixel offset ratio. **Def:** -0.3,0.3
* `scale=X,Y`: Shadow size multiplier. **Def:** 1.0

**gradient**

Cycles colors across the text.
* `stops=POS:COLOR,POS:COLOR...`: List of anchor points. Positions are character indices. **Def:** Rainbow
* `speed=FLOAT`: Speed in chars/sec. **Def:** 1

**hide**

Prevents the text from rendering completely.
* No parameters.

---

## Animations Reference
*All animations require a unique `id` parameter.*

**type**

Typing writer effect. Characters appear sequentially, left to right.
* `MODE`: `in` or `out`.
* `id=WORD`: Unique tracking identifier.
* `speed=FLOAT`: Characters per second. **Def:** 8
* `delay=FLOAT`: Delay in seconds before starting. **Def:** 0
* `cursor=CHAR`: Cursor character to display while typing. **Def:** none
Example: `{type_in_id=foo_cursor=\||Text to type}`

**fade**

Opacity transition, left to right.
* `MODE`: `in` or `out`.
* `id=WORD`: Unique tracking identifier.
* `speed=FLOAT`: Characters per second. **Def:** 3
* `trail=FLOAT`: Length of the fade gradient in characters. **Def:** 3
* `delay=FLOAT`: Delay in seconds before starting. **Def:** 0
Example: `{fade_out_id=bar_speed=5|Fade this text out quickly}`

**scale**

Size transition (pop-in/pop-out), left to right.
* `MODE`: `in` or `out`.
* `id=WORD`: Unique tracking identifier.
* `speed=FLOAT`: Characters per second. **Def:** 3
* `trail=FLOAT`: Length of the scaling gradient in characters. **Def:** 3
* `delay=FLOAT`: Delay in seconds before starting. **Def:** 0
Example: `{scale_in_id=buzz_speed=0.1_tail=10|Characters become bigger reaaaaaaaaaaaaaally slowly}`