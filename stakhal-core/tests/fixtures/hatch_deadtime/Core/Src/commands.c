#include "commands.h"
#include "hatch.h"
#include <string.h>
#include <stdio.h>
#include <stdbool.h>

static void cmd_open(uint32_t now) {
    if (hatch_is_busy()) printf("BUSY\r\n");
    else hatch_open(now);
}

static void cmd_close(uint32_t now) {
    if (hatch_is_busy()) printf("BUSY\r\n");
    else hatch_close(now);
}

static void cmd_stop(uint32_t now) {
    (void)now;
    hatch_stop_cmd();
}

typedef void (*CommandFn)(uint32_t now);
typedef struct { const char *name; CommandFn fn; } Command;

static const Command commandTable[] = {
    { "OPEN",  cmd_open  },
    { "CLOSE", cmd_close },
    { "STOP",  cmd_stop  },
};
#define NUM_COMMANDS (sizeof(commandTable) / sizeof(commandTable[0]))

void commands_update(uint32_t now) {
    (void)now;
}
