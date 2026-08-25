# Canvas and Visual Core

Status: **experimental Canvas v0** in the `v1.2` line.

Canvas v0 is a small immediate-mode 2D layer:

```text
Canvas draws into a software framebuffer; a platform Window presents the frame.
```

It is not a UI framework or a game engine. The current slice validates the
syntax, resource model, basic rasterizer, and separation between portable
drawing code and the platform backend.

## Quick example

```skadi
Canvas frame = canvas(320, 200)
Window window = windows.open("Skadi Canvas", 320, 200)

new Color background = color_hex("#1d1f21", 255)
new Rect panel = rect(24.0, 24.0, 272.0, 152.0)
new Vec2 center = {x = 160.0, y = 100.0}

frame.clear(background)
frame.fill_rect(panel, Color.terminal_blue)
frame.circle(center, 48.0, Color.terminal_bright_yellow)
frame.fill_circle(center, 12.0, color(213, 78, 83, 192))

while window.is_open() {
    window.present(edit frame)
    sleep(16ms)
}
```

`Canvas` and `Window` are resources. They cannot be copied, returned from a
function, or passed by value. Use `edit Canvas` for mutable function access
and `view Canvas` for read-only access.

## Types and constructors

| Form | Result | Purpose |
|---|---|---|
| `color(r, g, b, a)` | `Color` | RGBA components in `0..255` |
| `color_hex("#rrggbb", a)` | `Color` | Six hex digits and alpha in `0..255` |
| `rect(x, y, width, height)` | `Rect` | A rectangle with `Float` coordinates |
| `canvas(width, height)` | `Canvas` | A software RGBA framebuffer |
| `windows.open(title, width, height)` | `Window` | A Win32 presentation window |

`Color` and `Rect` are copyable values. `Canvas` owns its pixel buffer and
`Window` owns the platform window. Both resources are released deterministically.

## Drawing

| Canvas method | Action |
|---|---|
| `clear(color)` | Fill the entire frame |
| `pixel(position, color)` | Draw a pixel |
| `line(from, to, color)` | Draw a line |
| `rect(area, color)` | Draw a rectangle outline |
| `fill_rect(area, color)` | Fill a rectangle |
| `circle(center, radius, color)` | Draw a circle outline |
| `fill_circle(center, radius, color)` | Fill a circle |
| `checksum()` | Return a stable framebuffer checksum |

Drawing uses source-over alpha blending. Coordinates are rounded
deterministically and out-of-bounds pixels are clipped safely. A negative radius
is a runtime error; a rectangle with a non-positive size draws nothing.

## Colors

Short aliases such as `Color.red`, `Color.green`, and `Color.blue` use soft,
UI-friendly shades instead of pure RGB extremes.

The full 16-color palette is exposed as:

| Base | Bright |
|---|---|
| `Color.terminal_black` | `Color.terminal_bright_black` |
| `Color.terminal_red` | `Color.terminal_bright_red` |
| `Color.terminal_green` | `Color.terminal_bright_green` |
| `Color.terminal_yellow` | `Color.terminal_bright_yellow` |
| `Color.terminal_blue` | `Color.terminal_bright_blue` |
| `Color.terminal_magenta` | `Color.terminal_bright_magenta` |
| `Color.terminal_cyan` | `Color.terminal_bright_cyan` |
| `Color.terminal_white` | `Color.terminal_bright_white` |

`Color.transparent` is also available.

## Window backend

```skadi
while window.is_open() {
    window.present(edit frame)
    sleep(16ms)
}
```

In Canvas v0, `windows.open` is implemented only for Win32. Headless Canvas,
drawing, and `checksum()` remain platform-independent. Window and Canvas
dimensions must match when calling `present`.

`present()` and `is_open()` pump the Win32 message queue. Long-running work must
not block the presentation loop, otherwise the operating system will correctly
mark the window as unresponsive.

After a conditional `window.close()`, `present()` requires a handler:

```skadi
window.present(edit frame) on error {
    pass
}
```

A closed window remains a safe sentinel handle until automatic cleanup; the
handler never accesses a released Win32 handle.

## Not included yet

- keyboard and mouse events;
- a continuous application loop and frame timing;
- text and fonts;
- images and codecs;
- transforms, clipping regions, and `Matrix2D`;
- resize and logical-size scaling;
- Linux, macOS, and embedded display backends;
- GPU APIs, scene graphs, retained UI, and layout engines.
