#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

int sensor_calibrate(int16_t raw_value, int16_t offset, int32_t *out) {
    if (out == 0) {
        return 1;
    }
    int32_t calibrated = (int32_t)raw_value + (int32_t)offset;
    if (calibrated < 0 || calibrated > 1023) {
        return 2;
    }
    *out = calibrated;
    return 0;
}

bool sensor_is_valid(int32_t value) {
    return value >= 0 && value <= 1023;
}

uint32_t sensor_checksum(const uint8_t *data, size_t length) {
    uint32_t sum = 0;
    for (size_t index = 0; index < length; ++index) {
        sum += data[index];
    }
    return sum;
}

int sensor_adjust(uint8_t *data, size_t length, uint8_t delta) {
    if (data == 0 && length != 0) {
        return 1;
    }
    for (size_t index = 0; index < length; ++index) {
        if (data[index] > UINT8_MAX - delta) {
            return 2;
        }
        data[index] = (uint8_t)(data[index] + delta);
    }
    return 0;
}
