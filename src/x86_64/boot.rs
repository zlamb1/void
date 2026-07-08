use crate::println;

const EXCEPTION_NAMES: &[&str] = &[
    "division error",
    "debug",
    "non-maskable interrupt",
    "breakpoint",
    "overflow",
    "bound range exceeded",
    "invalid opcode",
    "device not available",
    "double fault",
    "coprocessor segment overrun",
    "invalid TSS",
    "segment not present",
    "stack-segment fault",
    "general protection fault",
    "page fault",
    "unknown",
    "x87 floating-point exception",
    "alignment check",
    "machine check",
    "SIMD floating-point exception",
    "virtualization exception",
    "control protection exception",
    "unknown",
    "hypervisor injection exception",
    "VMM communication exception",
    "security exception",
    "unknown",
    "triple fault",
    "FPU error interrupt",
];

#[unsafe(no_mangle)]
extern "C" fn boot_exception_handler(exception_number: u64) {
    let mut exception_name = "unknown";
    if exception_number < EXCEPTION_NAMES.len() as u64 {
        exception_name = EXCEPTION_NAMES[exception_number as usize];
    }
    println!("exception occurred: {}", exception_name);
    panic!();
}
