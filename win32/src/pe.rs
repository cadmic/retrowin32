use crate::{reader::{Reader, read_strz}, x86::write_u32};
use anyhow::{anyhow, bail};
use bitflags::bitflags;

// https://docs.microsoft.com/en-us/previous-versions/ms809762(v=msdn.10)
// https://learn.microsoft.com/en-us/windows/win32/debug/pe-format

fn dos_header(r: &mut Reader) -> anyhow::Result<u32> {
    r.expect("MZ")?;
    r.skip(0x3a)?;
    Ok(r.u32()?)
}

#[derive(Debug)]
pub struct PEHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: u32,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: u16,
}

bitflags! {
    pub struct DllCharacteristics: u16 {
        const HIGH_ENTROPY_VA = 0x0020;
        const DYNAMIC_BASE = 0x0040;
        const FORCE_INTEGRITY = 0x0080;
        const NX_COMPAT = 0x0100;
        const NO_ISOLATION = 0x0200;
        const NO_SEH = 0x0400;
        const NO_BIND = 0x0800;
        const APPCONTAINER = 0x1000;
        const WDM_DRIVER = 0x2000;
        const GUARD_CF = 0x4000;
        const TERMINAL_SERVER_AWARE = 0x8000;
    }
}

#[derive(Debug)]
pub struct PEOptionalHeader {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: u32,
    pub base_of_code: u32,
    pub base_of_data: u32,
    pub image_base: u32,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_operating_system_version: u16,
    pub minor_operating_system_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub check_sum: u32,
    pub subsystem: u16,
    pub dll_characteristics: DllCharacteristics,
    pub size_of_stack_reserve: u32,
    pub size_of_stack_commit: u32,
    pub size_of_heap_reserve: u32,
    pub size_of_heap_commit: u32,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
    pub data_directory: [ImageDataDirectory; 16],
}

#[derive(Debug)]
pub struct ImageDataDirectory {
    pub virtual_address: u32,
    pub size: u32,
}

fn pe_header(r: &mut Reader) -> anyhow::Result<PEHeader> {
    r.expect("PE\0\0")?;
    let header = PEHeader {
        machine: r.u16()?,
        number_of_sections: r.u16()?,
        time_date_stamp: r.u32()?,
        pointer_to_symbol_table: r.u32()?,
        number_of_symbols: r.u32()?,
        size_of_optional_header: r.u16()?,
        characteristics: r.u16()?,
    };
    if header.machine != 0x14c {
        bail!("bad machine {:#x}", header.machine);
    }
    Ok(header)
}

fn data_directory(r: &mut Reader) -> anyhow::Result<ImageDataDirectory> {
    Ok(ImageDataDirectory {
        virtual_address: r.u32()?,
        size: r.u32()?,
    })
}

fn pe_opt_header(r: &mut Reader) -> anyhow::Result<PEOptionalHeader> {
    let opt_header: PEOptionalHeader = PEOptionalHeader {
        magic: r.u16()?,
        major_linker_version: r.u8()?,
        minor_linker_version: r.u8()?,
        size_of_code: r.u32()?,
        size_of_initialized_data: r.u32()?,
        size_of_uninitialized_data: r.u32()?,
        address_of_entry_point: r.u32()?,
        base_of_code: r.u32()?,
        base_of_data: r.u32()?,
        image_base: r.u32()?,
        section_alignment: r.u32()?,
        file_alignment: r.u32()?,
        major_operating_system_version: r.u16()?,
        minor_operating_system_version: r.u16()?,
        major_image_version: r.u16()?,
        minor_image_version: r.u16()?,
        major_subsystem_version: r.u16()?,
        minor_subsystem_version: r.u16()?,
        win32_version_value: r.u32()?,
        size_of_image: r.u32()?,
        size_of_headers: r.u32()?,
        check_sum: r.u32()?,
        subsystem: r.u16()?,
        dll_characteristics: {
            let raw = r.u16()?;
            DllCharacteristics::from_bits(raw).ok_or_else(|| anyhow!("bad flags {raw:#x}"))?
        },
        size_of_stack_reserve: r.u32()?,
        size_of_stack_commit: r.u32()?,
        size_of_heap_reserve: r.u32()?,
        size_of_heap_commit: r.u32()?,
        loader_flags: r.u32()?,
        number_of_rva_and_sizes: r.u32()?,
        data_directory: [
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
            data_directory(r)?,
        ],
    };
    Ok(opt_header)
}

#[derive(Debug)]
pub struct ImageSectionHeader {
    pub name: String,
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
    pub pointer_to_relocations: u32,
    pub pointer_to_linenumbers: u32,
    pub number_of_relocations: u16,
    pub number_of_linenumbers: u16,
    pub characteristics: ImageSectionFlags,
}

bitflags! {
    pub struct ImageSectionFlags: u32 {
        const CODE = 0x20;
        const INITIALIZED_DATA = 0x40;
        const UNINITIALIZED_DATA = 0x80;
        const MEM_DISCARDABLE = 0x02000000;
        const MEM_EXECUTE = 0x20000000;
        const MEM_READ = 0x40000000;
        const MEM_WRITE = 0x80000000;
    }
}

fn read_section(r: &mut Reader) -> anyhow::Result<ImageSectionHeader> {
    Ok(ImageSectionHeader {
        name: r.str(8)?,
        virtual_size: r.u32()?,
        virtual_address: r.u32()?,
        size_of_raw_data: r.u32()?,
        pointer_to_raw_data: r.u32()?,
        pointer_to_relocations: r.u32()?,
        pointer_to_linenumbers: r.u32()?,
        number_of_relocations: r.u16()?,
        number_of_linenumbers: r.u16()?,
        characteristics: {
            let raw = r.u32()?;
            ImageSectionFlags::from_bits(raw).ok_or_else(|| anyhow!("bad flags {raw:#x}"))?
        },
    })
}

#[derive(Debug)]
pub struct File {
    pub header: PEHeader,
    pub opt_header: PEOptionalHeader,
    pub sections: Vec<ImageSectionHeader>,
}

pub fn parse(buf: &[u8]) -> anyhow::Result<File> {
    let mut r = Reader::new(buf);

    let ofs = dos_header(&mut r)?;
    r.seek(ofs as usize)?;

    let mut file = File {
        header: pe_header(&mut r)?,
        opt_header: pe_opt_header(&mut r)?,
        sections: Vec::new(),
    };

    for _ in 0..file.header.number_of_sections {
        file.sections.push(read_section(&mut r)?);
    }

    Ok(file)
}

#[derive(Debug)]
#[allow(dead_code)]
struct ImageImportDescriptor {
    original_first_thunk: u32,
    time_date_stamp: u32,
    forwarder_chain: u32,
    name: u32,
    first_thunk: u32,
}

/// mem: memory starting at image base
/// addr: address of imports table relative to mem start
/// resolve: map an import name to the address we will jump to for it
pub fn parse_imports(
    mem: &mut [u8],
    addr: usize,
    mut resolve: impl FnMut(&str, String, u32) -> u32,
) -> anyhow::Result<()> {
    // http://sandsprite.com/CodeStuff/Understanding_imports.html
    let mut r = Reader::new(mem);
    r.seek(addr)?;
    let mut patches = Vec::new();
    loop {
        let descriptor = ImageImportDescriptor {
            original_first_thunk: r.u32()?,
            time_date_stamp: r.u32()?,
            forwarder_chain: r.u32()?,
            name: r.u32()?,
            first_thunk: r.u32()?,
        };
        if descriptor.name == 0 {
            break;
        }
        let dll_name = read_strz(&mem[descriptor.name as usize..]).to_ascii_lowercase();

        // Officially original_first_thunk should be an array that contains pointers to
        // IMAGE_IMPORT_BY_NAME entries, but in my sample executable they're all 0.
        // Peering Inside the PE claims this is some difference between compilers, yikes.

        // Code calling a DLL looks like:
        //   call [XXX]
        // where XXX is the address of an entry in the IAT:
        //   IAT: [
        //      AAA,
        //      BBB,  <- XXX might point here
        //   ]
        // On load, each IAT entry points to the function name (as parsed below).
        // The loader is supposed to overwrite the IAT with the addresses to the loaded DLL,
        // but we instead just record the IAT addresses to remap them.
        let mut iat_reader = Reader::new(&mem[descriptor.first_thunk as usize..]);
        loop {
            let addr = descriptor.first_thunk + iat_reader.pos as u32;
            let entry = iat_reader.u32()?;
            if entry == 0 {
                break;
            }
            if entry & (1 << 31) != 0 {
                let ordinal = entry & 0xFFFF;
                log::warn!("TODO ordinal {}:{}", dll_name, ordinal);
            } else {
                // First two bytes at offset are hint/name table index, used to look up
                // the name faster in the DLL; we just skip them.
                let sym_name = read_strz(&mem[(entry + 2) as usize..]);
                let target = resolve(&dll_name, sym_name, addr);
                patches.push((addr, target));
            }
        }
    }

    for (addr, target) in patches {
        write_u32(mem, addr, target);
    }

    Ok(())
}
