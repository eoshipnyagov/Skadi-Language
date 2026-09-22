#include <stdbool.h>

#include "driver/gpio.h"

#ifndef SKADI_BLINK_GPIO
#define SKADI_BLINK_GPIO 2
#endif

void board_init(void) {
    gpio_config_t config = {
        .pin_bit_mask = 1ULL << SKADI_BLINK_GPIO,
        .mode = GPIO_MODE_OUTPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_config(&config);
    gpio_set_level(SKADI_BLINK_GPIO, 0);
}

void board_set_led(bool enabled) {
    gpio_set_level(SKADI_BLINK_GPIO, enabled ? 1 : 0);
}
