use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{arch, mem::boxed::Box, per_cpu::PerCpu, println, sync::once::Once};

pub const BSP_CPU_ID: usize = 0;

static BSP_PER_CPU: Once<PerCpu> = Once::new(PerCpu::new(BSP_CPU_ID));
static CPU_COUNT: AtomicUsize = AtomicUsize::new(1);

pub fn init() {
    BSP_PER_CPU
        .call_once(|per_cpu| {
            per_cpu.init();
            arch::set_per_cpu(per_cpu);
        })
        .expect("BSP already initialized");
}

pub fn prelude(_: u64) -> ! {
    let cpu_id = CPU_COUNT.fetch_add(1, Ordering::Relaxed);
    main(cpu_id);
}

pub fn main(cpu_id: usize) -> ! {
    let mut per_cpu = Box::new(PerCpu::new(cpu_id));
    per_cpu.init();
    let per_cpu = Box::leak(per_cpu);
    // SAFETY: Auxiliary APs (e.g. not BSP) _must_ not
    // take any locks prior to setting up their per-cpu data.
    arch::set_per_cpu(per_cpu);
    println!("running mp{}", cpu_id);
    loop {}
}

pub fn cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Relaxed)
}

pub fn cpu_id() -> usize {
    arch::get_per_cpu().cpu_id()
}
