#include <stdio.h>
#include "yyjson.h"
int main(void) {
    printf("yyjson_version=%u\n", yyjson_version());
    printf("YYJSON_VERSION_HEX=%u\n", (unsigned)YYJSON_VERSION_HEX);
    return yyjson_version() == YYJSON_VERSION_HEX ? 0 : 1;
}
