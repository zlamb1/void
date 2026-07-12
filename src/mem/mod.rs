pub mod fmt;
pub mod page;

#[cfg(debug_assertions)]
use core::cell::Cell;
use core::{alloc::Layout, cell::UnsafeCell, fmt::Display, mem::MaybeUninit, ptr::NonNull, slice};

use crate::{boot::BootInfo, println, sync::SpinLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryType {
    Free,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    Reclaimable,
    Kernel,
    BadMemory,
    Framebuffer,
}

impl Display for MemoryType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            MemoryType::Free => "free",
            MemoryType::Reserved => "reserved",
            MemoryType::AcpiReclaimable => "acpi reclaimable",
            MemoryType::AcpiNvs => "acpi nvs",
            MemoryType::Reclaimable => "reclaimable",
            MemoryType::Kernel => "kernel",
            MemoryType::BadMemory => "bad memory",
            MemoryType::Framebuffer => "framebuffer",
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MemoryRegion {
    addr: usize,
    allocated: usize,
    len: usize,
    memory_type: MemoryType,
}

impl MemoryRegion {
    pub const fn new(addr: usize, len: usize, memory_type: MemoryType) -> Self {
        Self {
            addr,
            allocated: 0,
            len,
            memory_type,
        }
    }

    pub fn addr(&self) -> usize {
        self.addr
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn memory_type(&self) -> MemoryType {
        self.memory_type
    }
}

struct MemoryMap {
    count: usize,
    regions: [MemoryRegion; 256],
}

struct PageMem {
    #[cfg(debug_assertions)]
    init: Cell<bool>,
    page_mem: UnsafeCell<MaybeUninit<&'static [page::Page]>>,
}

unsafe impl Sync for PageMem {}

static MMAP: SpinLock<MemoryMap> = SpinLock::new(MemoryMap {
    count: 0,
    regions: [MemoryRegion::new(0, 0, MemoryType::Reserved); _],
});

static PAGE_MEM: PageMem = PageMem {
    #[cfg(debug_assertions)]
    init: Cell::new(false),
    page_mem: UnsafeCell::new(MaybeUninit::uninit()),
};

fn build_mmap(bi: &impl BootInfo) -> (usize, usize) {
    let mut mmap = MMAP.acquire();
    let mut max_free_addr: usize = 0;
    let mut free_bytes: usize = 0;

    for entry in bi.mmap_iter() {
        let count = mmap.count;
        assert!(mmap.count < mmap.regions.len(), "memory map full");

        mmap.regions[count] = entry;
        mmap.count += 1;

        println!(
            "firmware reported memory region [addr=0x{:x}, len=0x{:x}, type={}]",
            entry.addr, entry.len, entry.memory_type,
        );

        if entry.len() == 0 {
            continue;
        }

        if entry.memory_type == MemoryType::Free
            || entry.memory_type == MemoryType::Reclaimable
            || entry.memory_type == MemoryType::AcpiReclaimable
        {
            let end_addr = entry.addr + entry.len - 1;
            if end_addr > max_free_addr {
                max_free_addr = end_addr;
            }

            free_bytes += entry.len();
        }
    }

    return (max_free_addr, free_bytes);
}

pub fn init(bi: &impl BootInfo) {
    let (max_free_addr, free_bytes) = build_mmap(bi);

    println!("max free address at 0x{:x}", max_free_addr);
    println!(
        "available physical memory: {}",
        fmt::Memory::new(free_bytes)
    );

    let pfns: usize = ((max_free_addr / page::SIZE) + 1).try_into().unwrap();
    let layout = Layout::array::<page::Page>(pfns).unwrap();

    println!(
        "reserving {} for page metadata",
        fmt::Memory::new(layout.size())
    );

    let page_mem: NonNull<page::Page> = allocate_early(layout)
        .expect("failed to allocate page metadata")
        .cast();
    for i in 0..pfns {
        unsafe {
            page_mem.add(i).write(page::Page::new());
        }
    }

    #[cfg(debug_assertions)]
    debug_assert_eq!(PAGE_MEM.init.get(), false);
    unsafe {
        let page_mem = slice::from_raw_parts::<page::Page>(page_mem.as_ptr(), pfns);
        // SAFETY: The bootstrap processor set up page_mem
        // long before any other processors are up and ready.
        // Synchronization must be performed prior to other APs
        // running to ensure this is visible.
        (&mut *PAGE_MEM.page_mem.get()).write(page_mem);
        #[cfg(debug_assertions)]
        PAGE_MEM.init.set(true);
    }
}

pub fn get_pfn(ptr: *const ()) -> usize {
    let addr = ptr.addr();
    addr >> page::LOG2_SIZE
}

pub fn try_get_page(pfn: usize) -> Option<&'static page::Page> {
    #[cfg(debug_assertions)]
    debug_assert_eq!(PAGE_MEM.init.get(), true);
    let page_mem = unsafe { (&*PAGE_MEM.page_mem.get()).assume_init() };
    if pfn < page_mem.len() {
        Some(&page_mem[pfn])
    } else {
        None
    }
}

pub fn get_page(pfn: usize) -> &'static page::Page {
    try_get_page(pfn).unwrap()
}

pub const VADDR: usize = 0xffff800000000000;

#[allow(unused)]
macro_rules! paddr {
    ($addr:expr) => {
        $addr.checked_sub($crate::mem::VADDR)
    };
}

macro_rules! vaddr {
    ($addr:expr) => {
        $addr.checked_add($crate::mem::VADDR)
    };
}

pub fn allocate_early(layout: Layout) -> Option<NonNull<u8>> {
    let mut mmap = MMAP.acquire();
    let count = mmap.count;
    let align = layout.align();
    let size = layout.size();
    debug_assert!(size > 0);
    for region in &mut mmap.regions[..count] {
        if region.memory_type != MemoryType::Free {
            continue;
        }
        debug_assert!(region.allocated <= region.len);
        let Some(start) = region.addr.checked_add(region.allocated) else {
            continue;
        };
        let aligned_start = start.next_multiple_of(align);
        let pad = aligned_start - start;
        let len = region.len - region.allocated;
        if pad >= len || size > len - pad {
            continue;
        }
        let Some(vaddr) = vaddr!(aligned_start) else {
            println!("allocate_early: bad physical address 0x{:x}", aligned_start);
            continue;
        };
        region.allocated += pad + size;
        return Some(unsafe { NonNull::new_unchecked(vaddr as *mut u8) });
    }
    None
}
