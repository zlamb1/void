use core::{
    arch::asm,
    bstr::ByteStr,
    ffi::{CStr, c_char},
    ptr::addr_of,
    slice::from_raw_parts,
};

use crate::println;

unsafe extern "C" {
    static _sksyms: u8;
    static _eksyms: u8;
}

#[repr(C)]
pub struct Symbol {
    address: usize,
    size: usize,
    name: *const c_char,
}

impl Symbol {
    pub fn address(&self) -> usize {
        self.address
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn name(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.name) }
    }
}

fn find_symbol(ksyms: &[Symbol], address: usize) -> Option<&Symbol> {
    let mut lo = 0usize;
    let mut hi = ksyms.len();

    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let symbol = &ksyms[mid];
        if symbol.address <= address {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    lo.checked_sub(1)
        .map(|i| &ksyms[i])
        .filter(|&symbol| symbol.address + symbol.size > address)
}

pub fn backtrace_for_each(f: impl Fn(usize, Option<&Symbol>)) {
    let ksyms = unsafe {
        let start: *const Symbol = addr_of!(_sksyms).cast();
        let end: *const Symbol = addr_of!(_eksyms).cast();
        let len = end.offset_from_unsigned(start);
        from_raw_parts(start, len)
    };

    let mut rbp: *const u64;
    unsafe {
        asm!("mov {}, rbp", out (reg) rbp);
    }

    while !rbp.is_null() {
        if !rbp.is_aligned() {
            return;
        }

        let return_address = unsafe {
            let return_address = *rbp.add(1);
            rbp = *rbp as *const u64;
            return_address
        } as usize;

        if return_address == 0 {
            return;
        }

        let symbol = find_symbol(ksyms, return_address - 1);
        f(return_address, symbol);
    }
}

pub fn backtrace() {
    backtrace_for_each(|return_address, symbol| {
        if let Some(symbol) = symbol {
            println!(
                " -- [{:x}] {}+{:#x}/{:#x}",
                return_address,
                ByteStr::new(symbol.name().to_bytes()),
                return_address - symbol.address,
                symbol.size,
            );
        } else {
            println!(" -- [{:x}] ???", return_address);
        }
    });
}
