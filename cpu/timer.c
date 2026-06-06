#include "timer.h"
#include "isr.h"
#include "ports.h"
#include "../drivers/screen.h"
#include "../libc/string.h"
#include "../libc/function.h"

uint32_t tick = 0;

static void timer_callback(registers_t *regs) {
    tick++;
    UNUSED(regs);

    if (tick % 50 != 0) return;

    uint32_t uptime = tick / 50;
    char num[10] = "";
    int_to_ascii(uptime, num);

    char buf[12] = "UP:";
    int i = 3, j = 0;
    while (num[j]) buf[i++] = num[j++];
    buf[i++] = 's';
    buf[i] = '\0';

    screen_write_at(buf, 72, 0);
}

void init_timer(uint32_t freq) {
    register_interrupt_handler(IRQ0, timer_callback);

    uint32_t divisor = 1193180 / freq;
    uint8_t low  = (uint8_t)(divisor & 0xFF);
    uint8_t high = (uint8_t)((divisor >> 8) & 0xFF);
    port_byte_out(0x43, 0x36);
    port_byte_out(0x40, low);
    port_byte_out(0x40, high);
}
