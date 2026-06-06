#include "serial.h"
#include "../cpu/ports.h"
#include "../cpu/isr.h"
#include "../libc/function.h"
#include <stdint.h>

static void serial_rx_handler(registers_t *regs) {
    char c = (char) port_byte_in(COM1);
    serial_write(c); /* echo back */
    UNUSED(regs);
}

void init_serial() {
    port_byte_out(COM1 + 1, 0x00); /* disable interrupts during init */
    port_byte_out(COM1 + 3, 0x80); /* enable DLAB to set baud divisor */
    port_byte_out(COM1 + 0, 0x03); /* divisor lo byte: 38400 baud */
    port_byte_out(COM1 + 1, 0x00); /* divisor hi byte */
    port_byte_out(COM1 + 3, 0x03); /* 8n1: 8 bits, no parity, 1 stop bit */
    port_byte_out(COM1 + 2, 0xC7); /* enable FIFO, clear, 14-byte threshold */
    port_byte_out(COM1 + 4, 0x0B); /* RTS/DSR set, IRQs enabled */
    port_byte_out(COM1 + 1, 0x01); /* enable received-data interrupt */
    register_interrupt_handler(IRQ4, serial_rx_handler);
}

static int tx_empty() {
    return port_byte_in(COM1 + 5) & 0x20; /* bit 5: transmitter holding register empty */
}

void serial_write(char c) {
    while (!tx_empty());
    port_byte_out(COM1, c);
}

void serial_write_str(char *str) {
    int i = 0;
    while (str[i]) {
        if (str[i] == '\n') serial_write('\r');
        serial_write(str[i++]);
    }
}
