#![cfg(unix)]

use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    root: PathBuf,
    bin: PathBuf,
    metadata: PathBuf,
    archive: PathBuf,
    rlib: PathBuf,
    host_rlib: PathBuf,
    log: PathBuf,
    valid_elf: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nts3 cli fake tools {unique}"));
        let bin = root.join("fake tools");
        let target = root.join("target output");
        let archive = target.join("nts3/.build/fake_plugin/libnts3_plugin_build.a");
        let rlib = target.join("thumbv7em-none-eabihf/release/deps/libfake_plugin-12345678.rlib");
        let host_rlib = target.join("release/deps/libfake_plugin-12345678.rlib");
        let metadata = root.join("metadata.json");
        let log = root.join("tool.log");
        let valid_elf = root.join("valid.nts3unit");
        fs::create_dir_all(root.join("external/logue-sdk/platform/nts-3_kaoss/ld")).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::write(&valid_elf, minimal_valid_elf()).unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[]\n").unwrap();
        fs::write(
            root.join("external/logue-sdk/platform/nts-3_kaoss/ld/unit.ld"),
            "SECTIONS {}\n",
        )
        .unwrap();
        let value = json!({
            "packages": [{
                "name": "fake-plugin",
                "manifest_path": root.join("plugin/Cargo.toml"),
                "targets": [{"name": "fake_plugin", "kind": ["lib"]}]
            }],
            "workspace_root": root,
            "target_directory": target
        });
        fs::write(&metadata, serde_json::to_vec(&value).unwrap()).unwrap();
        write_script(
            &bin.join("cargo"),
            r#"#!/usr/bin/env bash
set -euo pipefail
printf 'cargo %s\n' "$*" >>"${FAKE_LOG}"
if [[ "$1" == metadata ]]; then cat "${FAKE_METADATA}"; exit 0; fi
if [[ "$1" == check ]]; then exit "${FAKE_CHECK_STATUS:-0}"; fi
if [[ "$1" == build ]]; then mkdir -p "$(dirname "${FAKE_RLIB}")"; printf rlib >"${FAKE_RLIB}"; exit 0; fi
if [[ "$1" == rustc ]]; then mkdir -p "$(dirname "${FAKE_HOST_RLIB}")"; printf rlib >"${FAKE_HOST_RLIB}"; exit 0; fi
exit 91
"#,
        );
        write_script(
            &bin.join("rustc"),
            r#"#!/usr/bin/env bash
set -euo pipefail
printf 'rustc %s\n' "$*" >>"${FAKE_LOG}"
out=''
source=''
for arg in "$@"; do
  [[ "$arg" == *.rs ]] && source="$arg"
  if [[ "$out" == next ]]; then
    mkdir -p "$(dirname "$arg")"
    if [[ "$source" == *probe.rs ]]; then
      cat >"$arg" <<'EOF'
#!/usr/bin/env bash
echo 'NTS3_PROBE_V1 0 0 1 1 0 0 0 0 0 256'
EOF
      chmod +x "$arg"
    else
      printf archive >"$arg"
    fi
    out=''
  fi
  if [[ "$arg" == -o ]]; then out=next; fi
done
"#,
        );
        write_script(
            &bin.join("gcc"),
            r#"#!/usr/bin/env bash
set -euo pipefail
printf 'gcc %s\n' "$*" >>"${FAKE_LOG}"
out=''
for arg in "$@"; do
  case "$arg" in
    -Wl,-Map=*,--cref) map="${arg#-Wl,-Map=}"; map="${map%,--cref}"; printf map >"$map" ;;
  esac
  if [[ "$out" == next ]]; then cp "${FAKE_ELF}" "$arg"; out=''; fi
  if [[ "$arg" == -o ]]; then out=next; fi
done
"#,
        );
        write_script(
            &bin.join("strip"),
            "#!/usr/bin/env bash\nset -euo pipefail\nprintf 'strip %s\\n' \"$*\" >>\"${FAKE_LOG}\"\n",
        );
        write_script(
            &bin.join("report"),
            r#"#!/usr/bin/env bash
set -euo pipefail
printf 'report %s\n' "$*" >>"${FAKE_LOG}"
if [[ "$1" == -hAWSlrds ]]; then
cat <<'EOF'
Class:                             ELF32
Data:                              2's complement, little endian
OS/ABI:                            UNIX - System V
Type:                              DYN (Shared object file)
Machine:                           ARM
Version5 EABI, hard-float ABI
  Tag_FP_arch: VFPv4-D16
EOF
address=3
for symbol in unit_init unit_teardown unit_reset unit_resume unit_suspend unit_render unit_get_param_value unit_get_param_str_value unit_set_param_value unit_set_tempo unit_tempo_4ppqn_tick unit_touch_event; do
  printf '1: %08x 4 FUNC GLOBAL DEFAULT 1 %s\n' "$address" "$symbol"
  address=$((address + 2))
done
elif [[ "$1" == -D ]]; then
address=1
for symbol in unit_header unit_init unit_teardown unit_reset unit_resume unit_suspend unit_render unit_get_param_value unit_get_param_str_value unit_set_param_value unit_set_tempo unit_tempo_4ppqn_tick unit_touch_event nts3_resources; do
  printf '%08x T %s\n' "$address" "$symbol"
  address=$((address + 2))
done
else
  printf 'fake section 1\n'
fi
"#,
        );
        Self {
            root,
            bin,
            metadata,
            archive,
            rlib,
            host_rlib,
            log,
            valid_elf,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nts3-cli"));
        command
            .current_dir(&self.root)
            .env("NTS3_CARGO", self.bin.join("cargo"))
            .env("NTS3_CC", self.bin.join("gcc"))
            .env("NTS3_RUSTC", self.bin.join("rustc"))
            .env("NTS3_STRIP", self.bin.join("strip"))
            .env("NTS3_READELF", self.bin.join("report"))
            .env("NTS3_NM", self.bin.join("report"))
            .env("NTS3_SIZE", self.bin.join("report"))
            .env("FAKE_METADATA", &self.metadata)
            .env("FAKE_ARCHIVE", &self.archive)
            .env("FAKE_RLIB", &self.rlib)
            .env("FAKE_HOST_RLIB", &self.host_rlib)
            .env("FAKE_LOG", &self.log)
            .env("FAKE_ELF", &self.valid_elf);
        command
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_script(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn minimal_valid_elf() -> Vec<u8> {
    const PHOFF: usize = 52;
    const PHNUM: usize = 3;
    const DYNSYM_OFF: usize = PHOFF + PHNUM * 32;
    let exports = [
        "unit_header",
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
        "nts3_resources",
    ];
    let dynsym_size = (exports.len() + 1) * 16;
    let dynstr_off = DYNSYM_OFF + dynsym_size;
    let mut dynstr = vec![0_u8];
    let mut symbol_names = Vec::new();
    for name in exports {
        symbol_names.push(dynstr.len() as u32);
        dynstr.extend_from_slice(name.as_bytes());
        dynstr.push(0);
    }
    let text_off = align(dynstr_off + dynstr.len(), 4);
    let text_size = 32;
    let header_off = align(text_off + text_size, 128);
    let resource_off = header_off + 376;
    let rw_off = align(resource_off + 12, 128);
    let shstr = b"\0.dynsym\0.dynstr\0.text\0.unit_header\0.nts3_resources\0.bss\0.shstrtab\0";
    let shstr_off = rw_off;
    let shoff = align(shstr_off + shstr.len(), 4);
    let section_count = 8;
    let mut bytes = vec![0_u8; shoff + section_count * 40];

    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 1;
    bytes[5] = 1;
    bytes[6] = 1;
    put16(&mut bytes, 16, 3);
    put16(&mut bytes, 18, 40);
    put32(&mut bytes, 20, 1);
    put32(&mut bytes, 28, PHOFF as u32);
    put32(&mut bytes, 32, shoff as u32);
    put32(&mut bytes, 36, 0x0500_0400);
    put16(&mut bytes, 40, 52);
    put16(&mut bytes, 42, 32);
    put16(&mut bytes, 44, PHNUM as u16);
    put16(&mut bytes, 46, 40);
    put16(&mut bytes, 48, section_count as u16);
    put16(&mut bytes, 50, 7);

    program(
        &mut bytes,
        PHOFF,
        1,
        0,
        0,
        (text_off + text_size) as u32,
        (text_off + text_size) as u32,
        5,
        128,
    );
    program(
        &mut bytes,
        PHOFF + 32,
        1,
        header_off as u32,
        header_off as u32,
        388,
        388,
        4,
        128,
    );
    program(
        &mut bytes,
        PHOFF + 64,
        1,
        rw_off as u32,
        rw_off as u32,
        0,
        4,
        6,
        128,
    );

    bytes[dynstr_off..dynstr_off + dynstr.len()].copy_from_slice(&dynstr);
    for (index, name) in exports.iter().enumerate() {
        let offset = DYNSYM_OFF + (index + 1) * 16;
        put32(&mut bytes, offset, symbol_names[index]);
        let (value, size, info, section) = if *name == "unit_header" {
            (header_off as u32, 376, 0x11, 4)
        } else if *name == "nts3_resources" {
            (resource_off as u32, 12, 0x11, 5)
        } else {
            ((text_off as u32 + index as u32 * 2) | 1, 2, 0x12, 3)
        };
        put32(&mut bytes, offset + 4, value);
        put32(&mut bytes, offset + 8, size);
        bytes[offset + 12] = info;
        put16(&mut bytes, offset + 14, section);
    }
    put32(&mut bytes, header_off, 376);
    put32(&mut bytes, header_off + 4, 0x0607);
    put32(&mut bytes, header_off + 8, 0x0002_0000);
    bytes[header_off + 24..header_off + 32].copy_from_slice(b"Fixture\0");
    bytes[resource_off..resource_off + 4].copy_from_slice(b"N3RS");
    put16(&mut bytes, resource_off + 4, 1);
    put16(&mut bytes, resource_off + 6, 12);
    put32(&mut bytes, resource_off + 8, 1);
    bytes[shstr_off..shstr_off + shstr.len()].copy_from_slice(shstr);

    section(
        &mut bytes,
        shoff + 40,
        1,
        11,
        2,
        DYNSYM_OFF as u32,
        DYNSYM_OFF as u32,
        dynsym_size as u32,
        2,
        1,
        4,
        16,
    );
    section(
        &mut bytes,
        shoff + 80,
        9,
        3,
        2,
        dynstr_off as u32,
        dynstr_off as u32,
        dynstr.len() as u32,
        0,
        0,
        1,
        0,
    );
    section(
        &mut bytes,
        shoff + 120,
        17,
        1,
        6,
        text_off as u32,
        text_off as u32,
        text_size as u32,
        0,
        0,
        2,
        0,
    );
    section(
        &mut bytes,
        shoff + 160,
        23,
        1,
        2,
        header_off as u32,
        header_off as u32,
        376,
        0,
        0,
        8,
        0,
    );
    section(
        &mut bytes,
        shoff + 200,
        36,
        1,
        2,
        resource_off as u32,
        resource_off as u32,
        12,
        0,
        0,
        1,
        0,
    );
    section(
        &mut bytes,
        shoff + 240,
        52,
        8,
        3,
        rw_off as u32,
        rw_off as u32,
        4,
        0,
        0,
        4,
        0,
    );
    section(
        &mut bytes,
        shoff + 280,
        57,
        3,
        0,
        0,
        shstr_off as u32,
        shstr.len() as u32,
        0,
        0,
        1,
        0,
    );
    bytes
}

fn align(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

fn put16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[allow(clippy::too_many_arguments)]
fn program(
    bytes: &mut [u8],
    offset: usize,
    kind: u32,
    file: u32,
    address: u32,
    file_size: u32,
    memory_size: u32,
    flags: u32,
    alignment: u32,
) {
    put32(bytes, offset, kind);
    put32(bytes, offset + 4, file);
    put32(bytes, offset + 8, address);
    put32(bytes, offset + 12, address);
    put32(bytes, offset + 16, file_size);
    put32(bytes, offset + 20, memory_size);
    put32(bytes, offset + 24, flags);
    put32(bytes, offset + 28, alignment);
}

#[allow(clippy::too_many_arguments)]
fn section(
    bytes: &mut [u8],
    offset: usize,
    name: u32,
    kind: u32,
    flags: u32,
    address: u32,
    file: u32,
    size: u32,
    link: u32,
    info: u32,
    alignment: u32,
    entry_size: u32,
) {
    for (index, value) in [
        name, kind, flags, address, file, size, link, info, alignment, entry_size,
    ]
    .into_iter()
    .enumerate()
    {
        put32(bytes, offset + index * 4, value);
    }
}

#[test]
fn fake_cargo_and_gnu_tools_produce_deterministic_outputs_and_transcript() {
    let fixture = Fixture::new();
    let status = fixture
        .command()
        .args(["build", "-p", "fake-plugin", "--release", "--verbose"])
        .status()
        .unwrap();
    assert!(status.success());

    let output = fixture.root.join("target output/nts3");
    for suffix in [
        "nts3unit",
        "elf",
        "map",
        "readelf.txt",
        "nm.txt",
        "size.txt",
        "commands.txt",
        "memory.json",
        "memory.txt",
    ] {
        assert!(output.join(format!("fake_plugin.{suffix}")).is_file());
    }
    let transcript = fs::read_to_string(output.join("fake_plugin.commands.txt")).unwrap();
    assert!(transcript.contains("--crate-type=staticlib"));
    assert!(transcript.contains("--undefined=unit_header"));
    assert!(transcript.contains("--undefined=nts3_resources"));
    assert!(transcript.contains("relocation-model=pic"));
    assert!(!transcript.contains("target-cpu=cortex-m7"));
    let log = fs::read_to_string(&fixture.log).unwrap();
    assert!(log.contains("cargo build --locked -p fake-plugin"));
    assert!(log.contains("rustc --crate-name nts3_plugin_build"));
    assert!(log.contains("gcc "));
}

#[test]
fn malformed_elf_invariants_have_specific_diagnostics() {
    let fixture = Fixture::new();
    let base = fs::read(&fixture.valid_elf).unwrap();
    let resource = find_bytes(&base, b"N3RS");
    let unit_header = resource - 376;
    let shstr = find_bytes(&base, b".dynsym\0.dynstr\0.text\0.unit_header");
    let section_headers = read32(&base, 32) as usize;
    let unit_init_name = read32(&base, DYNSYM_FOR_TEST + 2 * 16);
    let unit_init_value = read32(&base, DYNSYM_FOR_TEST + 2 * 16 + 4);
    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        ("class", changed(&base, 4, &[2]), "wrong ELF class"),
        ("data", changed(&base, 5, &[2]), "wrong ELF data encoding"),
        ("osabi", changed(&base, 7, &[3]), "wrong OSABI"),
        (
            "type",
            changed(&base, 16, &1_u16.to_le_bytes()),
            "wrong ELF type",
        ),
        (
            "machine",
            changed(&base, 18, &3_u16.to_le_bytes()),
            "wrong machine",
        ),
        (
            "eabi",
            changed(&base, 36, &0x0400_0400_u32.to_le_bytes()),
            "wrong ARM EABI",
        ),
        (
            "float",
            changed(&base, 36, &0x0500_0200_u32.to_le_bytes()),
            "wrong float ABI",
        ),
        (
            "target",
            changed(&base, unit_header + 4, &0_u32.to_le_bytes()),
            "wrong unit target",
        ),
        (
            "api",
            changed(&base, unit_header + 8, &0_u32.to_le_bytes()),
            "wrong unit API",
        ),
        (
            "resource-schema",
            changed(&base, resource + 4, &2_u16.to_le_bytes()),
            "unsupported `.nts3_resources` schema",
        ),
        (
            "sdram",
            changed(
                &base,
                resource + 8,
                &(3 * 1024 * 1024 + 1_u32).to_le_bytes(),
            ),
            "declared SDRAM",
        ),
        (
            "alignment",
            changed(&base, PHOFF_FOR_TEST + 28, &256_u32.to_le_bytes()),
            "alignment 0x100",
        ),
        (
            "undefined",
            changed(&base, DYNSYM_FOR_TEST + 2 * 16 + 14, &0_u16.to_le_bytes()),
            "unexpected undefined dynamic symbol `unit_init`",
        ),
        (
            "thumb",
            changed(
                &base,
                DYNSYM_FOR_TEST + 2 * 16 + 4,
                &0x200_u32.to_le_bytes(),
            ),
            "lacks the Thumb bit",
        ),
        (
            "missing-header",
            changed(&base, shstr + 22, b".bad_header\0\0"),
            "expected exactly one `.unit_header`",
        ),
        (
            "plt",
            changed(&base, shstr + 16, b".plt\0\0"),
            "non-empty PLT section",
        ),
        (
            "debug",
            changed(&base, shstr + 8, b".debug\0"),
            "prohibited debug/unstripped section `.debug`",
        ),
        (
            "unwind",
            changed(&base, shstr + 35, b".ARM.extab\0\0\0\0\0"),
            "unsupported unwind section `.ARM.extab`",
        ),
        (
            "relocation",
            changed(&base, section_headers + 6 * 40 + 4, &9_u32.to_le_bytes()),
            "malformed relocations in `.bss`",
        ),
        (
            "duplicate-export",
            changed(
                &base,
                DYNSYM_FOR_TEST + 3 * 16,
                &unit_init_name.to_le_bytes(),
            ),
            "required dynamic export `unit_init` occurs 2 times",
        ),
        (
            "callback-alias",
            changed(
                &base,
                DYNSYM_FOR_TEST + 3 * 16 + 4,
                &unit_init_value.to_le_bytes(),
            ),
            "aliases another required callback",
        ),
    ];
    for (name, bytes, diagnostic) in cases {
        assert_rejected(&fixture, name, &bytes, diagnostic);
    }

    let mut oversized = base.clone();
    oversized.resize(32 * 1024 + 1, 0);
    assert_rejected(
        &fixture,
        "artifact-limit",
        &oversized,
        "stripped artifact exceeds 32768 bytes",
    );
    let load_over = changed(
        &base,
        PHOFF_FOR_TEST + 20,
        &(32 * 1024 + 1_u32).to_le_bytes(),
    );
    assert_rejected(
        &fixture,
        "load-limit",
        &load_over,
        "PT_LOAD memory union exceeds 32768 bytes",
    );
    let ram_over = changed(
        &base,
        PHOFF_FOR_TEST + 64 + 20,
        &(32 * 1024 + 1_u32).to_le_bytes(),
    );
    assert_rejected(
        &fixture,
        "ram-limit",
        &ram_over,
        "static writable RAM exceeds 32768 bytes",
    );
}

const PHOFF_FOR_TEST: usize = 52;
const DYNSYM_FOR_TEST: usize = PHOFF_FOR_TEST + 3 * 32;

fn changed(original: &[u8], offset: usize, replacement: &[u8]) -> Vec<u8> {
    let mut bytes = original.to_vec();
    bytes[offset..offset + replacement.len()].copy_from_slice(replacement);
    bytes
}

fn read32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap()
}

fn assert_rejected(fixture: &Fixture, name: &str, bytes: &[u8], diagnostic: &str) {
    let path = fixture.root.join(format!("invalid-{name}.nts3unit"));
    fs::write(&path, bytes).unwrap();
    let output = fixture
        .command()
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success(), "{name} unexpectedly passed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(diagnostic),
        "{name}: expected `{diagnostic}` in `{stderr}`"
    );
}

#[test]
fn cargo_failure_status_is_forwarded_exactly() {
    let fixture = Fixture::new();
    let status = fixture
        .command()
        .env("FAKE_CHECK_STATUS", "43")
        .args(["check", "-p", "fake-plugin"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(43));
}
