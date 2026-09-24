# ESP32 GPIO interrupt

This project demonstrates the Skadi `Interrupt` capability backed by an
ESP-IDF GPIO ISR. Connect a momentary button between GPIO 4 and ground; the
example enables the internal pull-up resistor and reacts to the falling edge.

```sh
skadi embedded build
skadi embedded flash --port <serial-port> --monitor
```

The ISR performs only `Channel.try_send`. Output and the rest of the work stay
in normal task context. The manifest selects static Task/Channel allocation.
