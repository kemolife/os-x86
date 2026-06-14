static mut FREE_MEM_ADDR: u32 = 0;

pub unsafe fn mem_init(heap_start: u32) {
    FREE_MEM_ADDR = heap_start;
}

pub unsafe fn memory_copy(source: *const u8, dest: *mut u8, nbytes: usize) {
    for i in 0..nbytes {
        *dest.add(i) = *source.add(i);
    }
}

pub unsafe fn memory_set(dest: *mut u8, val: u8, len: u32) {
    for i in 0..len as usize {
        *dest.add(i) = val;
    }
}

pub unsafe fn kmalloc(size: u32, align: i32, phys_addr: *mut u32) -> u32 {
    if align == 1 && (FREE_MEM_ADDR & 0xFFFFF000 != 0) {
        FREE_MEM_ADDR &= 0xFFFFF000;
        FREE_MEM_ADDR += 0x1000;
    }
    if !phys_addr.is_null() {
        *phys_addr = FREE_MEM_ADDR;
    }
    let ret = FREE_MEM_ADDR;
    FREE_MEM_ADDR += size;
    ret
}
