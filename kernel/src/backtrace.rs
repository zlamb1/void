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
struct Symbol {
    address: usize,
    size: usize,
    name: *const c_char,
}

pub fn backtrace() {
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

    if ksyms.len() == 0 {
        println!("no symbols found");
        return;
    }

    while !rbp.is_null() {
        let return_address = unsafe {
            let return_address = *rbp.add(1);
            rbp = *rbp as *const u64;
            return_address
        } as usize;
        if return_address == 0 {
            return;
        }
        let find = return_address - 1;

        let mut lo = 0usize;
        let mut hi = ksyms.len();

        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let symbol = &ksyms[mid];
            if symbol.address <= find {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        let found = lo
            .checked_sub(1)
            .map(|i| &ksyms[i])
            .filter(|&symbol| symbol.address + symbol.size > find);

        if let Some(symbol) = found {
            let cstr = unsafe { CStr::from_ptr(symbol.name) };
            let byte_str = ByteStr::new(cstr.to_bytes());

            println!(
                " -- [{:x}] {}+{:#x}",
                return_address,
                byte_str,
                return_address - symbol.address
            );
        } else {
            println!(" -- [{:x}] ???", return_address);
        }
    }
}
