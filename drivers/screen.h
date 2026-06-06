#ifndef SCREEN_H
#define SCREEN_H

#include <stdint.h>

typedef struct {
    uint32_t video_addr;
    int      max_rows;
    int      max_cols;
    char     default_attr;
    uint16_t ctrl_port;
    uint16_t data_port;
} screen_config_t;

#define SCREEN_VGA_DEFAULT ((screen_config_t){ 0xb8000, 25, 80, 0x0f, 0x3d4, 0x3d5 })

void screen_init(screen_config_t cfg);
void clear_screen();
void kprint_at(char *message, int col, int row);
void kprint(char *message);
void kprint_backspace();
void screen_write_at(char *msg, int col, int row); /* write without moving cursor */

#endif
