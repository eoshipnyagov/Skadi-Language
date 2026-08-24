#include <stdio.h>

#include <stdint.h>
#include <stdbool.h>

typedef int32_t SkInt;

SkInt add(SkInt a, SkInt b) {
    /* SK-STMT@2:5#1 */
    SkInt c = sk_num_add_int(a, b);
    return 0;
}

int main(void) {
    /* SK-STMT@4:1#1 */
    SkInt x = 3;
    return 0;
}
