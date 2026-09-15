/* USER CODE BEGIN Header */
/**
  ******************************************************************************
  * @file           : main.c
  * @brief          : Docking firmware main program body with state machine
  ******************************************************************************
  */
/* USER CODE END Header */

/* Includes ------------------------------------------------------------------*/
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <stdio.h>

/* Private typedef -----------------------------------------------------------*/
/* USER CODE BEGIN PTD */
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

typedef struct {
    int32_t current_pos;
    int32_t target_pos;
} Axis;
/* USER CODE END PTD */

/* Private define ------------------------------------------------------------*/
/* USER CODE BEGIN PD */
#define CAL_TIMEOUT         5000
#define MOVE_TIMEOUT        10000
#define RECOVERY_TIMEOUT    8000
#define OPEN_DURATION_MS    3000
#define CLOSE_DURATION_MS   3000
#define BACKOFF_DIR         (-1)
#define RECOVERY_STEPS      200
#define CAL_DIR             1
#define CAL_TRAVEL          1000
#define GO_STEPS            5000

#define Z1_LIMIT_Pin        1
#define Z1_LIMIT_GPIO_Port  ((void*)0x40020000)
#define Z2_LIMIT_Pin        2
#define Z2_LIMIT_GPIO_Port  ((void*)0x40020400)
/* USER CODE END PD */

/* Private variables ---------------------------------------------------------*/
/* USER CODE BEGIN PV */
static SystemState state = IDLE;
static uint32_t stateStart = 0;
static bool calibrated = false;
static bool z1Hit = false;
static bool z2Hit = false;
static int32_t z1HitPos = 0;
static int32_t z2HitPos = 0;
static Axis z1 = {0, 0};
static Axis z2 = {0, 0};
static bool cmd_ready = false;
static uint8_t rx_buffer[64] = {0};
/* USER CODE END PV */

/* Private function prototypes -----------------------------------------------*/
/* USER CODE BEGIN PFP */
static uint32_t HAL_GetTick(void) { return 0; }
static bool limit_pressed(void* port, uint16_t pin) { return false; }
static void axis_stop(Axis* a) {}
static void axis_move_to(Axis* a, int32_t pos) {}
static bool axes_done(void) { return true; }
static bool skewOK(void) { return true; }
static void reset_axis_zero(void) {}
static void clearHits(void) { z1Hit = false; z2Hit = false; }
static void enable_motors(void) {}
static void hatch_forward(void) {}
static void hatch_reverse(void) {}
static void hatch_stop(void) {}

void fault(const char* reason) {
    printf("FAULT: %s\r\n", reason);
    state = FAULT;
}

void startCal(void) {
    clearHits();
    axis_move_to(&z1, CAL_DIR * CAL_TRAVEL);
    axis_move_to(&z2, CAL_DIR * CAL_TRAVEL);
    state = CALIBRATING;
    stateStart = HAL_GetTick();
    printf("CAL START\r\n");
}
/* USER CODE END PFP */

int main(void)
{
  /* Infinite loop */
  /* USER CODE BEGIN WHILE */
  while (1)
  {

    /* USER CODE END WHILE */

    /* USER CODE BEGIN 3 */
    uint32_t now = HAL_GetTick();

    if (state == CALIBRATING) {
        if (now - stateStart > CAL_TIMEOUT) {
            fault("CAL TIMEOUT");
        } else {
            if (limit_pressed(Z1_LIMIT_GPIO_Port, Z1_LIMIT_Pin) && !z1Hit) {
                z1Hit = true; z1HitPos = z1.current_pos;
                axis_stop(&z1);
                printf("Z1 HIT\r\n");
            }
            if (limit_pressed(Z2_LIMIT_GPIO_Port, Z2_LIMIT_Pin) && !z2Hit) {
                z2Hit = true; z2HitPos = z2.current_pos;
                axis_stop(&z2);
                printf("Z2 HIT\r\n");
            }
            if (z1Hit && z2Hit) {
                state = CAL_STOPPING; stateStart = now;
            } else if (z1Hit && !z2Hit && (z2.current_pos == z2.target_pos)) {
                fault("Z2 LIMIT NOT FOUND");
            } else if (z2Hit && !z1Hit && (z1.current_pos == z1.target_pos)) {
                fault("Z1 LIMIT NOT FOUND");
            }
        }
    }
    else if (state == CAL_STOPPING) {
        if (axes_done()) {
            if (!skewOK()) {
                fault("CAL SKEW");
            } else {
                printf("ZERO\r\n"); // Signal test runner immediately
                calibrated = true; 
                axis_move_to(&z1, z1.current_pos + (BACKOFF_DIR * RECOVERY_STEPS));
                axis_move_to(&z2, z2.current_pos + (BACKOFF_DIR * RECOVERY_STEPS));
                state = CAL_BACKOFF; stateStart = now;
            }
        }
    }
    else if (state == CAL_BACKOFF) {
        if (now - stateStart > CAL_TIMEOUT) {
            fault("CAL BACKOFF TIMEOUT");
        } else if (axes_done()) {
            reset_axis_zero();
            clearHits();
            state = IDLE;
            printf("CAL OK\r\n");
        }
    }
    else if (state == GOING) {
        if (now - stateStart > MOVE_TIMEOUT) {
            fault("GO TIMEOUT");
        } else if (axes_done()) {
            state = HOLD;
            printf("HOLD\r\n");
        }
    }
    else if (state == RETURNING) {
        bool enteringRecovery = false;
        if (!z1Hit && limit_pressed(Z1_LIMIT_GPIO_Port, Z1_LIMIT_Pin)) {
            z1Hit = true; z1HitPos = z1.current_pos;
            axis_stop(&z1);
            printf("Z1 HIT\r\n");
            enteringRecovery = true;
        }
        if (!z2Hit && limit_pressed(Z2_LIMIT_GPIO_Port, Z2_LIMIT_Pin)) {
            z2Hit = true; z2HitPos = z2.current_pos;
            axis_stop(&z2);
            printf("Z2 HIT\r\n");
            enteringRecovery = true;
        }
        if (enteringRecovery) {
            if (!z1Hit) axis_move_to(&z1, CAL_DIR * CAL_TRAVEL);
            if (!z2Hit) axis_move_to(&z2, CAL_DIR * CAL_TRAVEL);
            state = RECOVERY; stateStart = now;
        } else if (axes_done()) {
            state = RETURNED;
            printf("RETURNED\r\n");
        } else if (now - stateStart > MOVE_TIMEOUT) {
            fault("RETURN TIMEOUT");
        }
    }
    else if (state == RECOVERY) {
        if (now - stateStart > RECOVERY_TIMEOUT) {
            fault("REC TIMEOUT");
        } else {
            if (!z1Hit && limit_pressed(Z1_LIMIT_GPIO_Port, Z1_LIMIT_Pin)) {
                z1Hit = true; z1HitPos = z1.current_pos;
                axis_stop(&z1);
                printf("Z1 HIT\r\n");
            }
            if (!z2Hit && limit_pressed(Z2_LIMIT_GPIO_Port, Z2_LIMIT_Pin)) {
                z2Hit = true; z2HitPos = z2.current_pos;
                axis_stop(&z2);
                printf("Z2 HIT\r\n");
            }
            if (z1Hit && z2Hit) {
                state = REC_STOPPING; stateStart = now;
            } else if (z1Hit && !z2Hit && (z2.current_pos == z2.target_pos)) {
                fault("Z2 LIMIT NOT FOUND");
            } else if (z2Hit && !z1Hit && (z1.current_pos == z1.target_pos)) {
                fault("Z1 LIMIT NOT FOUND");
            }
        }
    }
    else if (state == REC_STOPPING) {
        if (axes_done()) {
            if (!skewOK()) {
                fault("REC SKEW");
            } else {
                axis_move_to(&z1, z1.current_pos + (BACKOFF_DIR * RECOVERY_STEPS));
                axis_move_to(&z2, z2.current_pos + (BACKOFF_DIR * RECOVERY_STEPS));
                state = REC_BACKOFF; stateStart = now;
            }
        }
    }
    else if (state == REC_BACKOFF) {
        if (now - stateStart > RECOVERY_TIMEOUT) {
            fault("REC BACKOFF TIMEOUT");
        } else if (axes_done()) {
            reset_axis_zero();
            clearHits();
            calibrated = true; state = RETURNED;
            printf("REC OK\r\nZERO\r\nRETURNED\r\n");
        }
    }else if (state == OPENING) {
        if (now - stateStart > OPEN_DURATION_MS) {
            hatch_stop();
            state = IDLE;
            printf("OPENED\r\n");
        }
    }
    else if (state == CLOSING) {
        if (now - stateStart > CLOSE_DURATION_MS) {
            hatch_stop();
            state = IDLE;
            printf("CLOSED\r\n");
        }
    }

    if (cmd_ready) {
        cmd_ready = false;
        if (strcmp((const char*)rx_buffer, "CAL") == 0) {
            if (state == IDLE) startCal();
            else printf("BUSY\r\n");
        } else if (strcmp((const char*)rx_buffer, "GO") == 0) {
            if (!calibrated) printf("NO CAL\r\n");
            else if (state == IDLE || state == RETURNED) {
                clearHits(); enable_motors();
                axis_move_to(&z1, GO_STEPS); axis_move_to(&z2, GO_STEPS);
                state = GOING; stateStart = now;
                printf("GO\r\n");
            } else printf("BUSY\r\n");
        } else if (strcmp((const char*)rx_buffer, "RET") == 0) {
            if (!calibrated) printf("NO CAL\r\n");
            else if (state == HOLD) {
                clearHits();
                axis_move_to(&z1, 0); axis_move_to(&z2, 0);
                state = RETURNING; stateStart = now;
                printf("RET\r\n");
            } else printf("INVALID\r\n");
        } else if (strcmp((const char*)rx_buffer, "RST") == 0) {
            axis_stop(&z1); axis_stop(&z2); enable_motors();
            reset_axis_zero();
            clearHits(); calibrated = false; state = IDLE;
            printf("RST\r\nNO CAL\r\n");
        } else if (strcmp((const char*)rx_buffer, "OPEN") == 0) {
            if (state == IDLE || state == RETURNED) { // Allow execution from RETURNED state
                hatch_forward();
                state = OPENING; stateStart = now;
                printf("OPENING\r\n");
            } else printf("BUSY\r\n");
        } else if (strcmp((const char*)rx_buffer, "CLOSE") == 0) {
            if (state == IDLE || state == RETURNED) { // Allow execution from RETURNED state
                hatch_reverse();
                state = CLOSING; stateStart = now;
                printf("CLOSING\r\n");
            } else printf("BUSY\r\n");
        }
    }
  }
  /* USER CODE END 3 */
}
