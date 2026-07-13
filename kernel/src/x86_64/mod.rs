use core::arch::asm;

mod boot;

pub fn halt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

pub fn irq_disable() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

pub fn irq_enable() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

pub fn irq_restore(flags: usize) {
    unsafe { asm!("push {}; popfq", in(reg)flags, options(nomem)) }
}

pub fn irq_save() -> usize {
    let mut flags: usize;
    unsafe {
        asm!("pushfq; pop {}", out(reg)flags, options(nomem));
    };
    flags
}

pub fn irq_save_and_disable() -> usize {
    let flags = irq_save();
    irq_disable();
    flags
}

pub fn spin_hint() {
    unsafe { asm!("pause", options(nomem, nostack, preserves_flags)) }
}

pub fn sfence() {
    unsafe { asm!("sfence", options(nomem, nostack)) }
}
