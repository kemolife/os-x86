#ifndef SERIAL_H
#define SERIAL_H

#define COM1 0x3F8

void init_serial();
void serial_write(char c);
void serial_write_str(char *str);

#endif
