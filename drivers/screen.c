#include "screen.h"
#include "../cpu/ports.h"
#include "../libc/mem.h"
#include <stdint.h>

#define RED_ON_WHITE 0xf4

static screen_config_t cfg;

/* Private functions */
int get_screen_offset();
void set_screen_offset(int offset);
int print_char(char c, int col, int row, char attr);
int get_offset(int col, int row);
int get_offset_row(int offset);
int get_offset_col(int offset);

void screen_init(screen_config_t c) {
    cfg = c;
}

void kprint_at(char *message, int col, int row) {
    int offset;
    if (col >= 0 && row >= 0)
        offset = get_offset(col, row);
    else {
        offset = get_screen_offset();
        row = get_offset_row(offset);
        col = get_offset_col(offset);
    }

    int i = 0;
    while (message[i] != 0) {
        offset = print_char(message[i++], col, row, cfg.default_attr);
        row = get_offset_row(offset);
        col = get_offset_col(offset);
    }
}

void kprint(char *message) {
    kprint_at(message, -1, -1);
}

void kprint_backspace() {
    int offset = get_screen_offset()-2;
    int row = get_offset_row(offset);
    int col = get_offset_col(offset);
    print_char(0x08, col, row, cfg.default_attr);
}

int print_char(char c, int col, int row, char attr) {
    uint8_t *vidmem = (uint8_t*) cfg.video_addr;
    if (!attr) attr = cfg.default_attr;

    if (col >= cfg.max_cols || row >= cfg.max_rows) {
        vidmem[2*(cfg.max_cols)*(cfg.max_rows)-2] = 'E';
        vidmem[2*(cfg.max_cols)*(cfg.max_rows)-1] = RED_ON_WHITE;
        return get_offset(col, row);
    }

    int offset;
    if (col >= 0 && row >= 0) offset = get_offset(col, row);
    else offset = get_screen_offset();

    if (c == '\n') {
        row = get_offset_row(offset);
        offset = get_offset(0, row+1);
    } else if (c == 0x08) {
        vidmem[offset] = ' ';
        vidmem[offset+1] = attr;
    } else {
        vidmem[offset] = c;
        vidmem[offset+1] = attr;
        offset += 2;
    }

    if (offset >= cfg.max_rows * cfg.max_cols * 2) {
        int i;
        for (i = 1; i < cfg.max_rows; i++)
            memory_copy((uint8_t*)(get_offset(0, i) + cfg.video_addr),
                        (uint8_t*)(get_offset(0, i-1) + cfg.video_addr),
                        cfg.max_cols * 2);

        char *last_line = (char*) (get_offset(0, cfg.max_rows-1) + (uint8_t*) cfg.video_addr);
        for (i = 0; i < cfg.max_cols * 2; i++) last_line[i] = 0;

        offset -= 2 * cfg.max_cols;
    }

    set_screen_offset(offset);
    return offset;
}

int get_screen_offset() {
    port_byte_out(cfg.ctrl_port, 14);
    int offset = port_byte_in(cfg.data_port) << 8;
    port_byte_out(cfg.ctrl_port, 15);
    offset += port_byte_in(cfg.data_port);
    return offset * 2;
}

void set_screen_offset(int offset) {
    offset /= 2;
    port_byte_out(cfg.ctrl_port, 14);
    port_byte_out(cfg.data_port, (unsigned char)(offset >> 8));
    port_byte_out(cfg.ctrl_port, 15);
    port_byte_out(cfg.data_port, (unsigned char)(offset & 0xff));
}

void clear_screen() {
    int screen_size = cfg.max_cols * cfg.max_rows;
    int i;
    char *screen = (char*) cfg.video_addr;

    for (i = 0; i < screen_size; i++) {
        screen[i*2] = ' ';
        screen[i*2+1] = cfg.default_attr;
    }
    set_screen_offset(get_offset(0, 0));
}

int get_offset(int col, int row) { return 2 * (row * cfg.max_cols + col); }
int get_offset_row(int offset) { return offset / (2 * cfg.max_cols); }
int get_offset_col(int offset) { return (offset - (get_offset_row(offset)*2*cfg.max_cols))/2; }

void screen_write_at(char *msg, int col, int row) {
    uint8_t *vidmem = (uint8_t*) cfg.video_addr;
    int i = 0;
    while (msg[i] != 0) {
        int offset = get_offset(col + i, row);
        vidmem[offset]   = msg[i];
        vidmem[offset+1] = cfg.default_attr;
        i++;
    }
}
