#ifndef HATCH_H
#define HATCH_H

#include <stdint.h>
#include <stdbool.h>

void hatch_open(uint32_t now);
void hatch_close(uint32_t now);
void hatch_stop_cmd(void);

bool hatch_is_busy(void);

void hatch_update(uint32_t now);

void hatch_emergency_stop(void);

#endif /* HATCH_H */
