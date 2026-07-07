TARGET ?= x86_64

O ?= build
BIN ?= $(O)/void.bin
IMG ?= $(O)/void.img
CROSS_CC ?= $(TARGET)-elf-gcc

LD_SCRIPT := src/$(TARGET)/link.ld

RUST_TARGET := $(TARGET)-unknown-void

SRCS := src/$(TARGET)/_startup.S
RLIB := target/$(RUST_TARGET)/debug/libvoid.a
OBJS := $(SRCS:%.S=$(O)/%.o)
DEPS := $(OBJS:.o=.d) $(RLIB:.a=.d)

LIMINE_CONF ?= src/limine.conf
LIMINE_DIR := $(O)/limine-binary
LIMINE_EXE := $(LIMINE_DIR)/limine
LIMINE_BIOS_SYS := $(LIMINE_DIR)/limine-bios.sys
LIMINE_TAR := $(O)/limine-binary.tar
LIMINE_TAR_GZ := $(LIMINE_TAR).gz
LIMINE_VER ?= 12.3.3

.PHONY: all qemu limine-clean clean

all: $(IMG)

$(IMG): $(BIN) $(LIMINE_CONF) $(LIMINE_EXE)
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
	cargo build -Z json-target-spec --target targets/$(RUST_TARGET).json

$(O)/%.o: %.S
	mkdir -p $(dir $@)
	$(CROSS_CC) -g -MMD -MP -c $< -o $@

$(LIMINE_EXE): | $(LIMINE_DIR)
	make -C $(LIMINE_DIR)

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

qemu: $(IMG)
	qemu-system-x86_64 -drive format=raw,file=$< -m 64M -nic none

gdb: $(IMG)
	qemu-system-x86_64 -drive format=raw,file=$< -m 64M -nic none -s -S & \
	QEMU_PID=$$!; \
	gdb $(BIN) -q -ex 'target remote localhost:1234' -ex 'set pagination off' -ex 'layout src' -ex 'b kernel_main' -ex 'c'; \
	kill -2 $$QEMU_PID

lldb: $(IMG)
	qemu-system-x86_64 -drive format=raw,file=$< -m 64M -nic none -s -S & \
	QEMU_PID=$$!; \
	lldb $(BIN) -o 'gdb-remote localhost:1234' -o 'b kernel_main' -o 'c'
	kill -2 $$QEMU_PID

limine-clean:
	rm -f $(LIMINE_TAR_GZ) $(LIMINE_TAR)
	rm -rf $(LIMINE_DIR)

clean:
	rm -f $(IMG) $(BIN) $(RLIB) $(OBJS) $(DEPS)

-include $(DEPS)