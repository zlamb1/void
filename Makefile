TARGET ?= x86_64

O ?= build
BIN ?= $(O)/void.bin
BIOS_IMG ?= $(O)/void.bios.img
UEFI_IMG ?= $(O)/void.uefi.img
CROSS_CC ?= $(TARGET)-elf-gcc
CARGO ?= cargo
QEMU ?= qemu-system-x86_64
GDB ?= gdb
LLDB ?= lldb

LD_SCRIPT := src/$(TARGET)/link.ld

RUST_TARGET := $(TARGET)-unknown-void

RFLAGS ?=

ifdef RELEASE
RFLAGS += --release
RLIB := target/$(RUST_TARGET)/release/libvoid.a
else
RLIB := target/$(RUST_TARGET)/debug/libvoid.a
endif

SRCS := src/$(TARGET)/_startup.S
OBJS := $(SRCS:%.S=$(O)/%.o)
DEPS := $(OBJS:.o=.d)

LIMINE_CONF ?= src/boot/limine.conf
LIMINE_DIR := $(O)/limine-binary
LIMINE_EXE := $(LIMINE_DIR)/limine
LIMINE_BIOS_SYS := $(LIMINE_DIR)/limine-bios.sys
LIMINE_BOOT_EFI := $(LIMINE_DIR)/BOOTX64.EFI
LIMINE_TAR := $(O)/limine-binary.tar
LIMINE_TAR_GZ := $(LIMINE_TAR).gz
LIMINE_VER ?= 12.3.3

.PHONY: all bios uefi $(RLIB) qemu limine-clean clean

all: bios uefi

bios: $(BIOS_IMG)

uefi: $(UEFI_IMG)

$(UEFI_IMG): $(BIN) $(LIMINE_CONF) $(LIMINE_EXE)
	dd if=/dev/zero of=$@.tmp bs=1M count=64
	mkfs.fat -F 32 $@.tmp
	mcopy -i $@.tmp $(BIN) ::/void.bin
	mcopy -i $@.tmp $(LIMINE_BIOS_SYS) ::/limine-bios.sys
	mcopy -i $@.tmp $(LIMINE_CONF) ::/limine.conf
	mmd -i $@.tmp ::/EFI
	mmd -i $@.tmp ::/EFI/BOOT
	mcopy -i $@.tmp $(LIMINE_BOOT_EFI) ::/EFI/BOOT/BOOTX64.EFI
	dd if=/dev/zero of=$@ bs=1M count=64
	cat $@.tmp >>$@
	rm $@.tmp
	parted $@ -s -- mklabel gpt mkpart ESP fat32 64MiB 127MiB set 1 boot on set 1 esp on

$(BIOS_IMG): $(BIN) $(LIMINE_CONF) $(LIMINE_EXE)
	dd if=/dev/zero of=$@.tmp bs=1M count=64
	mkfs.fat -F 32 $@.tmp
	mcopy -i $@.tmp $(BIN) ::/void.bin
	mcopy -i $@.tmp $(LIMINE_BIOS_SYS) ::/limine-bios.sys
	mcopy -i $@.tmp $(LIMINE_CONF) ::/limine.conf
	dd if=/dev/zero of=$@ bs=1M count=64
	cat $@.tmp >>$@
	rm $@.tmp
	parted $@ -s -- mklabel msdos mkpart primary fat32 64MiB -1s set 1 boot on
	$(LIMINE_EXE) bios-install $@

$(BIN): $(RLIB) $(OBJS) $(LD_SCRIPT) | $(O)
	$(CROSS_CC) -T $(LD_SCRIPT) -Wl,-z noexecstack -nostdlib $(OBJS) $(RLIB) -o $@

$(RLIB):
	$(CARGO) build -Z json-target-spec --target targets/$(RUST_TARGET).json $(RFLAGS)

$(O)/%.o: %.S
	mkdir -p $(dir $@)
	$(CROSS_CC) -g -MMD -MP -c $< -o $@

$(LIMINE_EXE): | $(LIMINE_DIR)
	$(MAKE) -C $(LIMINE_DIR)

$(LIMINE_DIR): $(LIMINE_TAR)
	mkdir $@
	tar -xf $< -C $(O)

$(LIMINE_TAR): $(LIMINE_TAR_GZ)
	gunzip -k $<

$(LIMINE_TAR_GZ): | $(O)
	wget https://github.com/Limine-Bootloader/Limine/releases/download/v$(LIMINE_VER)/limine-binary.tar.gz -O $@
	echo '205f98218bb0d5a8ccabf5f903dba9d935f7b0aa66f4262a99b0f5a8e668ec6d  $@' | sha256sum -c -

$(O):
	mkdir $@

qemu: $(BIOS_IMG)
	$(QEMU) -drive format=raw,file=$< -m 64M -nic none -accel kvm

gdb: $(BIOS_IMG)
	$(QEMU) -drive format=raw,file=$< -m 64M -nic none -s -S & \
	QEMU_PID=$$!; \
	$(GDB) $(BIN) -q -ex 'target remote localhost:1234' -ex 'set pagination off' -ex 'layout src' -ex 'b kernel_main' -ex 'c'; \
	kill -2 $$QEMU_PID

lldb: $(BIOS_IMG)
	$(QEMU) -drive format=raw,file=$< -m 64M -nic none -s -S & \
	QEMU_PID=$$!; \
	$(LLDB) $(BIN) -o 'gdb-remote localhost:1234' -o 'b kernel_main' -o 'c'
	kill -2 $$QEMU_PID

limine-clean:
	rm -f $(LIMINE_TAR_GZ) $(LIMINE_TAR)
	rm -rf $(LIMINE_DIR)

clean:
	$(CARGO) clean
	rm -f $(BIOS_IMG) $(UEFI_IMG) $(BIN) $(OBJS) $(DEPS)

-include $(DEPS)