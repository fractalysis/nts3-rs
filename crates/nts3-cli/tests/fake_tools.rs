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
    log: PathBuf,
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
        let metadata = root.join("metadata.json");
        let log = root.join("tool.log");
        fs::create_dir_all(root.join("external/logue-sdk/platform/nts-3_kaoss/ld")).unwrap();
        fs::create_dir_all(&bin).unwrap();
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
exit 91
"#,
        );
        write_script(
            &bin.join("rustc"),
            r#"#!/usr/bin/env bash
set -euo pipefail
printf 'rustc %s\n' "$*" >>"${FAKE_LOG}"
out=''
for arg in "$@"; do
  if [[ "$out" == next ]]; then mkdir -p "$(dirname "$arg")"; printf archive >"$arg"; out=''; fi
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
  if [[ "$out" == next ]]; then printf elf >"$arg"; out=''; fi
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
            log,
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
            .env("FAKE_LOG", &self.log);
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
    ] {
        assert!(output.join(format!("fake_plugin.{suffix}")).is_file());
    }
    let transcript = fs::read_to_string(output.join("fake_plugin.commands.txt")).unwrap();
    assert!(transcript.contains("--crate-type=staticlib"));
    assert!(transcript.contains("--undefined=unit_header"));
    assert!(transcript.contains("--undefined=nts3_resources"));
    assert!(transcript.contains("target-cpu=cortex-m7"));
    let log = fs::read_to_string(&fixture.log).unwrap();
    assert!(log.contains("cargo build --locked -p fake-plugin"));
    assert!(log.contains("rustc --crate-name nts3_plugin_build"));
    assert!(log.contains("gcc "));
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
