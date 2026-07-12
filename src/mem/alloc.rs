use core::{alloc::Layout, ptr::null};

use crate::{println, sync::SpinLock};

use super::page;

pub struct Alloc {
    pages: usize,
    freelist: Option<&'static page::Page>,
}

impl Alloc {
    pub const fn new() -> Self {
        Self {
            pages: 0,
            freelist: None,
        }
    }

    pub fn init(&mut self) {
        let page_layout = Layout::from_size_align(page::SIZE, page::SIZE)
            .expect("page size must be valid for layout");

        while let Some(ptr) = super::allocate_early_phys(page_layout) {
            let pfn = super::get_pfn(ptr);
            let page = super::get_page(pfn);
            self.free_page(page);
        }

        println!(
            "pmm initialized with {} pages [{}]",
            self.pages,
            super::fmt::Memory::new(self.pages * page::SIZE)
        );
    }

    pub fn pages(&self) -> usize {
        self.pages
    }

    #[must_use]
    pub fn alloc_page(&mut self) -> Option<&'static page::Page> {
        let page = self.freelist?;
        let next: *const page::Page = unsafe { page.pointer().cast_const().cast() };

        debug_assert_ne!(self.pages, 0, "page allocator corruption detected");
        self.pages -= 1;
        self.freelist = unsafe { next.as_ref() };

        page.flags().set_page_type(page::Type::None);

        Some(page)
    }

    pub fn free_page(&mut self, page: &'static page::Page) {
        let mut flags = page.flags();
        let page_type = flags.page_type();

        assert_ne!(page_type, page::Type::Free, "double free detected");
        flags.set_page_type(page::Type::Free);

        let next = self.freelist.map_or(null(), |page| &raw const *page);
        unsafe {
            page.set_pointer(next.cast());
        }

        self.pages += 1;
        self.freelist = Some(page);
    }
}

pub static ALLOC: SpinLock<Alloc> = SpinLock::new(Alloc::new());
