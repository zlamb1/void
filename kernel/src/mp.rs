use core::{
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    arch,
    boot::{BootInfo, Mp},
    mem::boxed::Box,
    per_cpu::PerCpu,
    println,
    sync::once::Once,
};

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

pub fn kickoff(boot_info: &impl BootInfo) -> ! {
    let mp = boot_info.mp();

    let mut per_cpu = Box::new(PerCpu::new(BSP_CPU_ID));
    per_cpu.init();

    mp.set_extra_argument(|bsp| {
        if !bsp {
            let cpu_id = CPU_COUNT.fetch_add(1, Ordering::Relaxed);
            let mut per_cpu = Box::new(PerCpu::new(cpu_id));
            per_cpu.init();
            Box::into_raw(per_cpu).addr().try_into().unwrap()
        } else {
            0
        }
    });

    mp.start(main);

    main(Box::into_raw(per_cpu).addr().try_into().unwrap());
}

pub fn main(per_cpu: u64) -> ! {
    let per_cpu = NonNull::new(per_cpu as *mut PerCpu).unwrap();
    debug_assert!(per_cpu.is_aligned());

    let per_cpu = unsafe { per_cpu.as_ref() };

    // SAFETY: Auxiliary APs (e.g. not BSP) _must_ not
    // take any locks prior to setting up their per-cpu data.
    arch::set_per_cpu(per_cpu);

    println!("running mp{}", per_cpu.cpu_id());

    loop {}
}

pub fn cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Relaxed)
}

pub fn cpu_id() -> usize {
    arch::get_per_cpu().cpu_id()
}
