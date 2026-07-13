use core::arch::asm;

use crate::per_cpu::PerCpu;

mod boot;
mod msr;

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

pub fn set_per_cpu(per_cpu: &'static PerCpu) {
    msr::wrmsr(msr::Msr::GsBase, (&raw const *per_cpu).addr() as u64);
}

pub fn get_per_cpu() -> &'static PerCpu {
    unsafe {
        let per_cpu: *const PerCpu;
        asm!("mov {}, gs:[0]", out(reg) per_cpu, options(nostack));
        &*per_cpu
    }
}
