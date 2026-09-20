#include "main.h"
#include "hatch.h"
#include "commands.h"

int main(void) {
    uint32_t now = 0;
    while (1) {
        commands_update(now);
        hatch_update(now);
        now += 10;
    }
}
