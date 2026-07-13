#![no_main]
#![no_std]
#![feature(unsafe_pinned)]
#![feature(bstr)]

use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crate::{boot::BootInfo, mem::boxed::Box, per_cpu::PerCpu};

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
pub mod arch;
pub mod backtrace;
pub mod boot;
pub mod date;
pub mod gfx;
pub mod list;
pub mod log;
pub mod mem;
pub mod per_cpu;
pub mod ptr;
pub mod sync;

static HAS_PANICKED: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(pi: &PanicInfo) -> ! {
    arch::irq_disable();
    if HAS_PANICKED.swap(true, Ordering::SeqCst) {
        loop {
            arch::halt();
        }
    }

    log::clear();
    println!("panic: {}", pi.message());
    if let Some(location) = pi.location() {
        println!(
            "location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    println!("");
    println!("backtrace:");
    backtrace::backtrace();

    loop {
        arch::halt();
    }
}

static NEXT_CPU: AtomicUsize = AtomicUsize::new(1);

fn mp_prelude(_: u64) -> ! {
    let cpu_id = NEXT_CPU.fetch_add(1, Ordering::Relaxed);
    mp_main(cpu_id);
}

fn mp_main(cpu_id: usize) -> ! {
    let mut per_cpu = Box::new(PerCpu::new(cpu_id));
    per_cpu.init();
    let per_cpu = Box::leak(per_cpu);
    arch::set_per_cpu(per_cpu);
    println!("running mp{}", cpu_id);
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    log::init();
    let boot_info = boot::init();
    mem::init(&boot_info);
    boot_info.mp_start(mp_prelude);
    mp_main(0);
}
