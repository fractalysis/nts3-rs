use object::elf;
use object::read::elf::{FileHeader, ProgramHeader, SectionHeader};
use object::{Endianness, Object, ObjectSymbol};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const REPORT_SCHEMA_VERSION: u32 = 1;
pub const UNIT_LIMIT_BYTES: u64 = 32 * 1024;
pub const LOAD_LIMIT_BYTES: u64 = 32 * 1024;
pub const STATIC_RAM_LIMIT_BYTES: u64 = 32 * 1024;
pub const SDRAM_LIMIT_BYTES: u64 = 3 * 1024 * 1024;
const EXPECTED_TARGET: u32 = (6 << 8) | 7;
const EXPECTED_API: u32 = 2 << 16;
const GENERICFX_HEADER_BYTES: u64 = 376;
const RESOURCE_BYTES: u64 = 12;
const R_ARM_RELATIVE: u32 = 23;

pub const CALLBACKS: &[&str] = &[
    "unit_init",
    "unit_teardown",
    "unit_reset",
    "unit_resume",
    "unit_suspend",
    "unit_render",
    "unit_get_param_value",
    "unit_get_param_str_value",
    "unit_set_param_value",
    "unit_set_tempo",
    "unit_tempo_4ppqn_tick",
    "unit_touch_event",
];

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProbeReport {
    pub plugin_bytes: u64,
    pub parameters_bytes: u64,
    pub runtime_bytes: u64,
    pub budget_bytes: u64,
    pub requested_bytes: u64,
    pub alignment_padding_bytes: u64,
    pub high_water_bytes: u64,
    pub initialization_allocations: u64,
    pub post_init_allocations: u64,
    pub render_iterations: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Provenance {
    pub rust: String,
    pub cargo: String,
    pub gnu_arm_gcc: String,
    pub gnu_binutils: String,
    pub sdk_commit: String,
    pub framework_commit: String,
    pub framework_version: String,
    pub fundsp_version: String,
    pub container_base_image: String,
    pub cargo_lock_sha256: String,
    pub sdk_linker_script_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct InspectionReport {
    pub schema_version: u32,
    pub policy: String,
    pub status: String,
    pub artifact: ArtifactReport,
    pub elf: ElfReport,
    pub loads: LoadReport,
    pub sections: SectionReport,
    pub static_ram: BudgetReport,
    pub sdram: SdramReport,
    pub native_probe: Option<ProbeReport>,
    pub stack: UnknownMeasurement,
    pub realtime_cpu: UnknownMeasurement,
    pub provenance: Provenance,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArtifactReport {
    pub path: String,
    pub sha256: String,
    pub stripped_bytes: u64,
    pub limit_bytes: u64,
    pub margin_bytes: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ElfReport {
    pub class: String,
    pub data: String,
    pub machine: String,
    pub file_type: String,
    pub osabi: String,
    pub eabi: String,
    pub float_abi: String,
    pub target: u32,
    pub api: u32,
    pub unit_name: String,
    pub required_exports: usize,
    pub relocation_count: u64,
    pub unwind_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LoadReport {
    pub segments: Vec<LoadSegment>,
    pub file_union_bytes: u64,
    pub memory_union_bytes: u64,
    pub limit_bytes: u64,
    pub margin_bytes: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LoadSegment {
    pub index: usize,
    pub offset: u64,
    pub virtual_address: u64,
    pub file_bytes: u64,
    pub file_end: u64,
    pub memory_bytes: u64,
    pub memory_end: u64,
    pub alignment: u64,
    pub flags: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SectionReport {
    pub text_bytes: u64,
    pub rodata_bytes: u64,
    pub data_bytes: u64,
    pub bss_bytes: u64,
    pub got_bytes: u64,
    pub dynamic_bytes: u64,
    pub relocation_bytes: u64,
    pub unit_header_bytes: u64,
    pub resource_bytes: u64,
    pub allocated_memory_union_bytes: u64,
    pub load_overhead_bytes: u64,
    pub allocated: Vec<SectionSize>,
    pub writable: Vec<SectionSize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SectionSize {
    pub name: String,
    pub address: u64,
    pub bytes: u64,
    pub file_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct BudgetReport {
    pub bytes: u64,
    pub limit_bytes: u64,
    pub margin_bytes: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SdramReport {
    pub declared_bytes: Option<u64>,
    pub platform_limit_bytes: u64,
    pub declared_margin_bytes: Option<i64>,
    pub measured_high_water_bytes: Option<u64>,
    pub measured_margin_bytes: Option<i64>,
    pub measured_percent_of_platform: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnknownMeasurement {
    pub status: &'static str,
    pub reason: &'static str,
}

pub fn inspect(
    path: &Path,
    probe: Option<ProbeReport>,
    provenance: Provenance,
) -> Result<InspectionReport, String> {
    let data =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    validate_ident(&data)?;
    let header = elf::FileHeader32::<Endianness>::parse(&*data)
        .map_err(|error| format!("malformed ELF header: {error}"))?;
    let endian = header
        .endian()
        .map_err(|error| format!("malformed ELF endianness: {error}"))?;
    let sections = header
        .sections(endian, &*data)
        .map_err(|error| format!("malformed ELF sections: {error}"))?;
    let programs = header
        .program_headers(endian, &*data)
        .map_err(|error| format!("malformed ELF program headers: {error}"))?;
    let file = object::read::elf::ElfFile32::<Endianness>::parse(&*data)
        .map_err(|error| format!("malformed ELF: {error}"))?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    validate_header(header, endian, &mut errors);

    let mut section_names = Vec::with_capacity(sections.len());
    let mut counts = BTreeMap::<String, usize>::new();
    let mut allocated_sections = Vec::new();
    let mut allocated_ranges = Vec::new();
    let mut writable_sections = Vec::new();
    let mut writable_ranges = Vec::new();
    let mut breakdown = SectionReport {
        text_bytes: 0,
        rodata_bytes: 0,
        data_bytes: 0,
        bss_bytes: 0,
        got_bytes: 0,
        dynamic_bytes: 0,
        relocation_bytes: 0,
        unit_header_bytes: 0,
        resource_bytes: 0,
        allocated_memory_union_bytes: 0,
        load_overhead_bytes: 0,
        allocated: Vec::new(),
        writable: Vec::new(),
    };
    let mut unwind_bytes = 0_u64;
    let mut relocation_count = 0_u64;
    let mut has_plt = false;
    let mut strict_relocations = Vec::new();
    let mut initializer_sections = Vec::new();

    for (index, section) in sections.enumerate() {
        let name =
            String::from_utf8_lossy(sections.section_name(endian, section).map_err(|error| {
                format!("malformed section name at index {}: {error}", index.0)
            })?)
            .into_owned();
        *counts.entry(name.clone()).or_default() += 1;
        section_names.push(name.clone());
        let size: u64 = section.sh_size(endian).into();
        let address: u64 = section.sh_addr(endian).into();
        let flags: u64 = section.sh_flags(endian).into();
        let file_bytes = if section.sh_type(endian) == elf::SHT_NOBITS {
            0
        } else {
            size
        };
        let section_alignment: u64 = section.sh_addralign(endian).into();
        if section_alignment != 0 && !section_alignment.is_power_of_two() {
            errors.push(format!(
                "section `{name}` has invalid alignment {section_alignment}"
            ));
        }
        if flags & u64::from(elf::SHF_ALLOC) != 0
            && section_alignment > 1
            && !address.is_multiple_of(section_alignment)
        {
            errors.push(format!("allocated section `{name}` address is misaligned"));
        }

        match name.as_str() {
            ".text" => breakdown.text_bytes += size,
            ".rodata" => breakdown.rodata_bytes += size,
            ".data" => breakdown.data_bytes += size,
            ".bss" => breakdown.bss_bytes += size,
            ".got" | ".got.plt" => breakdown.got_bytes += size,
            ".dynamic" => breakdown.dynamic_bytes += size,
            ".unit_header" => breakdown.unit_header_bytes += size,
            ".nts3_resources" => breakdown.resource_bytes += size,
            _ => {}
        }
        if section.sh_type(endian) == elf::SHT_REL || section.sh_type(endian) == elf::SHT_RELA {
            breakdown.relocation_bytes += size;
        }
        if name == ".plt" || name.starts_with(".plt.") {
            has_plt |= size != 0;
        }
        if declared_initializer_section(&name) && size != 0 {
            initializer_sections.push((name.clone(), size));
        }
        if name.starts_with(".debug")
            || name.starts_with(".zdebug")
            || name == ".gnu_debuglink"
            || name == ".symtab"
        {
            errors.push(format!(
                "prohibited debug/unstripped section `{name}` ({size} bytes)"
            ));
        }
        if name.starts_with(".ARM.exidx")
            || name.starts_with(".ARM.extab")
            || name.starts_with(".eh_frame")
        {
            unwind_bytes = unwind_bytes.saturating_add(size);
            if name.starts_with(".ARM.extab") || name.starts_with(".eh_frame") {
                errors.push(format!(
                    "unsupported unwind section `{name}` ({size} bytes)"
                ));
            }
        }
        if flags & u64::from(elf::SHF_TLS) != 0 {
            errors.push(format!("unsupported TLS section `{name}`"));
        }
        if flags & u64::from(elf::SHF_ALLOC) != 0 && size != 0 {
            allocated_ranges.push((address, address.saturating_add(size)));
            allocated_sections.push(SectionSize {
                name: name.clone(),
                address,
                bytes: size,
                file_bytes,
            });
        }
        if flags & u64::from(elf::SHF_ALLOC | elf::SHF_WRITE)
            == u64::from(elf::SHF_ALLOC | elf::SHF_WRITE)
            && size != 0
        {
            writable_ranges.push((address, address.saturating_add(size)));
            writable_sections.push(SectionSize {
                name: name.clone(),
                address,
                bytes: size,
                file_bytes,
            });
        }

        if let Some((relocations, _)) = section
            .rel(endian, &*data)
            .map_err(|error| format!("malformed relocations in `{name}`: {error}"))?
        {
            relocation_count += relocations.len() as u64;
            for relocation in relocations {
                strict_relocations.push((
                    name.clone(),
                    relocation.r_type(endian),
                    relocation.r_sym(endian),
                ));
            }
        }
        if section.sh_type(endian) == elf::SHT_RELA && size != 0 {
            errors.push(format!("unsupported RELA relocation section `{name}`"));
        }
    }
    breakdown.allocated = allocated_sections;
    breakdown.writable = writable_sections;

    if unwind_bytes > 8 {
        errors.push(format!(
            "unwind index baggage exceeds the 8-byte cantunwind policy: {unwind_bytes} bytes"
        ));
    }

    let header_sections = counts.get(".unit_header").copied().unwrap_or(0);
    if header_sections != 1 {
        errors.push(format!(
            "expected exactly one `.unit_header` section, found {header_sections}"
        ));
    }
    let (target, api, unit_name) = parse_unit_header(&sections, endian, &data, &mut errors)?;

    let resource_sections = counts.get(".nts3_resources").copied().unwrap_or(0);
    let declared_sdram = if resource_sections == 1 {
        parse_resources(&sections, endian, &data, &mut errors)?
    } else if resource_sections == 0 {
        warnings.push(
            "legacy SDK artifact has no `.nts3_resources`; declared SDRAM and native probe are unavailable"
                .to_owned(),
        );
        None
    } else {
        errors.push(format!(
            "expected at most one `.nts3_resources` section, found {resource_sections}"
        ));
        None
    };
    let policy = if declared_sdram.is_some() {
        if breakdown.unit_header_bytes != GENERICFX_HEADER_BYTES {
            errors.push(format!(
                "framework `.unit_header` must contain exactly one {GENERICFX_HEADER_BYTES}-byte record, found {} bytes",
                breakdown.unit_header_bytes
            ));
        }
        "nts3-framework-v1"
    } else {
        if breakdown.unit_header_bytes > GENERICFX_HEADER_BYTES {
            warnings.push(format!(
                "legacy SDK `.unit_header` contains {} records; the first strong record is inspected",
                breakdown.unit_header_bytes / GENERICFX_HEADER_BYTES
            ));
        }
        "korg-sdk-legacy-v1"
    };

    validate_symbols(&file, declared_sdram.is_some(), &mut errors)?;
    if declared_sdram.is_some() {
        if has_plt {
            errors.push("framework artifact contains a non-empty PLT section".to_owned());
        }
        for (section, size) in &initializer_sections {
            errors.push(format!(
                "framework artifact contains unsupported initializer section `{section}` ({size} bytes)"
            ));
        }
        for (section, relocation, symbol) in &strict_relocations {
            if *relocation != R_ARM_RELATIVE || *symbol != 0 {
                errors.push(format!(
                    "unsupported framework relocation type {relocation} (symbol {symbol}) in `{section}`"
                ));
            }
        }
    } else if has_plt
        || strict_relocations
            .iter()
            .any(|(_, kind, _)| *kind != R_ARM_RELATIVE)
    {
        warnings.push(
            "legacy SDK policy permits locally-defined C/C++ relocations and PLT entries; framework builds forbid them"
                .to_owned(),
        );
    }

    let mut loads = Vec::new();
    let mut load_file_ranges = Vec::new();
    let mut load_memory_ranges = Vec::new();
    let mut writable_load_ranges = Vec::new();
    for (index, program) in programs.iter().enumerate() {
        let kind = program.p_type(endian);
        if kind == elf::PT_TLS {
            errors.push("unsupported PT_TLS program header".to_owned());
        }
        if kind != elf::PT_LOAD {
            continue;
        }
        let offset: u64 = program.p_offset(endian).into();
        let address: u64 = program.p_vaddr(endian).into();
        let file_bytes: u64 = program.p_filesz(endian).into();
        let memory_bytes: u64 = program.p_memsz(endian).into();
        let alignment: u64 = program.p_align(endian).into();
        let flags = program.p_flags(endian);
        if file_bytes > memory_bytes {
            errors.push(format!("PT_LOAD[{index}] filesz exceeds memsz"));
        }
        if offset
            .checked_add(file_bytes)
            .is_none_or(|end| end > data.len() as u64)
        {
            errors.push(format!(
                "PT_LOAD[{index}] file range is outside the artifact"
            ));
        }
        if alignment == 0 || !alignment.is_power_of_two() || alignment > 128 {
            errors.push(format!(
                "PT_LOAD[{index}] alignment {alignment:#x} violates the SDK max-page-size=128 policy"
            ));
        } else if offset % alignment != address % alignment {
            errors.push(format!(
                "PT_LOAD[{index}] offset/address alignment is incongruent"
            ));
        }
        if flags & elf::PF_W != 0 && flags & elf::PF_X != 0 {
            errors.push(format!("PT_LOAD[{index}] is writable and executable"));
        }
        load_file_ranges.push((offset, offset.saturating_add(file_bytes)));
        load_memory_ranges.push((address, address.saturating_add(memory_bytes)));
        if flags & elf::PF_W != 0 {
            writable_load_ranges.push((address, address.saturating_add(memory_bytes)));
        }
        loads.push(LoadSegment {
            index,
            offset,
            virtual_address: address,
            file_bytes,
            file_end: offset.saturating_add(file_bytes),
            memory_bytes,
            memory_end: address.saturating_add(memory_bytes),
            alignment,
            flags: program_flags(flags),
        });
    }
    if loads.is_empty() {
        errors.push("ELF has no PT_LOAD segments".to_owned());
    }
    validate_alloc_sections_in_loads(
        &sections,
        endian,
        &load_memory_ranges,
        &section_names,
        &mut errors,
    );

    let load_file_union = union_size(&load_file_ranges);
    let load_memory_union = union_size(&load_memory_ranges);
    breakdown.allocated_memory_union_bytes = union_size(&allocated_ranges);
    breakdown.load_overhead_bytes =
        load_memory_union.saturating_sub(breakdown.allocated_memory_union_bytes);
    let section_writable_bytes = union_size(&writable_ranges);
    let static_ram_bytes = union_size(&writable_load_ranges);
    if section_writable_bytes > static_ram_bytes {
        errors.push(format!(
            "writable section union {section_writable_bytes} exceeds writable PT_LOAD union {static_ram_bytes}"
        ));
    }
    budget_check(
        "stripped artifact",
        data.len() as u64,
        UNIT_LIMIT_BYTES,
        &mut errors,
    );
    budget_check(
        "PT_LOAD memory union",
        load_memory_union,
        LOAD_LIMIT_BYTES,
        &mut errors,
    );
    budget_check(
        "static writable RAM",
        static_ram_bytes,
        STATIC_RAM_LIMIT_BYTES,
        &mut errors,
    );

    if let Some(declared) = declared_sdram
        && (declared == 0 || declared > SDRAM_LIMIT_BYTES)
    {
        errors.push(format!(
            "declared SDRAM {declared} is outside 1..={SDRAM_LIMIT_BYTES} bytes"
        ));
    }
    if let Some(probe) = &probe {
        if let Some(declared) = declared_sdram {
            if probe.budget_bytes != declared {
                errors.push(format!(
                    "native probe budget {} does not match artifact declaration {declared}",
                    probe.budget_bytes
                ));
            }
            if probe.high_water_bytes > declared {
                errors.push(format!(
                    "native initialization high-water {} exceeds declared SDRAM {declared}",
                    probe.high_water_bytes
                ));
            }
            if probe.high_water_bytes != 0 {
                let required_margin = (declared * 2 / 100).max(16 * 1024);
                let actual_margin = declared.saturating_sub(probe.high_water_bytes);
                if actual_margin < required_margin {
                    errors.push(format!(
                        "native SDRAM margin {actual_margin} is below policy minimum {required_margin}"
                    ));
                }
            }
        }
        if probe.post_init_allocations != 0 {
            errors.push(format!(
                "post-initialization stress loop made {} allocations",
                probe.post_init_allocations
            ));
        }
    }

    let artifact_size = data.len() as u64;
    let measured_high_water = probe.as_ref().map(|value| value.high_water_bytes);
    let report = InspectionReport {
        schema_version: REPORT_SCHEMA_VERSION,
        policy: policy.to_owned(),
        status: if errors.is_empty() { "pass" } else { "fail" }.to_owned(),
        artifact: ArtifactReport {
            path: path.display().to_string(),
            sha256: format!("{:x}", Sha256::digest(&data)),
            stripped_bytes: artifact_size,
            limit_bytes: UNIT_LIMIT_BYTES,
            margin_bytes: margin(UNIT_LIMIT_BYTES, artifact_size),
        },
        elf: ElfReport {
            class: "ELF32".to_owned(),
            data: "little-endian".to_owned(),
            machine: "ARM".to_owned(),
            file_type: "ET_DYN".to_owned(),
            osabi: "System V".to_owned(),
            eabi: "EABI5".to_owned(),
            float_abi: "hard".to_owned(),
            target,
            api,
            unit_name,
            required_exports: CALLBACKS.len() + 1 + usize::from(declared_sdram.is_some()),
            relocation_count,
            unwind_bytes,
        },
        loads: LoadReport {
            segments: loads,
            file_union_bytes: load_file_union,
            memory_union_bytes: load_memory_union,
            limit_bytes: LOAD_LIMIT_BYTES,
            margin_bytes: margin(LOAD_LIMIT_BYTES, load_memory_union),
        },
        sections: breakdown,
        static_ram: BudgetReport {
            bytes: static_ram_bytes,
            limit_bytes: STATIC_RAM_LIMIT_BYTES,
            margin_bytes: margin(STATIC_RAM_LIMIT_BYTES, static_ram_bytes),
        },
        sdram: SdramReport {
            declared_bytes: declared_sdram,
            platform_limit_bytes: SDRAM_LIMIT_BYTES,
            declared_margin_bytes: declared_sdram.map(|value| margin(SDRAM_LIMIT_BYTES, value)),
            measured_high_water_bytes: measured_high_water,
            measured_margin_bytes: declared_sdram
                .zip(measured_high_water)
                .map(|(declared, measured)| margin(declared, measured)),
            measured_percent_of_platform: measured_high_water
                .map(|value| value as f64 * 100.0 / SDRAM_LIMIT_BYTES as f64),
        },
        native_probe: probe,
        stack: UnknownMeasurement {
            status: "unknown",
            reason: "the linker .stack sentinel is not a worst-case stack measurement",
        },
        realtime_cpu: UnknownMeasurement {
            status: "unknown",
            reason: "ELF inspection and native execution do not measure NTS-3 real-time CPU load",
        },
        provenance,
        warnings,
        errors,
    };
    Ok(report)
}

fn validate_ident(data: &[u8]) -> Result<(), String> {
    if data.len() < 16 {
        return Err("malformed ELF: file is shorter than e_ident".to_owned());
    }
    if data[0..4] != elf::ELFMAG {
        return Err("malformed ELF: bad magic".to_owned());
    }
    if data[4] != elf::ELFCLASS32 {
        return Err(format!(
            "wrong ELF class: expected ELF32, found {}",
            data[4]
        ));
    }
    if data[5] != elf::ELFDATA2LSB {
        return Err(format!(
            "wrong ELF data encoding: expected little-endian, found {}",
            data[5]
        ));
    }
    Ok(())
}

fn validate_header(
    header: &elf::FileHeader32<Endianness>,
    endian: Endianness,
    errors: &mut Vec<String>,
) {
    let ident = header.e_ident();
    if ident.os_abi != elf::ELFOSABI_SYSV || ident.abi_version != 0 {
        errors.push(format!(
            "wrong OSABI: expected System V ABI 0, found {}/{}",
            ident.os_abi, ident.abi_version
        ));
    }
    if header.e_type(endian) != elf::ET_DYN {
        errors.push(format!(
            "wrong ELF type: expected ET_DYN, found {}",
            header.e_type(endian)
        ));
    }
    if header.e_machine(endian) != elf::EM_ARM {
        errors.push(format!(
            "wrong machine: expected ARM, found {}",
            header.e_machine(endian)
        ));
    }
    let flags = header.e_flags(endian);
    if flags & elf::EF_ARM_EABIMASK != elf::EF_ARM_EABI_VER5 {
        errors.push(format!(
            "wrong ARM EABI flags: expected EABI5, found {flags:#010x}"
        ));
    }
    if flags & elf::EF_ARM_ABI_FLOAT_HARD == 0 || flags & elf::EF_ARM_ABI_FLOAT_SOFT != 0 {
        errors.push(format!(
            "wrong float ABI: expected hard-float, found flags {flags:#010x}"
        ));
    }
}

fn parse_unit_header(
    sections: &object::read::elf::SectionTable<'_, elf::FileHeader32<Endianness>>,
    endian: Endianness,
    data: &[u8],
    errors: &mut Vec<String>,
) -> Result<(u32, u32, String), String> {
    let Some((_index, section)) = sections.section_by_name(endian, b".unit_header") else {
        return Ok((0, 0, String::new()));
    };
    let bytes = section
        .data(endian, data)
        .map_err(|error| format!("malformed `.unit_header`: {error}"))?;
    if bytes.len() < GENERICFX_HEADER_BYTES as usize {
        errors.push(format!(
            "malformed `.unit_header`: expected at least {GENERICFX_HEADER_BYTES} bytes, found {}",
            bytes.len()
        ));
        return Ok((0, 0, String::new()));
    }
    let header_size = little_u32(bytes, 0);
    let target = little_u32(bytes, 4);
    let api = little_u32(bytes, 8);
    if header_size != GENERICFX_HEADER_BYTES as u32 {
        errors.push(format!(
            "wrong unit header size: expected 376, found {header_size}"
        ));
    }
    if target != EXPECTED_TARGET {
        errors.push(format!(
            "wrong unit target: expected {EXPECTED_TARGET:#06x}, found {target:#06x}"
        ));
    }
    if api != EXPECTED_API {
        errors.push(format!(
            "wrong unit API: expected {EXPECTED_API:#08x}, found {api:#08x}"
        ));
    }
    let name_bytes = &bytes[24..44];
    let name_end = name_bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name_bytes.len());
    let unit_name = String::from_utf8_lossy(&name_bytes[..name_end]).into_owned();
    Ok((target, api, unit_name))
}

fn parse_resources(
    sections: &object::read::elf::SectionTable<'_, elf::FileHeader32<Endianness>>,
    endian: Endianness,
    data: &[u8],
    errors: &mut Vec<String>,
) -> Result<Option<u64>, String> {
    let (_, section) = sections
        .section_by_name(endian, b".nts3_resources")
        .expect("caller counted one resource section");
    let bytes = section
        .data(endian, data)
        .map_err(|error| format!("malformed `.nts3_resources`: {error}"))?;
    if bytes.len() != RESOURCE_BYTES as usize {
        errors.push(format!(
            "malformed `.nts3_resources`: expected 12 bytes, found {}",
            bytes.len()
        ));
        return Ok(None);
    }
    if &bytes[0..4] != b"N3RS" {
        errors.push("malformed `.nts3_resources`: bad magic".to_owned());
        return Ok(None);
    }
    let schema = little_u16(bytes, 4);
    let record_size = little_u16(bytes, 6);
    if schema != 1 {
        errors.push(format!(
            "unsupported `.nts3_resources` schema version {schema}"
        ));
    }
    if record_size != RESOURCE_BYTES as u16 {
        errors.push(format!("wrong `.nts3_resources` record size {record_size}"));
    }
    Ok(Some(u64::from(little_u32(bytes, 8))))
}

fn validate_symbols(
    file: &object::read::elf::ElfFile32<'_, Endianness>,
    framework: bool,
    errors: &mut Vec<String>,
) -> Result<(), String> {
    let mut values = BTreeMap::<String, u64>::new();
    let mut counts = BTreeMap::<String, usize>::new();
    for symbol in file.dynamic_symbols() {
        let name = symbol
            .name()
            .map_err(|error| format!("malformed dynamic symbol name: {error}"))?;
        if symbol.is_undefined() && !name.is_empty() {
            errors.push(format!("unexpected undefined dynamic symbol `{name}`"));
        }
        *counts.entry(name.to_owned()).or_default() += 1;
        values.insert(name.to_owned(), symbol.address());
    }
    let mut required = vec!["unit_header"];
    required.extend_from_slice(CALLBACKS);
    if framework {
        required.push("nts3_resources");
    }
    for name in required {
        match counts.get(name).copied().unwrap_or(0) {
            1 => {}
            count => errors.push(format!(
                "required dynamic export `{name}` occurs {count} times"
            )),
        }
    }
    let mut callback_addresses = BTreeSet::new();
    for callback in CALLBACKS {
        if let Some(address) = values.get(*callback) {
            if address & 1 == 0 {
                errors.push(format!("callback `{callback}` lacks the Thumb bit"));
            }
            if !callback_addresses.insert(*address) {
                errors.push(format!(
                    "callback `{callback}` aliases another required callback"
                ));
            }
        }
    }
    Ok(())
}

fn validate_alloc_sections_in_loads(
    sections: &object::read::elf::SectionTable<'_, elf::FileHeader32<Endianness>>,
    endian: Endianness,
    load_ranges: &[(u64, u64)],
    names: &[String],
    errors: &mut Vec<String>,
) {
    for ((_, section), name) in sections.enumerate().zip(names) {
        let flags: u64 = section.sh_flags(endian).into();
        let size: u64 = section.sh_size(endian).into();
        if flags & u64::from(elf::SHF_ALLOC) == 0 || size == 0 {
            continue;
        }
        let start: u64 = section.sh_addr(endian).into();
        let Some(end) = start.checked_add(size) else {
            errors.push(format!(
                "allocated section `{name}` address range overflows"
            ));
            continue;
        };
        if !load_ranges
            .iter()
            .any(|(load_start, load_end)| start >= *load_start && end <= *load_end)
        {
            errors.push(format!(
                "allocated section `{name}` is not contained in one PT_LOAD"
            ));
        }
    }
}

fn little_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("checked record length"),
    )
}

fn little_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked record length"),
    )
}

fn declared_initializer_section(name: &str) -> bool {
    matches!(
        name,
        ".preinit_array" | ".init_array" | ".fini_array" | ".ctors" | ".dtors"
    )
}

fn program_flags(flags: u32) -> String {
    let mut value = String::new();
    if flags & elf::PF_R != 0 {
        value.push('R');
    }
    if flags & elf::PF_W != 0 {
        value.push('W');
    }
    if flags & elf::PF_X != 0 {
        value.push('X');
    }
    value
}

fn union_size(ranges: &[(u64, u64)]) -> u64 {
    let mut ranges = ranges
        .iter()
        .copied()
        .filter(|(start, end)| end > start)
        .collect::<Vec<_>>();
    ranges.sort_unstable();
    let mut total = 0_u64;
    let mut current: Option<(u64, u64)> = None;
    for (start, end) in ranges {
        match current {
            None => current = Some((start, end)),
            Some((current_start, current_end)) if start <= current_end => {
                current = Some((current_start, current_end.max(end)));
            }
            Some((current_start, current_end)) => {
                total = total.saturating_add(current_end - current_start);
                current = Some((start, end));
            }
        }
    }
    if let Some((start, end)) = current {
        total = total.saturating_add(end - start);
    }
    total
}

fn budget_check(label: &str, value: u64, limit: u64, errors: &mut Vec<String>) {
    if value > limit {
        errors.push(format!("{label} exceeds {limit} bytes: {value}"));
    }
}

fn margin(limit: u64, used: u64) -> i64 {
    i128::from(limit)
        .saturating_sub(i128::from(used))
        .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

pub fn write_reports(
    report: &InspectionReport,
    json_path: &Path,
    text_path: &Path,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|error| format!("cannot serialize memory report: {error}"))?;
    fs::write(json_path, format!("{json}\n"))
        .map_err(|error| format!("cannot write {}: {error}", json_path.display()))?;
    fs::write(text_path, human_report(report))
        .map_err(|error| format!("cannot write {}: {error}", text_path.display()))?;
    Ok(())
}

pub fn report_paths(artifact: &Path) -> (PathBuf, PathBuf) {
    let parent = artifact.parent().unwrap_or_else(|| Path::new("."));
    let stem = artifact
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("artifact");
    (
        parent.join(format!("{stem}.memory.json")),
        parent.join(format!("{stem}.memory.txt")),
    )
}

pub fn human_report(report: &InspectionReport) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "NTS-3 artifact inspection: {}\n",
        report.status.to_uppercase()
    ));
    output.push_str(&format!("Policy: {}\n", report.policy));
    output.push_str(&format!("Artifact: {}\n", report.artifact.path));
    output.push_str(&format!("SHA-256: {}\n", report.artifact.sha256));
    output.push_str(&format!(
        "Artifact bytes: {} / {} (margin {})\n",
        report.artifact.stripped_bytes, report.artifact.limit_bytes, report.artifact.margin_bytes
    ));
    output.push_str(&format!(
        "PT_LOAD file union: {} bytes; memory union: {} / {} (margin {})\n",
        report.loads.file_union_bytes,
        report.loads.memory_union_bytes,
        report.loads.limit_bytes,
        report.loads.margin_bytes
    ));
    for segment in &report.loads.segments {
        output.push_str(&format!(
            "  PT_LOAD[{}] file={:#x}..{:#x} memory={:#x}..{:#x} filesz={} memsz={} align={:#x} {}\n",
            segment.index,
            segment.offset,
            segment.file_end,
            segment.virtual_address,
            segment.memory_end,
            segment.file_bytes,
            segment.memory_bytes,
            segment.alignment,
            segment.flags
        ));
    }
    output.push_str(&format!(
        "Static writable RAM: {} / {} (margin {})\n",
        report.static_ram.bytes, report.static_ram.limit_bytes, report.static_ram.margin_bytes
    ));
    for section in &report.sections.writable {
        output.push_str(&format!(
            "  {}: {} memory bytes, {} file bytes\n",
            section.name, section.bytes, section.file_bytes
        ));
    }
    output.push_str(&format!(
        "Sections: .text={} .rodata={} .data={} .bss={} .got={} dynamic={} relocations={} header={} resources={} allocated_union={} load_overhead={}\n",
        report.sections.text_bytes,
        report.sections.rodata_bytes,
        report.sections.data_bytes,
        report.sections.bss_bytes,
        report.sections.got_bytes,
        report.sections.dynamic_bytes,
        report.sections.relocation_bytes,
        report.sections.unit_header_bytes,
        report.sections.resource_bytes,
        report.sections.allocated_memory_union_bytes,
        report.sections.load_overhead_bytes
    ));
    output.push_str("Allocated section breakdown (GNU size -A categories):\n");
    for section in &report.sections.allocated {
        output.push_str(&format!(
            "  {}: {} memory bytes, {} file bytes\n",
            section.name, section.bytes, section.file_bytes
        ));
    }
    match report.sdram.declared_bytes {
        Some(declared) => output.push_str(&format!(
            "Declared target SDRAM: {} / {} (margin {})\n",
            declared,
            report.sdram.platform_limit_bytes,
            report.sdram.declared_margin_bytes.unwrap_or_default()
        )),
        None => output
            .push_str("Declared target SDRAM: unknown (legacy artifact has no resource record)\n"),
    }
    if let Some(probe) = &report.native_probe {
        output.push_str(&format!(
            "Native sizes: Plugin={} Parameters={} Runtime={} bytes\n",
            probe.plugin_bytes, probe.parameters_bytes, probe.runtime_bytes
        ));
        output.push_str(&format!(
            "Measured initialization SDRAM: high-water={} requested={} padding={} allocations={} margin={} ({:.3}% of platform)\n",
            probe.high_water_bytes,
            probe.requested_bytes,
            probe.alignment_padding_bytes,
            probe.initialization_allocations,
            report.sdram.measured_margin_bytes.unwrap_or_default(),
            report.sdram.measured_percent_of_platform.unwrap_or_default()
        ));
        output.push_str(&format!(
            "Post-init stress: {} renders, {} new allocations\n",
            probe.render_iterations, probe.post_init_allocations
        ));
    } else {
        output.push_str("Native initialization probe: not run for standalone inspection\n");
    }
    output.push_str("Stack usage: unknown (not measured; linker sentinel is not usage)\n");
    output.push_str("Real-time CPU: unknown (requires NTS-3 hardware measurement)\n");
    output.push_str(&format!(
        "Provenance: Rust={} GNU_ARM_GCC={} binutils={} framework={} SDK={} FunDSP={}\n",
        report.provenance.rust,
        report.provenance.gnu_arm_gcc,
        report.provenance.gnu_binutils,
        report.provenance.framework_commit,
        report.provenance.sdk_commit,
        report.provenance.fundsp_version
    ));
    output.push_str(&format!(
        "Provenance hashes: Cargo.lock={} unit.ld={} base={}\n",
        report.provenance.cargo_lock_sha256,
        report.provenance.sdk_linker_script_sha256,
        report.provenance.container_base_image
    ));
    for warning in &report.warnings {
        output.push_str(&format!("WARNING: {warning}\n"));
    }
    for error in &report.errors {
        output.push_str(&format!("ERROR: {error}\n"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_ranges_do_not_double_count_overlap() {
        assert_eq!(union_size(&[(0, 10), (5, 12), (20, 30), (30, 40)]), 32);
    }

    #[test]
    fn malformed_ident_diagnostics_are_specific() {
        assert_eq!(
            validate_ident(b"bad").unwrap_err(),
            "malformed ELF: file is shorter than e_ident"
        );
        let mut bytes = [0_u8; 16];
        assert_eq!(
            validate_ident(&bytes).unwrap_err(),
            "malformed ELF: bad magic"
        );
        bytes[0..4].copy_from_slice(&elf::ELFMAG);
        bytes[4] = elf::ELFCLASS64;
        bytes[5] = elf::ELFDATA2LSB;
        assert!(
            validate_ident(&bytes)
                .unwrap_err()
                .contains("wrong ELF class")
        );
        bytes[4] = elf::ELFCLASS32;
        bytes[5] = elf::ELFDATA2MSB;
        assert!(
            validate_ident(&bytes)
                .unwrap_err()
                .contains("wrong ELF data encoding")
        );
    }

    #[test]
    fn json_schema_snapshot_has_stable_top_level_order() {
        let report = InspectionReport {
            schema_version: 1,
            policy: "test".to_owned(),
            status: "pass".to_owned(),
            artifact: ArtifactReport {
                path: "fixture.nts3unit".to_owned(),
                sha256: "00".to_owned(),
                stripped_bytes: 1,
                limit_bytes: 2,
                margin_bytes: 1,
            },
            elf: ElfReport {
                class: "ELF32".to_owned(),
                data: "little-endian".to_owned(),
                machine: "ARM".to_owned(),
                file_type: "ET_DYN".to_owned(),
                osabi: "System V".to_owned(),
                eabi: "EABI5".to_owned(),
                float_abi: "hard".to_owned(),
                target: EXPECTED_TARGET,
                api: EXPECTED_API,
                unit_name: "Fixture".to_owned(),
                required_exports: 14,
                relocation_count: 0,
                unwind_bytes: 0,
            },
            loads: LoadReport {
                segments: vec![],
                file_union_bytes: 1,
                memory_union_bytes: 1,
                limit_bytes: 2,
                margin_bytes: 1,
            },
            sections: SectionReport {
                text_bytes: 0,
                rodata_bytes: 0,
                data_bytes: 0,
                bss_bytes: 0,
                got_bytes: 0,
                dynamic_bytes: 0,
                relocation_bytes: 0,
                unit_header_bytes: 376,
                resource_bytes: 12,
                allocated_memory_union_bytes: 0,
                load_overhead_bytes: 1,
                allocated: vec![],
                writable: vec![],
            },
            static_ram: BudgetReport {
                bytes: 0,
                limit_bytes: 2,
                margin_bytes: 2,
            },
            sdram: SdramReport {
                declared_bytes: Some(1),
                platform_limit_bytes: SDRAM_LIMIT_BYTES,
                declared_margin_bytes: Some(SDRAM_LIMIT_BYTES as i64 - 1),
                measured_high_water_bytes: None,
                measured_margin_bytes: None,
                measured_percent_of_platform: None,
            },
            native_probe: None,
            stack: UnknownMeasurement {
                status: "unknown",
                reason: "test",
            },
            realtime_cpu: UnknownMeasurement {
                status: "unknown",
                reason: "test",
            },
            provenance: Provenance::default(),
            warnings: vec![],
            errors: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.starts_with(
            "{\"schema_version\":1,\"policy\":\"test\",\"status\":\"pass\",\"artifact\":"
        ));
        assert!(json.contains("\"stack\":{\"status\":\"unknown\""));
    }
}
