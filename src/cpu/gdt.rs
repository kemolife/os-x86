//! Runtime GDT (Global Descriptor Table) with kernel + user segments and a TSS.
//!
//! The boot GDT only had ring-0 code/data. To run user code we need ring-3
//! (DPL 3) code/data descriptors and a **TSS** (Task State Segment): when an
//! interrupt occurs while in ring 3, the CPU reads the ring-0 stack pointer
//! (`esp0`) from the TSS and switches to it before running the handler.
//!
//! Selectors (kernel ones keep the boot values so the switch is seamless):
//!   0x08 kernel code   0x10 kernel data
//!   0x18 user code     0x20 user data     0x28 TSS

const KERNEL_STACK_SIZE: usize = 16 * 1024;

pub const KERNEL_CODE: u16 = 0x08;
pub const KERNEL_DATA: u16 = 0x10;
pub const USER_CODE: u16 = 0x18;
pub const USER_DATA: u16 = 0x20;
pub const TSS_SEL: u16 = 0x28;

#[repr(C, packed)]
struct Tss {
    prev: u32,
    esp0: u32, // ring-0 stack pointer loaded on a ring3->ring0 trap
    ss0: u32,  // ring-0 stack segment
    _unused: [u32; 23],
}

static mut GDT: [u64; 6] = [0; 6];
static mut TSS: Tss = Tss { prev: 0, esp0: 0, ss0: 0, _unused: [0; 23] };
static mut KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

#[repr(C, packed)]
struct GdtPtr {
    limit: u16,
    base: u32,
}
static mut GDT_PTR: GdtPtr = GdtPtr { limit: 0, base: 0 };

fn entry(base: u32, limit: u32, access: u8, gran: u8) -> u64 {
    (limit as u64 & 0xFFFF)
        | ((base as u64 & 0xFF_FFFF) << 16)
        | ((access as u64) << 40)
        | (((limit as u64 >> 16) & 0xF) << 48)
        | ((gran as u64 & 0xF) << 52)
        | (((base as u64 >> 24) & 0xFF) << 56)
}

pub unsafe fn init() {
    let gdt = &raw mut GDT;
    (*gdt)[0] = 0;
    (*gdt)[1] = entry(0, 0xFFFFF, 0x9A, 0xC); // kernel code (ring 0)
    (*gdt)[2] = entry(0, 0xFFFFF, 0x92, 0xC); // kernel data (ring 0)
    (*gdt)[3] = entry(0, 0xFFFFF, 0xFA, 0xC); // user code   (ring 3, DPL=3)
    (*gdt)[4] = entry(0, 0xFFFFF, 0xF2, 0xC); // user data   (ring 3, DPL=3)

    // TSS descriptor: base = &TSS, limit = sizeof-1, access 0x89 (present,
    // 32-bit available TSS), byte granularity.
    let tss_base = (&raw const TSS) as u32;
    let tss_limit = (core::mem::size_of::<Tss>() - 1) as u32;
    (*gdt)[5] = entry(tss_base, tss_limit, 0x89, 0x0);

    (*tss_ptr()).ss0 = KERNEL_DATA as u32;
    (*tss_ptr()).esp0 = (&raw const KERNEL_STACK as u32) + KERNEL_STACK_SIZE as u32;

    let ptr = &raw mut GDT_PTR;
    (*ptr).limit = (core::mem::size_of::<[u64; 6]>() - 1) as u16;
    (*ptr).base = gdt as u32;

    core::arch::asm!("lgdt [{}]", in(reg) ptr, options(nostack));

    // Reload the data segment registers, then far-jump to reload CS, then load
    // the task register with the TSS selector.
    core::arch::asm!(
        "mov ds, {d:x}",
        "mov es, {d:x}",
        "mov fs, {d:x}",
        "mov gs, {d:x}",
        "mov ss, {d:x}",
        "push {c}",
        "lea {t}, [2f]",
        "push {t}",
        "retf",
        "2:",
        "ltr {ts:x}",
        d = in(reg) KERNEL_DATA,
        c = in(reg) KERNEL_CODE as u32,
        t = out(reg) _,
        ts = in(reg) TSS_SEL,
        options(nostack),
    );
}

unsafe fn tss_ptr() -> *mut Tss {
    &raw mut TSS
}

/// Update the ring-0 stack the CPU switches to on a ring3->ring0 trap.
pub unsafe fn set_kernel_stack(esp: u32) {
    (*tss_ptr()).esp0 = esp;
}
