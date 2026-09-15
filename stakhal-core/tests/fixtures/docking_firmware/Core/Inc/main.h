#ifndef __MAIN_H
#define __MAIN_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stdbool.h>

#define Z1_LIMIT_Pin 1
#define Z1_LIMIT_GPIO_Port ((void*)0x40020000)
#define Z2_LIMIT_Pin 2
#define Z2_LIMIT_GPIO_Port ((void*)0x40020400)

typedef enum {
    IDLE,
    CALIBRATING,
    CAL_STOPPING,
    CAL_BACKOFF,
    GOING,
    HOLD,
    RETURNING,
    RETURNED,
    RECOVERY,
    REC_STOPPING,
    REC_BACKOFF,
    FAULT,
    OPENING,
    CLOSING
} SystemState;

#ifdef __cplusplus
}
#endif

#endif /* __MAIN_H */
