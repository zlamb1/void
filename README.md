## Void Kernel
A simple kernel for x86-64 written entirely in Rust (with some assembly components).
## Build Dependencies
- cargo
- rustc
- coreutils
- make
- wget
- mtools
- mkfs.fat
- parted
## Build Instructions
- make (creates BIOS+UEFI image)
- make bios (creates BIOS image)
- make uefi (creates UEFI image)
## Debugging
- make qemu (builds & runs in QEMU)
- make gdb (builds & runs in QEMU & attaches GDB)
- make lldb (builds & runs in QEMU & attaches LLDB)