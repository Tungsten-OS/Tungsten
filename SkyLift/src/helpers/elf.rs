// Copyright (c) ChefKiss Inc 2021-2024. Licensed under the Thou Shalt Not Profit License version 1.5. See LICENSE for details.

use alloc::{borrow::ToOwned, boxed::Box, vec::Vec};

use amd64::paging::{KERNEL_VIRT_OFFSET, PAGE_SIZE};

pub fn parse(
    mem_mgr: &mut super::mem::MemoryManager,
    buffer: &[u8],
) -> (skyliftkit::EntryPoint, Vec<skyliftkit::KernSymbol>) {
    let elf = elf::ElfBytes::<elf::endian::LittleEndian>::minimal_parse(buffer).unwrap();

    assert_eq!(elf.ehdr.class, elf::file::Class::ELF64);
    assert_eq!(elf.ehdr.e_machine, elf::abi::EM_X86_64);
    assert!(
        elf.ehdr.e_entry >= KERNEL_VIRT_OFFSET,
        "Only higher-half kernels"
    );

    let symbols = elf
        .symbol_table()
        .unwrap()
        .map(|(symtab, strtab)| {
            symtab
                .iter()
                .map(|v| skyliftkit::KernSymbol {
                    start: v.st_value,
                    end: v.st_value + v.st_size,
                    name: Box::leak(
                        strtab
                            .get(v.st_name as _)
                            .unwrap_or("<unknown>")
                            .to_owned()
                            .into_boxed_str(),
                    ),
                })
                .collect()
        })
        .unwrap_or_default();

    trace!("Parsing program headers: ");
    let st = uefi::table::system_table_boot().unwrap();
    let bs = st.boot_services();
    let segments = elf.segments().unwrap();
    let lowest_addr = segments
        .iter()
        .filter_map(|v| {
            if v.p_type == elf::abi::PT_LOAD {
                Some(v.p_vaddr)
            } else {
                None
            }
        })
        .min()
        .unwrap();
    assert!(
        lowest_addr >= KERNEL_VIRT_OFFSET,
        "Only higher-half kernels"
    );
    let lowest_addr_phys = lowest_addr - KERNEL_VIRT_OFFSET;
    let highest_addr = segments
        .iter()
        .filter_map(|v| {
            if v.p_type == elf::abi::PT_LOAD {
                Some(v.p_vaddr + v.p_memsz)
            } else {
                None
            }
        })
        .max()
        .unwrap();
    let kern_region_pages = ((highest_addr - lowest_addr + (PAGE_SIZE - 1)) / PAGE_SIZE) as usize;
    assert_eq!(
        bs.allocate_pages(
            uefi::table::boot::AllocateType::Address(lowest_addr_phys as _),
            uefi::table::boot::MemoryType::LOADER_DATA,
            kern_region_pages,
        )
        .unwrap(),
        lowest_addr_phys,
    );
    mem_mgr.allocate((lowest_addr_phys, kern_region_pages as _));
    for phdr in segments
        .iter()
        .filter(|phdr| phdr.p_type == elf::abi::PT_LOAD)
    {
        let offset = phdr.p_offset as usize;
        let mem_size = phdr.p_memsz as usize;
        let file_size = phdr.p_filesz as usize;
        let source = unsafe { buffer.as_ptr().add(offset) };
        let dest = (phdr.p_vaddr - KERNEL_VIRT_OFFSET) as *mut u8;
        trace!(
            "vaddr: {:#X}, paddr: {:#X}",
            phdr.p_vaddr,
            phdr.p_vaddr - KERNEL_VIRT_OFFSET
        );

        unsafe {
            source.copy_to(dest, file_size);
            dest.add(file_size).write_bytes(0, mem_size - file_size);
        }
    }

    (
        unsafe {
            core::mem::transmute::<*const (), skyliftkit::EntryPoint>(elf.ehdr.e_entry as *const ())
        },
        symbols,
    )
}
