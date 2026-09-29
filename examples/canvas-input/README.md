# Canvas input showcase

Build and run this project on Windows with a C compiler available:

```sh
skadi-cli check
skadi-cli build
skadi-cli run
```

Use WASD to move the circle, click to place it under the pointer, and press
Escape or close the window to exit. `window.is_open()` processes pending OS
messages at the start of each frame; `window.input` exposes that frame's
keyboard and mouse snapshot. On other host targets, headless Canvas remains
available, but `windows.open` requires the Win32 backend.
