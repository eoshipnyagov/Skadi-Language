# ESP32 interrupt blink

This project is the first Skadi embedded vertical slice. A hardware GPTimer is
bound through `on interrupt`, the interrupt-safe handler uses `try_send`, and a
normal Skadi `Task` consumes the bounded `Channel` and drives a GPIO through a
small C board adapter.

The Skadi source contains no FreeRTOS types or handles.

```powershell
skadi-cli embedded prepare
skadi-cli embedded build
skadi-cli embedded flash --port COM7 --monitor
```

Run the commands from this directory in an activated ESP-IDF 5.4+ environment.
The default LED GPIO is `2`; change `SKADI_BLINK_GPIO` in `native/board.c` for
your board.
