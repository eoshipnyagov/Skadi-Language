#include <stdio.h>

#include <stddef.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

typedef int32_t SkInt;

#include <limits.h>

static void sk_numeric_panic(const char *message) {
    fprintf(stderr, "Skadi numeric runtime error [SC-RT-350]: %s\n", message);
    exit(1);
}

static SkInt sk_num_add_int(SkInt a, SkInt b) { if ((b > 0 && a > INT32_MAX - b) || (b < 0 && a < INT32_MIN - b)) sk_numeric_panic("integer addition overflow"); return (SkInt)(a + b); }
static SkInt sk_num_sub_int(SkInt a, SkInt b) { if ((b > 0 && a < INT32_MIN + b) || (b < 0 && a > INT32_MAX + b)) sk_numeric_panic("integer subtraction overflow"); return (SkInt)(a - b); }
static SkInt sk_num_mul_int(SkInt a, SkInt b) { if (a == 0 || b == 0) return 0; if ((a == -1 && b == INT32_MIN) || (b == -1 && a == INT32_MIN)) sk_numeric_panic("integer multiplication overflow"); if (a > 0 ? (b > 0 ? a > INT32_MAX / b : b < INT32_MIN / a) : (b > 0 ? a < INT32_MIN / b : a < INT32_MAX / b)) sk_numeric_panic("integer multiplication overflow"); return (SkInt)(a * b); }
static SkInt sk_num_div_int(SkInt a, SkInt b) { if (b == 0) sk_numeric_panic("integer division by zero"); if (a == INT32_MIN && b == -1) sk_numeric_panic("integer division overflow"); return (SkInt)(a / b); }
static SkInt sk_num_mod_int(SkInt a, SkInt b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); if (a == INT32_MIN && b == -1) return 0; return (SkInt)(a % b); }
static SkInt sk_num_neg_int(SkInt value) { if (value == INT32_MIN) sk_numeric_panic("integer negation overflow"); return (SkInt)(-value); }
static SkInt sk_num_pow_int(SkInt base, SkInt exponent) { if (exponent < 0) sk_numeric_panic("negative integer exponent"); SkInt result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_int(result, base); exponent = (SkInt)(exponent / 2); if (exponent > 0) base = sk_num_mul_int(base, base); } return result; }
static int8_t sk_num_add_i8(int8_t a, int8_t b) { if ((b > 0 && a > INT8_MAX - b) || (b < 0 && a < INT8_MIN - b)) sk_numeric_panic("integer addition overflow"); return (int8_t)(a + b); }
static int8_t sk_num_sub_i8(int8_t a, int8_t b) { if ((b > 0 && a < INT8_MIN + b) || (b < 0 && a > INT8_MAX + b)) sk_numeric_panic("integer subtraction overflow"); return (int8_t)(a - b); }
static int8_t sk_num_mul_i8(int8_t a, int8_t b) { if (a == 0 || b == 0) return 0; if ((a == -1 && b == INT8_MIN) || (b == -1 && a == INT8_MIN)) sk_numeric_panic("integer multiplication overflow"); if (a > 0 ? (b > 0 ? a > INT8_MAX / b : b < INT8_MIN / a) : (b > 0 ? a < INT8_MIN / b : a < INT8_MAX / b)) sk_numeric_panic("integer multiplication overflow"); return (int8_t)(a * b); }
static int8_t sk_num_div_i8(int8_t a, int8_t b) { if (b == 0) sk_numeric_panic("integer division by zero"); if (a == INT8_MIN && b == -1) sk_numeric_panic("integer division overflow"); return (int8_t)(a / b); }
static int8_t sk_num_mod_i8(int8_t a, int8_t b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); if (a == INT8_MIN && b == -1) return 0; return (int8_t)(a % b); }
static int8_t sk_num_neg_i8(int8_t value) { if (value == INT8_MIN) sk_numeric_panic("integer negation overflow"); return (int8_t)(-value); }
static int8_t sk_num_pow_i8(int8_t base, int8_t exponent) { if (exponent < 0) sk_numeric_panic("negative integer exponent"); int8_t result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_i8(result, base); exponent = (int8_t)(exponent / 2); if (exponent > 0) base = sk_num_mul_i8(base, base); } return result; }
static int16_t sk_num_add_i16(int16_t a, int16_t b) { if ((b > 0 && a > INT16_MAX - b) || (b < 0 && a < INT16_MIN - b)) sk_numeric_panic("integer addition overflow"); return (int16_t)(a + b); }
static int16_t sk_num_sub_i16(int16_t a, int16_t b) { if ((b > 0 && a < INT16_MIN + b) || (b < 0 && a > INT16_MAX + b)) sk_numeric_panic("integer subtraction overflow"); return (int16_t)(a - b); }
static int16_t sk_num_mul_i16(int16_t a, int16_t b) { if (a == 0 || b == 0) return 0; if ((a == -1 && b == INT16_MIN) || (b == -1 && a == INT16_MIN)) sk_numeric_panic("integer multiplication overflow"); if (a > 0 ? (b > 0 ? a > INT16_MAX / b : b < INT16_MIN / a) : (b > 0 ? a < INT16_MIN / b : a < INT16_MAX / b)) sk_numeric_panic("integer multiplication overflow"); return (int16_t)(a * b); }
static int16_t sk_num_div_i16(int16_t a, int16_t b) { if (b == 0) sk_numeric_panic("integer division by zero"); if (a == INT16_MIN && b == -1) sk_numeric_panic("integer division overflow"); return (int16_t)(a / b); }
static int16_t sk_num_mod_i16(int16_t a, int16_t b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); if (a == INT16_MIN && b == -1) return 0; return (int16_t)(a % b); }
static int16_t sk_num_neg_i16(int16_t value) { if (value == INT16_MIN) sk_numeric_panic("integer negation overflow"); return (int16_t)(-value); }
static int16_t sk_num_pow_i16(int16_t base, int16_t exponent) { if (exponent < 0) sk_numeric_panic("negative integer exponent"); int16_t result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_i16(result, base); exponent = (int16_t)(exponent / 2); if (exponent > 0) base = sk_num_mul_i16(base, base); } return result; }
static int32_t sk_num_add_i32(int32_t a, int32_t b) { if ((b > 0 && a > INT32_MAX - b) || (b < 0 && a < INT32_MIN - b)) sk_numeric_panic("integer addition overflow"); return (int32_t)(a + b); }
static int32_t sk_num_sub_i32(int32_t a, int32_t b) { if ((b > 0 && a < INT32_MIN + b) || (b < 0 && a > INT32_MAX + b)) sk_numeric_panic("integer subtraction overflow"); return (int32_t)(a - b); }
static int32_t sk_num_mul_i32(int32_t a, int32_t b) { if (a == 0 || b == 0) return 0; if ((a == -1 && b == INT32_MIN) || (b == -1 && a == INT32_MIN)) sk_numeric_panic("integer multiplication overflow"); if (a > 0 ? (b > 0 ? a > INT32_MAX / b : b < INT32_MIN / a) : (b > 0 ? a < INT32_MIN / b : a < INT32_MAX / b)) sk_numeric_panic("integer multiplication overflow"); return (int32_t)(a * b); }
static int32_t sk_num_div_i32(int32_t a, int32_t b) { if (b == 0) sk_numeric_panic("integer division by zero"); if (a == INT32_MIN && b == -1) sk_numeric_panic("integer division overflow"); return (int32_t)(a / b); }
static int32_t sk_num_mod_i32(int32_t a, int32_t b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); if (a == INT32_MIN && b == -1) return 0; return (int32_t)(a % b); }
static int32_t sk_num_neg_i32(int32_t value) { if (value == INT32_MIN) sk_numeric_panic("integer negation overflow"); return (int32_t)(-value); }
static int32_t sk_num_pow_i32(int32_t base, int32_t exponent) { if (exponent < 0) sk_numeric_panic("negative integer exponent"); int32_t result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_i32(result, base); exponent = (int32_t)(exponent / 2); if (exponent > 0) base = sk_num_mul_i32(base, base); } return result; }
static int64_t sk_num_add_i64(int64_t a, int64_t b) { if ((b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b)) sk_numeric_panic("integer addition overflow"); return (int64_t)(a + b); }
static int64_t sk_num_sub_i64(int64_t a, int64_t b) { if ((b > 0 && a < INT64_MIN + b) || (b < 0 && a > INT64_MAX + b)) sk_numeric_panic("integer subtraction overflow"); return (int64_t)(a - b); }
static int64_t sk_num_mul_i64(int64_t a, int64_t b) { if (a == 0 || b == 0) return 0; if ((a == -1 && b == INT64_MIN) || (b == -1 && a == INT64_MIN)) sk_numeric_panic("integer multiplication overflow"); if (a > 0 ? (b > 0 ? a > INT64_MAX / b : b < INT64_MIN / a) : (b > 0 ? a < INT64_MIN / b : a < INT64_MAX / b)) sk_numeric_panic("integer multiplication overflow"); return (int64_t)(a * b); }
static int64_t sk_num_div_i64(int64_t a, int64_t b) { if (b == 0) sk_numeric_panic("integer division by zero"); if (a == INT64_MIN && b == -1) sk_numeric_panic("integer division overflow"); return (int64_t)(a / b); }
static int64_t sk_num_mod_i64(int64_t a, int64_t b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); if (a == INT64_MIN && b == -1) return 0; return (int64_t)(a % b); }
static int64_t sk_num_neg_i64(int64_t value) { if (value == INT64_MIN) sk_numeric_panic("integer negation overflow"); return (int64_t)(-value); }
static int64_t sk_num_pow_i64(int64_t base, int64_t exponent) { if (exponent < 0) sk_numeric_panic("negative integer exponent"); int64_t result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_i64(result, base); exponent = (int64_t)(exponent / 2); if (exponent > 0) base = sk_num_mul_i64(base, base); } return result; }
static uint8_t sk_num_add_u8(uint8_t a, uint8_t b) { if (a > UINT8_MAX - b) sk_numeric_panic("integer addition overflow"); return (uint8_t)(a + b); }
static uint8_t sk_num_sub_u8(uint8_t a, uint8_t b) { if (a < b) sk_numeric_panic("integer subtraction overflow"); return (uint8_t)(a - b); }
static uint8_t sk_num_mul_u8(uint8_t a, uint8_t b) { if (b != 0 && a > UINT8_MAX / b) sk_numeric_panic("integer multiplication overflow"); return (uint8_t)(a * b); }
static uint8_t sk_num_div_u8(uint8_t a, uint8_t b) { if (b == 0) sk_numeric_panic("integer division by zero"); return (uint8_t)(a / b); }
static uint8_t sk_num_mod_u8(uint8_t a, uint8_t b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); return (uint8_t)(a % b); }
static uint8_t sk_num_pow_u8(uint8_t base, uint8_t exponent) { uint8_t result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_u8(result, base); exponent = (uint8_t)(exponent / 2); if (exponent > 0) base = sk_num_mul_u8(base, base); } return result; }
static uint16_t sk_num_add_u16(uint16_t a, uint16_t b) { if (a > UINT16_MAX - b) sk_numeric_panic("integer addition overflow"); return (uint16_t)(a + b); }
static uint16_t sk_num_sub_u16(uint16_t a, uint16_t b) { if (a < b) sk_numeric_panic("integer subtraction overflow"); return (uint16_t)(a - b); }
static uint16_t sk_num_mul_u16(uint16_t a, uint16_t b) { if (b != 0 && a > UINT16_MAX / b) sk_numeric_panic("integer multiplication overflow"); return (uint16_t)(a * b); }
static uint16_t sk_num_div_u16(uint16_t a, uint16_t b) { if (b == 0) sk_numeric_panic("integer division by zero"); return (uint16_t)(a / b); }
static uint16_t sk_num_mod_u16(uint16_t a, uint16_t b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); return (uint16_t)(a % b); }
static uint16_t sk_num_pow_u16(uint16_t base, uint16_t exponent) { uint16_t result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_u16(result, base); exponent = (uint16_t)(exponent / 2); if (exponent > 0) base = sk_num_mul_u16(base, base); } return result; }
static uint32_t sk_num_add_u32(uint32_t a, uint32_t b) { if (a > UINT32_MAX - b) sk_numeric_panic("integer addition overflow"); return (uint32_t)(a + b); }
static uint32_t sk_num_sub_u32(uint32_t a, uint32_t b) { if (a < b) sk_numeric_panic("integer subtraction overflow"); return (uint32_t)(a - b); }
static uint32_t sk_num_mul_u32(uint32_t a, uint32_t b) { if (b != 0 && a > UINT32_MAX / b) sk_numeric_panic("integer multiplication overflow"); return (uint32_t)(a * b); }
static uint32_t sk_num_div_u32(uint32_t a, uint32_t b) { if (b == 0) sk_numeric_panic("integer division by zero"); return (uint32_t)(a / b); }
static uint32_t sk_num_mod_u32(uint32_t a, uint32_t b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); return (uint32_t)(a % b); }
static uint32_t sk_num_pow_u32(uint32_t base, uint32_t exponent) { uint32_t result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_u32(result, base); exponent = (uint32_t)(exponent / 2); if (exponent > 0) base = sk_num_mul_u32(base, base); } return result; }
static uint64_t sk_num_add_u64(uint64_t a, uint64_t b) { if (a > UINT64_MAX - b) sk_numeric_panic("integer addition overflow"); return (uint64_t)(a + b); }
static uint64_t sk_num_sub_u64(uint64_t a, uint64_t b) { if (a < b) sk_numeric_panic("integer subtraction overflow"); return (uint64_t)(a - b); }
static uint64_t sk_num_mul_u64(uint64_t a, uint64_t b) { if (b != 0 && a > UINT64_MAX / b) sk_numeric_panic("integer multiplication overflow"); return (uint64_t)(a * b); }
static uint64_t sk_num_div_u64(uint64_t a, uint64_t b) { if (b == 0) sk_numeric_panic("integer division by zero"); return (uint64_t)(a / b); }
static uint64_t sk_num_mod_u64(uint64_t a, uint64_t b) { if (b == 0) sk_numeric_panic("integer remainder by zero"); return (uint64_t)(a % b); }
static uint64_t sk_num_pow_u64(uint64_t base, uint64_t exponent) { uint64_t result = 1; while (exponent > 0) { if (exponent & 1) result = sk_num_mul_u64(result, base); exponent = (uint64_t)(exponent / 2); if (exponent > 0) base = sk_num_mul_u64(base, base); } return result; }

#include <string.h>

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/queue.h"
#include "freertos/semphr.h"
#include "esp_timer.h"
#include "esp_attr.h"
#include "driver/gptimer.h"
#include "esp_err.h"
#define SK_INTERRUPT_ATTR IRAM_ATTR

typedef struct { float x; float y; } Vec2;
typedef struct { float x; float y; float z; } Vec3;
typedef struct { float x; float y; float z; float w; } Vec4;

static Vec2 sk_vec2_add(Vec2 a, Vec2 b) { return (Vec2){.x = a.x + b.x, .y = a.y + b.y}; }
static Vec2 sk_vec2_sub(Vec2 a, Vec2 b) { return (Vec2){.x = a.x - b.x, .y = a.y - b.y}; }
static Vec2 sk_vec2_scale(Vec2 value, float scalar) { return (Vec2){.x = value.x * scalar, .y = value.y * scalar}; }
static Vec2 sk_vec2_div(Vec2 value, float scalar) { return (Vec2){.x = value.x / scalar, .y = value.y / scalar}; }
static Vec2 sk_vec2_neg(Vec2 value) { return (Vec2){.x = -value.x, .y = -value.y}; }
static float sk_vec2_dot(Vec2 a, Vec2 b) { return a.x * b.x + a.y * b.y; }
static float sk_vec2_length_sq(Vec2 value) { return sk_vec2_dot(value, value); }
static float sk_vec2_distance_sq(Vec2 a, Vec2 b) { return sk_vec2_length_sq(sk_vec2_sub(a, b)); }

static Vec3 sk_vec3_add(Vec3 a, Vec3 b) { return (Vec3){.x = a.x + b.x, .y = a.y + b.y, .z = a.z + b.z}; }
static Vec3 sk_vec3_sub(Vec3 a, Vec3 b) { return (Vec3){.x = a.x - b.x, .y = a.y - b.y, .z = a.z - b.z}; }
static Vec3 sk_vec3_scale(Vec3 value, float scalar) { return (Vec3){.x = value.x * scalar, .y = value.y * scalar, .z = value.z * scalar}; }
static Vec3 sk_vec3_div(Vec3 value, float scalar) { return (Vec3){.x = value.x / scalar, .y = value.y / scalar, .z = value.z / scalar}; }
static Vec3 sk_vec3_neg(Vec3 value) { return (Vec3){.x = -value.x, .y = -value.y, .z = -value.z}; }
static float sk_vec3_dot(Vec3 a, Vec3 b) { return a.x * b.x + a.y * b.y + a.z * b.z; }
static float sk_vec3_length_sq(Vec3 value) { return sk_vec3_dot(value, value); }
static float sk_vec3_distance_sq(Vec3 a, Vec3 b) { return sk_vec3_length_sq(sk_vec3_sub(a, b)); }

static Vec4 sk_vec4_add(Vec4 a, Vec4 b) { return (Vec4){.x = a.x + b.x, .y = a.y + b.y, .z = a.z + b.z, .w = a.w + b.w}; }
static Vec4 sk_vec4_sub(Vec4 a, Vec4 b) { return (Vec4){.x = a.x - b.x, .y = a.y - b.y, .z = a.z - b.z, .w = a.w - b.w}; }
static Vec4 sk_vec4_scale(Vec4 value, float scalar) { return (Vec4){.x = value.x * scalar, .y = value.y * scalar, .z = value.z * scalar, .w = value.w * scalar}; }
static Vec4 sk_vec4_div(Vec4 value, float scalar) { return (Vec4){.x = value.x / scalar, .y = value.y / scalar, .z = value.z / scalar, .w = value.w / scalar}; }
static Vec4 sk_vec4_neg(Vec4 value) { return (Vec4){.x = -value.x, .y = -value.y, .z = -value.z, .w = -value.w}; }
static float sk_vec4_dot(Vec4 a, Vec4 b) { return a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w; }
static float sk_vec4_length_sq(Vec4 value) { return sk_vec4_dot(value, value); }
static float sk_vec4_distance_sq(Vec4 a, Vec4 b) { return sk_vec4_length_sq(sk_vec4_sub(a, b)); }

static Vec3 sk_vec3_cross(Vec3 a, Vec3 b) { return (Vec3){.x = a.y * b.z - a.z * b.y, .y = a.z * b.x - a.x * b.z, .z = a.x * b.y - a.y * b.x}; }

typedef struct { uint8_t r; uint8_t g; uint8_t b; uint8_t a; } SkColor;
typedef struct { float x; float y; float width; float height; } SkRect;
typedef struct {
    int64_t width;
    int64_t height;
    uint32_t *pixels;
} SkCanvas;

static void sk_visual_panic(const char *message) {
    fprintf(stderr, "Runtime error: [SC-RT-330] %s\n", message);
    exit(1);
}

static SkColor sk_color(int64_t r, int64_t g, int64_t b, int64_t a) {
    if (r < 0 || r > 255 || g < 0 || g > 255 || b < 0 || b > 255 || a < 0 || a > 255) {
        sk_visual_panic("color components must be in 0..255");
    }
    return (SkColor){(uint8_t)r, (uint8_t)g, (uint8_t)b, (uint8_t)a};
}

static int sk_hex_digit(char value) {
    if (value >= '0' && value <= '9') return value - '0';
    if (value >= 'a' && value <= 'f') return value - 'a' + 10;
    if (value >= 'A' && value <= 'F') return value - 'A' + 10;
    return -1;
}

static SkColor sk_color_hex(const char *text, int64_t alpha) {
    if (!text) sk_visual_panic("color_hex expects rrggbb or #rrggbb text");
    if (text[0] == '#') text += 1;
    if (strlen(text) != 6) sk_visual_panic("color_hex expects exactly six hexadecimal digits");
    int digits[6];
    for (int index = 0; index < 6; ++index) {
        digits[index] = sk_hex_digit(text[index]);
        if (digits[index] < 0) sk_visual_panic("color_hex contains a non-hexadecimal digit");
    }
    return sk_color(
        digits[0] * 16 + digits[1],
        digits[2] * 16 + digits[3],
        digits[4] * 16 + digits[5],
        alpha
    );
}

#define Color_terminal_black ((SkColor){29, 31, 33, 255})
#define Color_terminal_red ((SkColor){204, 102, 102, 255})
#define Color_terminal_green ((SkColor){181, 189, 104, 255})
#define Color_terminal_yellow ((SkColor){240, 198, 116, 255})
#define Color_terminal_blue ((SkColor){129, 162, 190, 255})
#define Color_terminal_magenta ((SkColor){178, 148, 187, 255})
#define Color_terminal_cyan ((SkColor){138, 190, 183, 255})
#define Color_terminal_white ((SkColor){197, 200, 198, 255})
#define Color_terminal_bright_black ((SkColor){102, 102, 102, 255})
#define Color_terminal_bright_red ((SkColor){213, 78, 83, 255})
#define Color_terminal_bright_green ((SkColor){185, 202, 74, 255})
#define Color_terminal_bright_yellow ((SkColor){231, 197, 71, 255})
#define Color_terminal_bright_blue ((SkColor){122, 166, 218, 255})
#define Color_terminal_bright_magenta ((SkColor){195, 151, 216, 255})
#define Color_terminal_bright_cyan ((SkColor){112, 192, 177, 255})
#define Color_terminal_bright_white ((SkColor){234, 234, 234, 255})
#define Color_black Color_terminal_black
#define Color_red Color_terminal_red
#define Color_green Color_terminal_green
#define Color_yellow Color_terminal_yellow
#define Color_blue Color_terminal_blue
#define Color_magenta Color_terminal_magenta
#define Color_cyan Color_terminal_cyan
#define Color_white Color_terminal_bright_white
#define Color_transparent ((SkColor){0, 0, 0, 0})

static SkRect sk_rect(float x, float y, float width, float height) {
    return (SkRect){x, y, width, height};
}

static int64_t sk_visual_round(float value) {
    return (int64_t)(value >= 0.0 ? value + 0.5 : value - 0.5);
}

static uint32_t sk_color_pack(SkColor color) {
    return ((uint32_t)color.a << 24) | ((uint32_t)color.r << 16)
        | ((uint32_t)color.g << 8) | (uint32_t)color.b;
}

static SkColor sk_color_unpack(uint32_t value) {
    return (SkColor){
        (uint8_t)((value >> 16) & 255),
        (uint8_t)((value >> 8) & 255),
        (uint8_t)(value & 255),
        (uint8_t)((value >> 24) & 255)
    };
}

static SkCanvas sk_canvas_create(int64_t width, int64_t height) {
    if (width <= 0 || height <= 0 || (uint64_t)width > SIZE_MAX / sizeof(uint32_t)
        || (uint64_t)height > SIZE_MAX / ((size_t)width * sizeof(uint32_t))) {
        sk_visual_panic("canvas dimensions must be positive and fit addressable memory");
    }
    SkCanvas canvas = {width, height, NULL};
    canvas.pixels = (uint32_t*)calloc((size_t)width * (size_t)height, sizeof(uint32_t));
    if (!canvas.pixels) sk_visual_panic("canvas framebuffer allocation failed");
    return canvas;
}

static void sk_canvas_destroy(SkCanvas *canvas) {
    if (!canvas) return;
    free(canvas->pixels);
    canvas->pixels = NULL;
    canvas->width = 0;
    canvas->height = 0;
}

static SkCanvas sk_canvas_move(SkCanvas *source) {
    if (!source) {
        SkCanvas empty = {0};
        return empty;
    }
    SkCanvas result = *source;
    source->width = 0;
    source->height = 0;
    source->pixels = NULL;
    return result;
}

static void sk_canvas_blend_pixel(SkCanvas *canvas, int64_t x, int64_t y, SkColor source) {
    if (!canvas || !canvas->pixels || x < 0 || y < 0 || x >= canvas->width || y >= canvas->height) return;
    size_t index = (size_t)y * (size_t)canvas->width + (size_t)x;
    if (source.a == 255) {
        canvas->pixels[index] = sk_color_pack(source);
        return;
    }
    if (source.a == 0) return;
    SkColor target = sk_color_unpack(canvas->pixels[index]);
    uint32_t alpha = source.a;
    uint32_t inverse = 255 - alpha;
    SkColor blended = {
        (uint8_t)((source.r * alpha + target.r * inverse + 127) / 255),
        (uint8_t)((source.g * alpha + target.g * inverse + 127) / 255),
        (uint8_t)((source.b * alpha + target.b * inverse + 127) / 255),
        (uint8_t)(alpha + (target.a * inverse + 127) / 255)
    };
    canvas->pixels[index] = sk_color_pack(blended);
}

static int64_t sk_canvas_clear(SkCanvas *canvas, SkColor color) {
    if (!canvas || !canvas->pixels) sk_visual_panic("clear requires a live Canvas");
    uint32_t packed = sk_color_pack(color);
    size_t count = (size_t)canvas->width * (size_t)canvas->height;
    for (size_t index = 0; index < count; ++index) canvas->pixels[index] = packed;
    return 0;
}

static int64_t sk_canvas_pixel(SkCanvas *canvas, Vec2 position, SkColor color) {
    sk_canvas_blend_pixel(canvas, sk_visual_round(position.x), sk_visual_round(position.y), color);
    return 0;
}

static int64_t sk_canvas_line(SkCanvas *canvas, Vec2 from, Vec2 to, SkColor color) {
    int64_t x0 = sk_visual_round(from.x), y0 = sk_visual_round(from.y);
    int64_t x1 = sk_visual_round(to.x), y1 = sk_visual_round(to.y);
    int64_t dx = x1 >= x0 ? x1 - x0 : x0 - x1;
    int64_t sx = x0 < x1 ? 1 : -1;
    int64_t dy_abs = y1 >= y0 ? y1 - y0 : y0 - y1;
    int64_t dy = -dy_abs;
    int64_t sy = y0 < y1 ? 1 : -1;
    int64_t error = dx + dy;
    for (;;) {
        sk_canvas_blend_pixel(canvas, x0, y0, color);
        if (x0 == x1 && y0 == y1) break;
        int64_t doubled = 2 * error;
        if (doubled >= dy) { error += dy; x0 += sx; }
        if (doubled <= dx) { error += dx; y0 += sy; }
    }
    return 0;
}

static int64_t sk_canvas_fill_rect(SkCanvas *canvas, SkRect area, SkColor color) {
    if (area.width <= 0.0 || area.height <= 0.0) return 0;
    int64_t left = sk_visual_round(area.x);
    int64_t top = sk_visual_round(area.y);
    int64_t right = sk_visual_round(area.x + area.width) - 1;
    int64_t bottom = sk_visual_round(area.y + area.height) - 1;
    for (int64_t y = top; y <= bottom; ++y)
        for (int64_t x = left; x <= right; ++x)
            sk_canvas_blend_pixel(canvas, x, y, color);
    return 0;
}

static int64_t sk_canvas_rect(SkCanvas *canvas, SkRect area, SkColor color) {
    if (area.width <= 0.0 || area.height <= 0.0) return 0;
    int64_t left = sk_visual_round(area.x);
    int64_t top = sk_visual_round(area.y);
    int64_t right = sk_visual_round(area.x + area.width) - 1;
    int64_t bottom = sk_visual_round(area.y + area.height) - 1;
    for (int64_t x = left; x <= right; ++x) {
        sk_canvas_blend_pixel(canvas, x, top, color);
        sk_canvas_blend_pixel(canvas, x, bottom, color);
    }
    for (int64_t y = top; y <= bottom; ++y) {
        sk_canvas_blend_pixel(canvas, left, y, color);
        sk_canvas_blend_pixel(canvas, right, y, color);
    }
    return 0;
}

static void sk_canvas_circle_octants(SkCanvas *canvas, int64_t cx, int64_t cy, int64_t x, int64_t y, SkColor color) {
    sk_canvas_blend_pixel(canvas, cx + x, cy + y, color);
    sk_canvas_blend_pixel(canvas, cx + y, cy + x, color);
    sk_canvas_blend_pixel(canvas, cx - y, cy + x, color);
    sk_canvas_blend_pixel(canvas, cx - x, cy + y, color);
    sk_canvas_blend_pixel(canvas, cx - x, cy - y, color);
    sk_canvas_blend_pixel(canvas, cx - y, cy - x, color);
    sk_canvas_blend_pixel(canvas, cx + y, cy - x, color);
    sk_canvas_blend_pixel(canvas, cx + x, cy - y, color);
}

static int64_t sk_canvas_circle(SkCanvas *canvas, Vec2 center, float radius, SkColor color) {
    if (radius < 0.0) sk_visual_panic("circle radius cannot be negative");
    int64_t cx = sk_visual_round(center.x), cy = sk_visual_round(center.y);
    int64_t x = sk_visual_round(radius), y = 0, error = 1 - x;
    while (x >= y) {
        sk_canvas_circle_octants(canvas, cx, cy, x, y, color);
        ++y;
        if (error < 0) error += 2 * y + 1;
        else { --x; error += 2 * (y - x) + 1; }
    }
    return 0;
}

static int64_t sk_canvas_fill_circle(SkCanvas *canvas, Vec2 center, float radius, SkColor color) {
    if (radius < 0.0) sk_visual_panic("fill_circle radius cannot be negative");
    int64_t cx = sk_visual_round(center.x), cy = sk_visual_round(center.y);
    int64_t r = sk_visual_round(radius);
    int64_t radius_squared = r * r;
    for (int64_t y = -r; y <= r; ++y)
        for (int64_t x = -r; x <= r; ++x)
            if (x * x + y * y <= radius_squared)
                sk_canvas_blend_pixel(canvas, cx + x, cy + y, color);
    return 0;
}

static int64_t sk_canvas_checksum(const SkCanvas *canvas) {
    if (!canvas || !canvas->pixels) sk_visual_panic("checksum requires a live Canvas");
    uint64_t hash = 1469598103934665603ULL;
    size_t bytes = (size_t)canvas->width * (size_t)canvas->height * sizeof(uint32_t);
    const unsigned char *data = (const unsigned char*)canvas->pixels;
    for (size_t index = 0; index < bytes; ++index) {
        hash ^= data[index];
        hash *= 1099511628211ULL;
    }
    return (int64_t)(hash & 0x7fffffffffffffffULL);
}

#if defined(_MSC_VER)
#define SK_THREAD_LOCAL __declspec(thread)
#else
#define SK_THREAD_LOCAL _Thread_local
#endif

typedef TaskHandle_t SkPlatformThread;

typedef struct SkTask SkTask;
typedef void (*SkTaskEntry)(SkTask *task, void *context);
typedef void (*SkTaskWake)(void *context);

struct SkTask {
    SkPlatformThread thread;
    void *context;
    bool started;
    bool joined;
    volatile bool stop_requested;
    volatile bool completed;
    SemaphoreHandle_t completion;
    portMUX_TYPE lock;
    void *wait_context;
    SkTaskWake wake_wait;
};

static SK_THREAD_LOCAL SkTask *sk_current_task = NULL;
static SK_THREAD_LOCAL bool sk_operation_timed_out_state = false;

static bool sk_operation_timed_out(void) {
    return sk_operation_timed_out_state;
}

typedef struct {
    SkTask *task;
    SkTaskEntry entry;
} SkTaskLaunch;

static void sk_task_panic(const char *code, const char *message) {
    fprintf(stderr, "Runtime error: [%s] %s\n", code, message);
    abort();
}

static void sk_task_platform_entry(void *raw) {
    SkTaskLaunch *launch = (SkTaskLaunch*)raw;
    SkTask *task = launch->task;
    SkTaskEntry entry = launch->entry;
    free(launch);
    sk_current_task = task;
    entry(task, task->context);
    taskENTER_CRITICAL(&task->lock);
    task->completed = true;
    taskEXIT_CRITICAL(&task->lock);
    xSemaphoreGive(task->completion);
    sk_current_task = NULL;
    vTaskDelete(NULL);
}

#ifndef SKADI_TASK_STACK_WORDS
#define SKADI_TASK_STACK_WORDS 4096
#endif

#ifndef SKADI_TASK_PRIORITY
#define SKADI_TASK_PRIORITY (tskIDLE_PRIORITY + 1)
#endif

static bool sk_task_start(SkTask *task, SkTaskEntry entry, void *context) {
    if (!task || !entry || !context) return false;
    memset(task, 0, sizeof(*task));
    task->context = context;
    task->lock = (portMUX_TYPE)portMUX_INITIALIZER_UNLOCKED;
    task->completion = xSemaphoreCreateBinary();
    if (!task->completion) return false;
    SkTaskLaunch *launch = (SkTaskLaunch*)malloc(sizeof(SkTaskLaunch));
    if (!launch) { vSemaphoreDelete(task->completion); task->completion = NULL; return false; }
    launch->task = task;
    launch->entry = entry;
    BaseType_t created = xTaskCreate(
        sk_task_platform_entry,
        "skadi-task",
        SKADI_TASK_STACK_WORDS,
        launch,
        SKADI_TASK_PRIORITY,
        &task->thread
    );
    if (created != pdPASS) {
        free(launch);
        vSemaphoreDelete(task->completion);
        task->completion = NULL;
        return false;
    }
    task->started = true;
    return true;
}

static void sk_task_request_stop(SkTask *task) {
    if (!task || !task->started || task->joined) sk_task_panic("SC-RT-303", "invalid task state at stop");
    taskENTER_CRITICAL(&task->lock);
    task->stop_requested = true;
    taskEXIT_CRITICAL(&task->lock);
    if (task->thread) xTaskAbortDelay(task->thread);
}

static bool sk_task_register_wait(void *context, SkTaskWake wake_wait) {
    SkTask *task = sk_current_task;
    if (!task) return true;
    taskENTER_CRITICAL(&task->lock);
    bool registered = !task->stop_requested;
    if (registered) { task->wait_context = context; task->wake_wait = wake_wait; }
    taskEXIT_CRITICAL(&task->lock);
    return registered;
}

static bool sk_task_finish_wait(void *context) {
    SkTask *task = sk_current_task;
    if (!task) return true;
    taskENTER_CRITICAL(&task->lock);
    bool active = !task->stop_requested;
    if (task->wait_context == context) { task->wait_context = NULL; task->wake_wait = NULL; }
    taskEXIT_CRITICAL(&task->lock);
    return active;
}

static bool sk_task_is_stopping(void) {
    SkTask *task = sk_current_task;
    if (!task) sk_task_panic("SC-RT-303", "stopping evaluated outside task context");
    taskENTER_CRITICAL(&task->lock);
    bool requested = task->stop_requested;
    taskEXIT_CRITICAL(&task->lock);
    return requested;
}

static void sk_task_join(SkTask *task) {
    if (!task || !task->started || task->joined) sk_task_panic("SC-RT-303", "invalid task state at wait");
    if (xSemaphoreTake(task->completion, portMAX_DELAY) != pdTRUE)
        sk_task_panic("SC-RT-302", "task join failed");
    vSemaphoreDelete(task->completion);
    task->completion = NULL;
    task->joined = true;
}

static bool sk_task_join_for(SkTask *task, int64_t timeout_ns) {
    if (!task || !task->started || task->joined) sk_task_panic("SC-RT-303", "invalid task state at timed wait");
    if (timeout_ns < 0) timeout_ns = 0;
    uint64_t millis = ((uint64_t)timeout_ns + 999999ULL) / 1000000ULL;
    TickType_t ticks = timeout_ns == 0 ? 0 : pdMS_TO_TICKS(millis);
    if (timeout_ns > 0 && ticks == 0) ticks = 1;
    if (xSemaphoreTake(task->completion, ticks) != pdTRUE) return false;
    vSemaphoreDelete(task->completion);
    task->completion = NULL;
    task->joined = true;
    return true;
}

static void sk_task_release_context(SkTask *task) {
    if (!task || !task->joined || !task->context) sk_task_panic("SC-RT-303", "invalid task state at context release");
    free(task->context);
    task->context = NULL;
}

typedef enum {
    SK_CHANNEL_OK = 0,
    SK_CHANNEL_CLOSED = 1,
    SK_CHANNEL_CANCELLED = 2,
    SK_CHANNEL_TIMED_OUT = 3
} SkChannelStatus;

typedef struct SkChannel {
    QueueHandle_t queue;
    size_t capacity;
    size_t element_size;
    volatile bool closed;
    portMUX_TYPE lock;
} SkChannel;

static void sk_channel_panic(const char *code, const char *message) {
    fprintf(stderr, "Runtime error: [%s] %s\n", code, message);
    abort();
}

static TickType_t sk_channel_timeout_ticks(int64_t timeout_ns) {
    if (timeout_ns < 0) return portMAX_DELAY;
    if (timeout_ns == 0) return 0;
    uint64_t millis = ((uint64_t)timeout_ns + 999999ULL) / 1000000ULL;
    TickType_t ticks = pdMS_TO_TICKS(millis);
    return ticks == 0 ? 1 : ticks;
}

#ifndef SKADI_CHANNEL_POLL_TICKS
#define SKADI_CHANNEL_POLL_TICKS 1
#endif

static TickType_t sk_channel_wait_slice(TickType_t remaining) {
    TickType_t slice = SKADI_CHANNEL_POLL_TICKS == 0 ? 1 : SKADI_CHANNEL_POLL_TICKS;
    return remaining == portMAX_DELAY || remaining > slice ? slice : remaining;
}

static SkChannel* sk_channel_create(int64_t capacity_value, size_t element_size) {
    if (capacity_value <= 0 || element_size == 0 || (uint64_t)capacity_value > SIZE_MAX / element_size)
        sk_channel_panic("SC-RT-312", "channel capacity must be positive and fit addressable memory");
    SkChannel *channel = (SkChannel*)calloc(1, sizeof(SkChannel));
    if (!channel) sk_channel_panic("SC-RT-311", "channel allocation failed");
    channel->capacity = (size_t)capacity_value;
    channel->element_size = element_size;
    channel->lock = (portMUX_TYPE)portMUX_INITIALIZER_UNLOCKED;
    channel->queue = xQueueCreate((UBaseType_t)channel->capacity, (UBaseType_t)element_size);
    if (!channel->queue) { free(channel); sk_channel_panic("SC-RT-311", "channel queue allocation failed"); }
    return channel;
}

static void sk_channel_wake_waiters(void *context) {
    (void)context;
}

static SkChannelStatus sk_channel_send_raw(SkChannel *channel, const void *value, int64_t timeout_ns) {
    if (!channel || !value) sk_channel_panic("SC-RT-313", "invalid channel send state");
    TickType_t timeout = sk_channel_timeout_ticks(timeout_ns);
    TickType_t started = xTaskGetTickCount();
    for (;;) {
        taskENTER_CRITICAL(&channel->lock);
        if (channel->closed) {
            taskEXIT_CRITICAL(&channel->lock);
            return SK_CHANNEL_CLOSED;
        }
        BaseType_t sent = xQueueSend(channel->queue, value, 0);
        taskEXIT_CRITICAL(&channel->lock);
        if (sent == pdTRUE) return SK_CHANNEL_OK;
        if (timeout == 0) return SK_CHANNEL_TIMED_OUT;
        TickType_t elapsed = xTaskGetTickCount() - started;
        if (timeout != portMAX_DELAY && elapsed >= timeout) return SK_CHANNEL_TIMED_OUT;
        TickType_t remaining = timeout == portMAX_DELAY ? portMAX_DELAY : timeout - elapsed;
        if (!sk_task_register_wait(channel, sk_channel_wake_waiters)) return SK_CHANNEL_CANCELLED;
        vTaskDelay(sk_channel_wait_slice(remaining));
        if (!sk_task_finish_wait(channel)) return SK_CHANNEL_CANCELLED;
    }
}

static SkChannelStatus sk_channel_receive_raw(SkChannel *channel, void *out_value, int64_t timeout_ns) {
    if (!channel || !out_value) sk_channel_panic("SC-RT-313", "invalid channel receive state");
    TickType_t timeout = sk_channel_timeout_ticks(timeout_ns);
    TickType_t started = xTaskGetTickCount();
    for (;;) {
        taskENTER_CRITICAL(&channel->lock);
        BaseType_t received = xQueueReceive(channel->queue, out_value, 0);
        bool closed = channel->closed;
        taskEXIT_CRITICAL(&channel->lock);
        if (received == pdTRUE) return SK_CHANNEL_OK;
        if (closed) return SK_CHANNEL_CLOSED;
        if (timeout == 0) return SK_CHANNEL_TIMED_OUT;
        TickType_t elapsed = xTaskGetTickCount() - started;
        if (timeout != portMAX_DELAY && elapsed >= timeout) return SK_CHANNEL_TIMED_OUT;
        TickType_t remaining = timeout == portMAX_DELAY ? portMAX_DELAY : timeout - elapsed;
        if (!sk_task_register_wait(channel, sk_channel_wake_waiters)) return SK_CHANNEL_CANCELLED;
        vTaskDelay(sk_channel_wait_slice(remaining));
        if (!sk_task_finish_wait(channel)) return SK_CHANNEL_CANCELLED;
    }
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_raw(SkChannel *channel, const void *value) {
    if (!channel || !value) return false;
    if (xPortInIsrContext()) {
        BaseType_t higher_priority_woken = pdFALSE;
        taskENTER_CRITICAL_ISR(&channel->lock);
        BaseType_t sent = channel->closed
            ? pdFALSE
            : xQueueSendFromISR(channel->queue, value, &higher_priority_woken);
        taskEXIT_CRITICAL_ISR(&channel->lock);
        if (higher_priority_woken == pdTRUE) portYIELD_FROM_ISR();
        return sent == pdTRUE;
    }
    taskENTER_CRITICAL(&channel->lock);
    BaseType_t sent = channel->closed ? pdFALSE : xQueueSend(channel->queue, value, 0);
    taskEXIT_CRITICAL(&channel->lock);
    return sent == pdTRUE;
}

static bool sk_channel_close(SkChannel *channel) {
    if (!channel) return false;
    taskENTER_CRITICAL(&channel->lock);
    bool changed = !channel->closed;
    channel->closed = true;
    taskEXIT_CRITICAL(&channel->lock);
    return changed;
}

static void sk_channel_destroy(SkChannel *channel) {
    if (!channel) return;
    vQueueDelete(channel->queue);
    free(channel);
}

static SkChannel* sk_channel_move(SkChannel **source) {
    if (!source) return NULL;
    SkChannel *result = *source;
    *source = NULL;
    return result;
}

typedef void (*SkInterruptHandler)(void *context);

typedef struct {
    int64_t period_ns;
    SkInterruptHandler handler;
    void *context;
    bool started;
    gptimer_handle_t timer;
} SkInterrupt;

static void sk_interrupt_panic(const char *message) {
    fprintf(stderr, "Runtime error: [SC-RT-320] %s\n", message);
    abort();
}

static bool IRAM_ATTR sk_interrupt_alarm_callback(
    gptimer_handle_t timer,
    const gptimer_alarm_event_data_t *event_data,
    void *opaque
) {
    (void)timer;
    (void)event_data;
    SkInterrupt *interrupt = (SkInterrupt*)opaque;
    if (interrupt && interrupt->handler) interrupt->handler(interrupt->context);
    return false;
}

static SkInterrupt* sk_interrupt_periodic(int64_t period_ns) {
    if (period_ns <= 0) sk_interrupt_panic("periodic interrupt duration must be positive");
    SkInterrupt *interrupt = (SkInterrupt*)calloc(1, sizeof(SkInterrupt));
    if (!interrupt) sk_interrupt_panic("interrupt allocation failed");
    interrupt->period_ns = period_ns;
    gptimer_config_t timer_config = {
        .clk_src = GPTIMER_CLK_SRC_DEFAULT,
        .direction = GPTIMER_COUNT_UP,
        .resolution_hz = 1000000,
    };
    if (gptimer_new_timer(&timer_config, &interrupt->timer) != ESP_OK) {
        free(interrupt);
        sk_interrupt_panic("hardware timer creation failed");
    }
    return interrupt;
}

static void sk_interrupt_bind(SkInterrupt *interrupt, SkInterruptHandler handler, void *context) {
    if (!interrupt || !handler || interrupt->started) sk_interrupt_panic("invalid or duplicate interrupt binding");
    interrupt->handler = handler;
    interrupt->context = context;
    gptimer_event_callbacks_t callbacks = {
        .on_alarm = sk_interrupt_alarm_callback,
    };
    if (gptimer_register_event_callbacks(interrupt->timer, &callbacks, interrupt) != ESP_OK)
        sk_interrupt_panic("hardware timer callback registration failed");
    uint64_t period_us = ((uint64_t)interrupt->period_ns + 999ULL) / 1000ULL;
    if (period_us == 0) period_us = 1;
    gptimer_alarm_config_t alarm = {
        .alarm_count = period_us,
        .reload_count = 0,
        .flags.auto_reload_on_alarm = true,
    };
    if (gptimer_set_alarm_action(interrupt->timer, &alarm) != ESP_OK)
        sk_interrupt_panic("hardware timer alarm configuration failed");
    if (gptimer_enable(interrupt->timer) != ESP_OK || gptimer_start(interrupt->timer) != ESP_OK)
        sk_interrupt_panic("hardware timer start failed");
    interrupt->started = true;
}

static void sk_interrupt_destroy(SkInterrupt *interrupt) {
    if (!interrupt) return;
    if (interrupt->started) {
        if (gptimer_stop(interrupt->timer) != ESP_OK)
            sk_interrupt_panic("hardware timer stop failed");
        if (gptimer_disable(interrupt->timer) != ESP_OK)
            sk_interrupt_panic("hardware timer disable failed");
    }
    if (gptimer_del_timer(interrupt->timer) != ESP_OK)
        sk_interrupt_panic("hardware timer destroy failed");
    free(interrupt->context);
    free(interrupt);
}

static SkInterrupt* sk_interrupt_move(SkInterrupt **source) {
    if (!source) return NULL;
    SkInterrupt *result = *source;
    *source = NULL;
    return result;
}

typedef union {
    long double long_double_value;
    void *pointer_value;
    int64_t integer_value;
} SkMemoryAlignment;

typedef struct SkMemoryChunk {
    unsigned char *buffer;
    size_t capacity;
    size_t offset;
    bool owns_buffer;
    struct SkMemoryChunk *next;
} SkMemoryChunk;

typedef struct SkMemoryRegion {
    SkMemoryChunk first;
    SkMemoryChunk *current;
    bool failed;
    bool allow_grow;
    bool allow_drop;
} SkMemoryRegion;

typedef struct SkAllocHeader {
    uint32_t magic;
    SkMemoryRegion *owner_region;
    size_t size;
    SkMemoryAlignment alignment;
} SkAllocHeader;

#define SK_ALLOC_MAGIC 0x534B4144u

static SK_THREAD_LOCAL SkMemoryRegion *sk_active_region = NULL;

static size_t sk_mem_align_up(size_t value, size_t alignment) {
    size_t rem = value % alignment;
    return rem == 0 ? value : (value + (alignment - rem));
}

static SkAllocHeader* sk_header_from_ptr(const void *ptr) {
    if (!ptr) return NULL;
    SkAllocHeader *header = ((SkAllocHeader*)ptr) - 1;
    if (header->magic != SK_ALLOC_MAGIC) return NULL;
    return header;
}

static SkMemoryRegion* sk_mem_set_active(SkMemoryRegion *region) {
    SkMemoryRegion *previous = sk_active_region;
    sk_active_region = region;
    return previous;
}

static SkMemoryRegion* sk_mem_current(void) {
    return sk_active_region;
}

static void sk_mem_clear_failure(SkMemoryRegion *region) {
    if (region) region->failed = false;
}

static bool sk_mem_failed(SkMemoryRegion *region) {
    return region && region->failed;
}

static void sk_mem_panic(const char *message) {
    fprintf(stderr, "Skadi memory runtime error: %s\n", message ? message : "unknown");
    exit(1);
}

static bool sk_mem_region_init_external(
    SkMemoryRegion *region,
    unsigned char *buffer,
    size_t capacity,
    bool owns_buffer,
    bool allow_grow,
    bool allow_drop
) {
    if (!region || !buffer || capacity == 0) return false;
    memset(region, 0, sizeof(*region));
    region->first.buffer = buffer;
    region->first.capacity = capacity;
    region->first.owns_buffer = owns_buffer;
    region->current = &region->first;
    region->allow_grow = allow_grow;
    region->allow_drop = allow_drop;
    return true;
}

static bool sk_mem_region_init(
    SkMemoryRegion *region,
    size_t capacity,
    bool allow_grow,
    bool allow_drop
) {
    if (!region || capacity == 0) return false;
    unsigned char *buffer = (unsigned char*)malloc(capacity);
    if (!buffer) return false;
    return sk_mem_region_init_external(
        region, buffer, capacity, true, allow_grow, allow_drop
    );
}

static void sk_mem_region_release_growth(SkMemoryRegion *region) {
    if (!region) return;
    SkMemoryChunk *chunk = region->first.next;
    while (chunk) {
        SkMemoryChunk *next = chunk->next;
        if (chunk->owns_buffer) free(chunk->buffer);
        free(chunk);
        chunk = next;
    }
    region->first.next = NULL;
    region->current = &region->first;
}

static void sk_mem_region_clear(SkMemoryRegion *region) {
    if (!region) return;
    sk_mem_region_release_growth(region);
    region->first.offset = 0;
    region->failed = false;
}

static void sk_mem_region_destroy(SkMemoryRegion *region) {
    if (!region) return;
    if (sk_active_region == region) sk_active_region = NULL;
    sk_mem_region_release_growth(region);
    if (region->first.owns_buffer) free(region->first.buffer);
    memset(region, 0, sizeof(*region));
}

static SkMemoryChunk* sk_mem_grow(SkMemoryRegion *region, size_t minimum) {
    if (!region || !region->allow_grow) return NULL;
    size_t capacity = region->first.capacity;
    if (capacity < minimum) capacity = minimum;
    while (capacity < minimum || capacity < region->current->capacity * 2) {
        if (capacity > SIZE_MAX / 2) {
            capacity = minimum;
            break;
        }
        capacity *= 2;
    }
    SkMemoryChunk *chunk = (SkMemoryChunk*)calloc(1, sizeof(*chunk));
    if (!chunk) return NULL;
    chunk->buffer = (unsigned char*)malloc(capacity);
    if (!chunk->buffer) {
        free(chunk);
        return NULL;
    }
    chunk->capacity = capacity;
    chunk->owns_buffer = true;
    region->current->next = chunk;
    region->current = chunk;
    return chunk;
}

static void* sk_alloc_bytes_in(SkMemoryRegion *region, size_t size) {
    if (size > SIZE_MAX - sizeof(SkAllocHeader)) return NULL;
    size_t total = sizeof(SkAllocHeader) + size;
    if (region) {
        SkMemoryChunk *chunk = region->current;
        size_t start = sk_mem_align_up(chunk->offset, sizeof(SkMemoryAlignment));
        if (!chunk->buffer || start > chunk->capacity || total > chunk->capacity - start) {
            chunk = sk_mem_grow(region, total + sizeof(SkMemoryAlignment));
            if (!chunk) {
                region->failed = true;
                return NULL;
            }
            start = 0;
        }
        SkAllocHeader *header = (SkAllocHeader*)(chunk->buffer + start);
        header->magic = SK_ALLOC_MAGIC;
        header->owner_region = region;
        header->size = size;
        chunk->offset = start + total;
        return (void*)(header + 1);
    }
    SkAllocHeader *header = (SkAllocHeader*)malloc(total);
    if (!header) return NULL;
    header->magic = SK_ALLOC_MAGIC;
    header->owner_region = NULL;
    header->size = size;
    return (void*)(header + 1);
}

static bool sk_mem_region_init_child(
    SkMemoryRegion *child,
    SkMemoryRegion *parent,
    size_t capacity,
    bool allow_drop
) {
    if (!child || !parent || capacity == 0) return false;
    unsigned char *buffer = (unsigned char*)sk_alloc_bytes_in(parent, capacity);
    if (!buffer) return false;
    return sk_mem_region_init_external(
        child, buffer, capacity, false, false, allow_drop
    );
}

static void* sk_alloc_bytes(size_t size) {
    return sk_alloc_bytes_in(sk_mem_current(), size);
}

static char* sk_text_alloc(size_t size) {
    return (char*)sk_alloc_bytes(size + 1);
}

static char* sk_text_dup(const char *s) {
    const char *src = s ? s : "";
    size_t n = strlen(src);
    char *out = sk_text_alloc(n);
    if (!out) return NULL;
    memcpy(out, src, n);
    out[n] = '\0';
    return out;
}

static void sk_free_text(void *ptr) {
    SkAllocHeader *header = sk_header_from_ptr(ptr);
    if (!header) return;
    if (header->owner_region) return;
    free(header);
}

typedef enum { FileMode_Read, FileMode_Write, FileMode_Append, FileMode_ReadWrite } SkFileMode;
typedef struct { FILE *handle; SkFileMode mode; } SkFile;

static int sk_file_open(const char *path, SkFileMode mode, SkFile *out_file) {
    if (!path || !out_file) return 1;
    out_file->handle = NULL;
    out_file->mode = mode;
    const char *native_mode = mode == FileMode_Read ? "rb" : mode == FileMode_Write ? "wb" : mode == FileMode_Append ? "ab" : "r+b";
    out_file->handle = fopen(path, native_mode);
    return out_file->handle ? 0 : 1;
}

static int sk_file_read_all(SkFile *file, const char **out_text) {
    if (!file || !file->handle || !out_text || (file->mode != FileMode_Read && file->mode != FileMode_ReadWrite)) return 1;
    if (fseek(file->handle, 0, SEEK_END) != 0) return 1;
    long length = ftell(file->handle);
    if (length < 0 || fseek(file->handle, 0, SEEK_SET) != 0) return 1;
    char *buffer = sk_text_alloc((size_t)length);
    if (!buffer) return 1;
    size_t read_count = fread(buffer, 1, (size_t)length, file->handle);
    if (read_count != (size_t)length && ferror(file->handle)) { sk_free_text(buffer); return 1; }
    buffer[read_count] = '\0';
    *out_text = buffer;
    return 0;
}

static int sk_file_write(SkFile *file, const char *text) {
    if (!file || !file->handle || (file->mode != FileMode_Write && file->mode != FileMode_Append && file->mode != FileMode_ReadWrite)) return 1;
    const char *data = text ? text : "";
    size_t length = strlen(data);
    return fwrite(data, 1, length, file->handle) == length && fflush(file->handle) == 0 ? 0 : 1;
}

static int sk_file_close(SkFile *file) {
    if (!file || !file->handle) return 1;
    FILE *handle = file->handle;
    file->handle = NULL;
    return fclose(handle) == 0 ? 0 : 1;
}

static void sk_file_destroy(SkFile *file) { if (file && file->handle) { fclose(file->handle); file->handle = NULL; } }
static SkFile sk_file_move(SkFile *source) { SkFile moved = *source; source->handle = NULL; return moved; }

static int sk_output_text(const char *s) { printf("%s\n", s ? s : ""); return 0; }
static int sk_output_int(int64_t v) { printf("%lld\n", (long long)v); return 0; }
static int sk_output_float(double v) { printf("%f\n", v); return 0; }
static int sk_output_bool(bool v) { printf("%s\n", v ? "true" : "false"); return 0; }
static int sk_output_char(char v) { printf("%c\n", v); return 0; }

static int sk_output_text_part(const char *s) { printf("%s", s ? s : ""); return 0; }
static int sk_output_int_part(int64_t v) { printf("%lld", (long long)v); return 0; }
static int sk_output_float_part(double v) { printf("%f", v); return 0; }
static int sk_output_bool_part(bool v) { printf("%s", v ? "true" : "false"); return 0; }
static int sk_output_char_part(char v) { printf("%c", v); return 0; }
static int sk_output_end(void) { putchar('\n'); return 0; }

static char* sk_input(const char *prompt) {
    if (prompt) printf("%s", prompt);
    char buf[4096];
    if (!fgets(buf, sizeof(buf), stdin)) return sk_text_dup("");
    size_t n = strlen(buf);
    if (n > 0 && buf[n - 1] == '\n') buf[n - 1] = '\0';
    return sk_text_dup(buf);
}

static char* sk_read_file(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return sk_text_dup("");
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (n < 0) { fclose(f); return sk_text_dup(""); }
    char *buf = sk_text_alloc((size_t)n);
    if (!buf) { fclose(f); return NULL; }
    size_t r = fread(buf, 1, (size_t)n, f);
    buf[r] = '\0';
    fclose(f);
    return buf;
}

static int sk_write_file(const char *path, const char *data) {
    FILE *f = fopen(path, "wb");
    if (!f) return 1;
    size_t n = data ? strlen(data) : 0;
    size_t w = fwrite(data ? data : "", 1, n, f);
    fclose(f);
    return w == n ? 0 : 1;
}

static int64_t sk_channel_send_i8(SkChannel *channel, int8_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_i8(SkChannel *channel, int8_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_i8(SkChannel *channel, int8_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_i8(SkChannel *channel, int8_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_i8(SkChannel *channel, int8_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_i8(SkChannel *channel, int8_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static int8_t sk_channel_receive_i8(SkChannel *channel) {
    int8_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_i16(SkChannel *channel, int16_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_i16(SkChannel *channel, int16_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_i16(SkChannel *channel, int16_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_i16(SkChannel *channel, int16_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_i16(SkChannel *channel, int16_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_i16(SkChannel *channel, int16_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static int16_t sk_channel_receive_i16(SkChannel *channel) {
    int16_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_i32(SkChannel *channel, int32_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_i32(SkChannel *channel, int32_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_i32(SkChannel *channel, int32_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_i32(SkChannel *channel, int32_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_i32(SkChannel *channel, int32_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_i32(SkChannel *channel, int32_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static int32_t sk_channel_receive_i32(SkChannel *channel) {
    int32_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_i64(SkChannel *channel, int64_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_i64(SkChannel *channel, int64_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_i64(SkChannel *channel, int64_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_i64(SkChannel *channel, int64_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_i64(SkChannel *channel, int64_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_i64(SkChannel *channel, int64_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static int64_t sk_channel_receive_i64(SkChannel *channel) {
    int64_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Int(SkChannel *channel, SkInt value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Int(SkChannel *channel, SkInt value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Int(SkChannel *channel, SkInt value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Int(SkChannel *channel, SkInt value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Int(SkChannel *channel, SkInt *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Int(SkChannel *channel, SkInt *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static SkInt sk_channel_receive_Int(SkChannel *channel) {
    SkInt value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_u8(SkChannel *channel, uint8_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_u8(SkChannel *channel, uint8_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_u8(SkChannel *channel, uint8_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_u8(SkChannel *channel, uint8_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_u8(SkChannel *channel, uint8_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_u8(SkChannel *channel, uint8_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static uint8_t sk_channel_receive_u8(SkChannel *channel) {
    uint8_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_u16(SkChannel *channel, uint16_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_u16(SkChannel *channel, uint16_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_u16(SkChannel *channel, uint16_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_u16(SkChannel *channel, uint16_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_u16(SkChannel *channel, uint16_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_u16(SkChannel *channel, uint16_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static uint16_t sk_channel_receive_u16(SkChannel *channel) {
    uint16_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_u32(SkChannel *channel, uint32_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_u32(SkChannel *channel, uint32_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_u32(SkChannel *channel, uint32_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_u32(SkChannel *channel, uint32_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_u32(SkChannel *channel, uint32_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_u32(SkChannel *channel, uint32_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static uint32_t sk_channel_receive_u32(SkChannel *channel) {
    uint32_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_u64(SkChannel *channel, uint64_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_u64(SkChannel *channel, uint64_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_u64(SkChannel *channel, uint64_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_u64(SkChannel *channel, uint64_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_u64(SkChannel *channel, uint64_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_u64(SkChannel *channel, uint64_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static uint64_t sk_channel_receive_u64(SkChannel *channel) {
    uint64_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_f32(SkChannel *channel, float value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_f32(SkChannel *channel, float value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_f32(SkChannel *channel, float value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_f32(SkChannel *channel, float value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_f32(SkChannel *channel, float *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_f32(SkChannel *channel, float *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static float sk_channel_receive_f32(SkChannel *channel) {
    float value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_f64(SkChannel *channel, double value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_f64(SkChannel *channel, double value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_f64(SkChannel *channel, double value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_f64(SkChannel *channel, double value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_f64(SkChannel *channel, double *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_f64(SkChannel *channel, double *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static double sk_channel_receive_f64(SkChannel *channel) {
    double value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Float(SkChannel *channel, float value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Float(SkChannel *channel, float value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Float(SkChannel *channel, float value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Float(SkChannel *channel, float value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Float(SkChannel *channel, float *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Float(SkChannel *channel, float *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static float sk_channel_receive_Float(SkChannel *channel) {
    float value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_bool(SkChannel *channel, bool value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_bool(SkChannel *channel, bool value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_bool(SkChannel *channel, bool value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_bool(SkChannel *channel, bool value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_bool(SkChannel *channel, bool *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_bool(SkChannel *channel, bool *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_receive_bool(SkChannel *channel) {
    bool value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Bool(SkChannel *channel, bool value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Bool(SkChannel *channel, bool value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Bool(SkChannel *channel, bool value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Bool(SkChannel *channel, bool value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Bool(SkChannel *channel, bool *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Bool(SkChannel *channel, bool *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_receive_Bool(SkChannel *channel) {
    bool value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_char(SkChannel *channel, char value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_char(SkChannel *channel, char value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_char(SkChannel *channel, char value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_char(SkChannel *channel, char value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_char(SkChannel *channel, char *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_char(SkChannel *channel, char *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static char sk_channel_receive_char(SkChannel *channel) {
    char value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Char(SkChannel *channel, char value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Char(SkChannel *channel, char value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Char(SkChannel *channel, char value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Char(SkChannel *channel, char value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Char(SkChannel *channel, char *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Char(SkChannel *channel, char *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static char sk_channel_receive_Char(SkChannel *channel) {
    char value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Text(SkChannel *channel, const char* value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Text(SkChannel *channel, const char* value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Text(SkChannel *channel, const char* value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Text(SkChannel *channel, const char* value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Text(SkChannel *channel, const char* *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Text(SkChannel *channel, const char* *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static const char* sk_channel_receive_Text(SkChannel *channel) {
    const char* value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Path(SkChannel *channel, const char* value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Path(SkChannel *channel, const char* value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Path(SkChannel *channel, const char* value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Path(SkChannel *channel, const char* value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Path(SkChannel *channel, const char* *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Path(SkChannel *channel, const char* *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static const char* sk_channel_receive_Path(SkChannel *channel) {
    const char* value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Time(SkChannel *channel, int64_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Time(SkChannel *channel, int64_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Time(SkChannel *channel, int64_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Time(SkChannel *channel, int64_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Time(SkChannel *channel, int64_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Time(SkChannel *channel, int64_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static int64_t sk_channel_receive_Time(SkChannel *channel) {
    int64_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Duration(SkChannel *channel, int64_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Duration(SkChannel *channel, int64_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Duration(SkChannel *channel, int64_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Duration(SkChannel *channel, int64_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Duration(SkChannel *channel, int64_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Duration(SkChannel *channel, int64_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static int64_t sk_channel_receive_Duration(SkChannel *channel) {
    int64_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_ByteSize(SkChannel *channel, int64_t value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_ByteSize(SkChannel *channel, int64_t value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_ByteSize(SkChannel *channel, int64_t value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_ByteSize(SkChannel *channel, int64_t value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_ByteSize(SkChannel *channel, int64_t *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_ByteSize(SkChannel *channel, int64_t *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static int64_t sk_channel_receive_ByteSize(SkChannel *channel) {
    int64_t value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Angle(SkChannel *channel, float value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Angle(SkChannel *channel, float value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Angle(SkChannel *channel, float value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Angle(SkChannel *channel, float value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Angle(SkChannel *channel, float *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Angle(SkChannel *channel, float *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static float sk_channel_receive_Angle(SkChannel *channel) {
    float value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Vec2(SkChannel *channel, Vec2 value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Vec2(SkChannel *channel, Vec2 value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Vec2(SkChannel *channel, Vec2 value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Vec2(SkChannel *channel, Vec2 value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Vec2(SkChannel *channel, Vec2 *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Vec2(SkChannel *channel, Vec2 *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static Vec2 sk_channel_receive_Vec2(SkChannel *channel) {
    Vec2 value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Vec3(SkChannel *channel, Vec3 value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Vec3(SkChannel *channel, Vec3 value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Vec3(SkChannel *channel, Vec3 value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Vec3(SkChannel *channel, Vec3 value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Vec3(SkChannel *channel, Vec3 *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Vec3(SkChannel *channel, Vec3 *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static Vec3 sk_channel_receive_Vec3(SkChannel *channel) {
    Vec3 value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

static int64_t sk_channel_send_Vec4(SkChannel *channel, Vec4 value) {
    return (int64_t)sk_channel_send_raw(channel, &value, -1);
}

static void sk_channel_send_or_panic_Vec4(SkChannel *channel, Vec4 value) {
    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel send requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "send on closed channel requires 'on error'");
}

static bool SK_INTERRUPT_ATTR sk_channel_try_send_Vec4(SkChannel *channel, Vec4 value) {
    return sk_channel_try_send_raw(channel, &value);
}

static int64_t sk_channel_send_for_Vec4(SkChannel *channel, Vec4 value, int64_t timeout_ns) {
    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);
}

static bool sk_channel_try_receive_Vec4(SkChannel *channel, Vec4 *out) {
    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;
}

static int64_t sk_channel_receive_for_Vec4(SkChannel *channel, Vec4 *out, int64_t timeout_ns) {
    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);
}

static Vec4 sk_channel_receive_Vec4(SkChannel *channel) {
    Vec4 value;
    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);
    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic("SC-RT-315", "cancelled channel receive requires 'on error'");
    if (status == SK_CHANNEL_CLOSED) sk_channel_panic("SC-RT-314", "receive on drained closed channel requires 'on error'");
    return value;
}

typedef struct SkInterruptContext_0 {
    SkChannel *ticks;
} SkInterruptContext_0;

static void SK_INTERRUPT_ATTR sk_interrupt_handler_0(void *opaque) {
    SkInterruptContext_0 *context = (SkInterruptContext_0*)opaque;
    SkChannel *ticks = context->ticks;
    /* SK-STMT@24:5#1 */
    sk_channel_try_send_Int(ticks, 1);
}

SkInt blink(SkChannel* ticks);

typedef struct {
    SkChannel* arg_0;
    SkInt result;
} SkTaskContext_blink;

static void sk_task_entry_blink(SkTask *task, void *raw_context) {
    (void)task;
    SkTaskContext_blink *context = (SkTaskContext_blink*)raw_context;
    context->result = blink(context->arg_0);
}

void board_init(void);
void board_set_led(bool enabled);

SkInt blink(SkChannel* ticks) {
    /* SK-STMT@5:5#1 */
    SkInt handled = 0;
    /* SK-STMT@6:5#1 */
    while ((handled < 6)) {
        /* SK-STMT@7:9#1 */
        SkInt signal = sk_channel_receive_Int(ticks);
        /* SK-STMT@8:9#1 */
        handled = sk_num_add_int(handled, signal);
        /* SK-STMT@9:9#1 */
        bool enabled = (sk_num_mod_int(handled, 2) == 1);
        /* SK-STMT@10:9#1 */
        board_set_led(enabled);
        /* SK-STMT@11:9#1 */
        (sk_output_text_part("embedded tick: "), sk_output_int_part(handled), sk_output_end());
    }
    /* SK-STMT@13:5#1 */
    board_set_led(false);
    /* SK-STMT@14:5#1 */
    return handled;
    return 0;
}

void app_main(void) {
    /* SK-STMT@17:1#1 */
    board_init();
    /* SK-STMT@19:1#1 */
    SkChannel *ticks = sk_channel_create(4, sizeof(SkInt));
    /* SK-STMT@20:1#1 */
    SkTask worker = {0};
    SkTaskContext_blink *worker_context = (SkTaskContext_blink*)malloc(sizeof(SkTaskContext_blink));
    if (!worker_context) sk_task_panic("SC-RT-301", "task context allocation failed");
    worker_context->arg_0 = ticks;
    if (!sk_task_start(&worker, sk_task_entry_blink, worker_context)) { free(worker_context); sk_task_panic("SC-RT-301", "native task creation failed"); }
    /* SK-STMT@21:1#1 */
    SkInterrupt* timer = sk_interrupt_periodic(500000000);
    /* SK-STMT@23:1#1 */
    SkInterruptContext_0 *sk_interrupt_context_0 = (SkInterruptContext_0*)calloc(1, sizeof(SkInterruptContext_0));
    if (!sk_interrupt_context_0) sk_interrupt_panic("interrupt context allocation failed");
    sk_interrupt_context_0->ticks = ticks;
    sk_interrupt_bind(timer, sk_interrupt_handler_0, sk_interrupt_context_0);
    /* SK-STMT@27:1#1 */
    sk_task_join(&worker);
    SkInt handled = ((SkTaskContext_blink*)worker.context)->result;
    sk_task_release_context(&worker);
    /* SK-STMT@28:1#1 */
    (sk_channel_close(ticks) ? 0 : 1);
    /* SK-STMT@29:1#1 */
    (sk_output_text_part("embedded smoke complete: "), sk_output_int_part(handled), sk_output_end());
    sk_interrupt_destroy(timer);
    sk_channel_destroy(ticks);
}
