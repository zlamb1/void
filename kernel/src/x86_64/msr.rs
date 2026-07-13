use core::arch::asm;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Msr {
    #[allow(unused)]
    FsBase = 0xC0000100,
    GsBase = 0xC0000101,
    #[allow(unused)]
    KernelGsBase = 0xC0000102,
}

#[allow(unused)]
pub fn rdmsr(msr: Msr) -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        asm!("rdmsr", out("eax") lo, in("ecx") msr as u32, out("edx") hi, options(nostack, preserves_flags));
    }
    (lo as u64) | ((hi as u64) << 32)
}

pub fn wrmsr(msr: Msr, v: u64) {
    unsafe {
        asm!("wrmsr", in("eax") v as u32, in("ecx") msr as u32, in("edx") (v >> 32) as u32, options(nostack, preserves_flags));
    }
}
