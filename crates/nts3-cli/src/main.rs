mod inspector;

use inspector::{ProbeReport, Provenance};
use serde::Deserialize;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

const TARGET: &str = "thumbv7em-none-eabihf";
const RUST_VERSION: &str = "1.98.1";
const GCC_VERSION: &str = "9.2.1 20191025";
const BINUTILS_VERSION: &str = "2.34";
const EXPORTS: &[&str] = &[
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

#[derive(Debug)]
struct CliError {
    code: i32,
    message: String,
}

type Result<T> = std::result::Result<T, CliError>;

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: 2,
            message: message.into(),
        }
    }

    fn failed(message: impl Into<String>) -> Self {
        Self {
            code: 1,
            message: message.into(),
        }
    }
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_root: PathBuf,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    manifest_path: PathBuf,
    targets: Vec<TargetInfo>,
}

#[derive(Deserialize)]
struct TargetInfo {
    name: String,
    kind: Vec<String>,
    #[serde(default)]
    crate_types: Vec<String>,
}

struct SelectedPackage {
    name: String,
    lib_name: String,
    is_staticlib: bool,
}

struct Transcript {
    file: fs::File,
    verbose: bool,
}

impl Transcript {
    fn create(path: &Path, verbose: bool) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                CliError::failed(format!("cannot create {}: {error}", parent.display()))
            })?;
        }
        let file = fs::File::create(path).map_err(|error| {
            CliError::failed(format!(
                "cannot create transcript {}: {error}",
                path.display()
            ))
        })?;
        Ok(Self { file, verbose })
    }

    fn record(
        &mut self,
        program: &OsStr,
        args: &[OsString],
        envs: &[(&str, OsString)],
    ) -> Result<()> {
        let mut line = String::new();
        for (key, value) in envs {
            line.push_str(key);
            line.push('=');
            line.push_str(&shell_quote(value));
            line.push(' ');
        }
        line.push_str(&shell_quote(program));
        for arg in args {
            line.push(' ');
            line.push_str(&shell_quote(arg));
        }
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .and_then(|_| self.file.flush())
            .map_err(|error| {
                CliError::failed(format!("cannot write command transcript: {error}"))
            })?;
        if self.verbose {
            eprint!("{line}");
        }
        Ok(())
    }
}

fn shell_quote(value: &OsStr) -> String {
    let value = value.to_string_lossy();
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "_+-./:=,@%".contains(ch))
    {
        value.into_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn tool(variable: &str, default: &str) -> OsString {
    env::var_os(variable).unwrap_or_else(|| OsString::from(default))
}

fn rust_env(workspace_root: &Path, release: bool) -> Vec<(&'static str, OsString)> {
    let flags = [
        "-C".to_owned(),
        "target-cpu=cortex-m7".to_owned(),
        "-C".to_owned(),
        "relocation-model=pic".to_owned(),
        "-C".to_owned(),
        "panic=abort".to_owned(),
        "-C".to_owned(),
        "force-unwind-tables=no".to_owned(),
        "-C".to_owned(),
        "llvm-args=-mergefunc-use-aliases=0".to_owned(),
        format!(
            "--remap-path-prefix={}=<WORKSPACE>",
            workspace_root.display()
        ),
    ]
    .join("\u{1f}");
    let mut envs = vec![
        ("CARGO_ENCODED_RUSTFLAGS", OsString::from(flags)),
        ("CARGO_INCREMENTAL", OsString::from("0")),
        ("SOURCE_DATE_EPOCH", OsString::from("0")),
    ];
    if release {
        envs.extend([
            ("CARGO_PROFILE_RELEASE_OPT_LEVEL", OsString::from("z")),
            ("CARGO_PROFILE_RELEASE_LTO", OsString::from("fat")),
            ("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", OsString::from("1")),
            ("CARGO_PROFILE_RELEASE_PANIC", OsString::from("abort")),
            ("CARGO_PROFILE_RELEASE_STRIP", OsString::from("none")),
            ("CARGO_PROFILE_RELEASE_INCREMENTAL", OsString::from("false")),
        ]);
    }
    envs
}

fn status_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    1
}

fn run_command(
    program: &OsStr,
    args: &[OsString],
    envs: &[(&str, OsString)],
    transcript: Option<&mut Transcript>,
) -> Result<()> {
    if let Some(transcript) = transcript {
        transcript.record(program, args, envs)?;
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .envs(envs.iter().map(|(key, value)| (*key, value)));
    let status = command.status().map_err(|error| {
        CliError::failed(format!(
            "failed to launch {}: {error}",
            program.to_string_lossy()
        ))
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(CliError {
            code: status_code(status),
            message: format!("{} failed", program.to_string_lossy()),
        })
    }
}

fn capture(program: &OsStr, args: &[&str], label: &str) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            CliError::failed(format!("in-container check failed ({label}): {error}"))
        })?;
    if !output.status.success() {
        return Err(CliError {
            code: status_code(output.status),
            message: format!(
                "in-container check failed ({label}): {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn metadata() -> Result<Metadata> {
    let cargo = tool("NTS3_CARGO", "cargo");
    let output = Command::new(&cargo)
        .args(["metadata", "--locked", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|error| CliError::failed(format!("failed to launch Cargo metadata: {error}")))?;
    if !output.status.success() {
        return Err(CliError {
            code: status_code(output.status),
            message: "Cargo metadata failed".to_owned(),
        });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| CliError::failed(format!("invalid Cargo metadata: {error}")))
}

fn select_package(metadata: &Metadata, requested: Option<&str>) -> Result<SelectedPackage> {
    let package = if let Some(requested) = requested {
        metadata
            .packages
            .iter()
            .find(|package| package.name == requested)
            .ok_or_else(|| {
                CliError::usage(format!("workspace package `{requested}` was not found"))
            })?
    } else {
        return Err(CliError::usage(
            "select a plugin package with `-p <package>`",
        ));
    };
    let target = package
        .targets
        .iter()
        .find(|target| {
            target
                .kind
                .iter()
                .any(|kind| kind == "lib" || kind == "staticlib")
        })
        .ok_or_else(|| {
            CliError::usage(format!(
                "package `{}` has no library target ({})",
                package.name,
                package.manifest_path.display()
            ))
        })?;
    Ok(SelectedPackage {
        name: package.name.clone(),
        lib_name: target.name.replace('-', "_"),
        is_staticlib: target.crate_types.iter().any(|kind| kind == "staticlib"),
    })
}

fn parse_package_args(args: &[String], build: bool) -> Result<(String, bool)> {
    let mut package = None;
    let mut verbose = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-p" | "--package" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| CliError::usage("missing package name after -p"))?;
                if package.replace(value.clone()).is_some() {
                    return Err(CliError::usage("package was specified more than once"));
                }
            }
            "--release" if build => {}
            "--verbose" | "-v" => verbose = true,
            other => return Err(CliError::usage(format!("unknown argument `{other}`"))),
        }
        index += 1;
    }
    let package =
        package.ok_or_else(|| CliError::usage("select a plugin package with `-p <package>`"))?;
    Ok((package, verbose))
}

fn check(args: &[String]) -> Result<()> {
    let (package, verbose) = parse_package_args(args, false)?;
    let metadata = metadata()?;
    let selected = select_package(&metadata, Some(&package))?;
    let transcript_path = metadata
        .target_directory
        .join("nts3")
        .join(format!("{}.check.commands.txt", selected.lib_name));
    let mut transcript = Transcript::create(&transcript_path, verbose)?;
    let cargo = tool("NTS3_CARGO", "cargo");
    let command_args = strings(&[
        "check",
        "--locked",
        "-p",
        &selected.name,
        "--target",
        TARGET,
    ]);
    let envs = rust_env(&metadata.workspace_root, false);
    run_command(&cargo, &command_args, &envs, Some(&mut transcript))?;
    println!("target check passed: {}", selected.name);
    println!("command transcript: {}", transcript_path.display());
    Ok(())
}

fn build(args: &[String]) -> Result<()> {
    let (package, verbose) = parse_package_args(args, true)?;
    let metadata = metadata()?;
    let selected = select_package(&metadata, Some(&package))?;
    let output_dir = metadata.target_directory.join("nts3");
    fs::create_dir_all(&output_dir).map_err(|error| {
        CliError::failed(format!("cannot create {}: {error}", output_dir.display()))
    })?;
    let stem = &selected.lib_name;
    let wrapper_directory = output_dir.join(".build").join(stem);
    let wrapper_source = wrapper_directory.join("wrapper.rs");
    let archive = if selected.is_staticlib {
        metadata
            .target_directory
            .join(TARGET)
            .join("release")
            .join(format!("lib{stem}.a"))
    } else {
        fs::create_dir_all(&wrapper_directory).map_err(|error| {
            CliError::failed(format!(
                "cannot create {}: {error}",
                wrapper_directory.display()
            ))
        })?;
        fs::write(&wrapper_source, "#![no_std]\nextern crate plugin;\n")
            .map_err(|error| CliError::failed(format!("cannot write build wrapper: {error}")))?;
        wrapper_directory.join("libnts3_plugin_build.a")
    };
    let elf = output_dir.join(format!("{stem}.elf"));
    let unit = output_dir.join(format!("{stem}.nts3unit"));
    let map = output_dir.join(format!("{stem}.map"));
    let transcript_path = output_dir.join(format!("{stem}.commands.txt"));
    for path in [&archive, &elf, &unit, &map] {
        if path == &archive {
            continue;
        }
        let _ = fs::remove_file(path);
    }

    let mut transcript = Transcript::create(&transcript_path, verbose)?;
    let cargo = tool("NTS3_CARGO", "cargo");
    let cargo_args = strings(&[
        "build",
        "--locked",
        "-p",
        &selected.name,
        "--lib",
        "--target",
        TARGET,
        "--release",
    ]);
    let dependency_dir = metadata.target_directory.join(TARGET).join("release/deps");
    if !selected.is_staticlib {
        remove_matching_rlibs(&dependency_dir, stem)?;
    }
    let envs = rust_env(&metadata.workspace_root, true);
    run_command(&cargo, &cargo_args, &envs, Some(&mut transcript))?;
    if !selected.is_staticlib {
        let plugin_rlib = find_rlib(&dependency_dir, stem)?;
        let rustc = tool("NTS3_RUSTC", "rustc");
        let rustc_args = vec![
            OsString::from("--crate-name"),
            OsString::from("nts3_plugin_build"),
            OsString::from("--edition=2024"),
            OsString::from("--target"),
            OsString::from(TARGET),
            OsString::from("--crate-type=staticlib"),
            wrapper_source.as_os_str().to_owned(),
            OsString::from("--extern"),
            OsString::from(format!("plugin={}", plugin_rlib.display())),
            OsString::from("-L"),
            OsString::from(format!("dependency={}", dependency_dir.display())),
            OsString::from("-L"),
            OsString::from(format!(
                "dependency={}",
                metadata.target_directory.join("release/deps").display()
            )),
            OsString::from("-Copt-level=z"),
            OsString::from("-Ccodegen-units=1"),
            OsString::from("-Clto=fat"),
            OsString::from("-Cpanic=abort"),
            OsString::from("-Ctarget-cpu=cortex-m7"),
            OsString::from("-Crelocation-model=pic"),
            OsString::from("-Cforce-unwind-tables=no"),
            OsString::from("-Cllvm-args=-mergefunc-use-aliases=0"),
            OsString::from(format!(
                "--remap-path-prefix={}=<WORKSPACE>",
                metadata.workspace_root.display()
            )),
            OsString::from("-o"),
            archive.as_os_str().to_owned(),
        ];
        run_command(&rustc, &rustc_args, &[], Some(&mut transcript))?;
    }
    if !archive.is_file() {
        return Err(CliError::failed(format!(
            "Rust produced no static archive at {}",
            archive.display()
        )));
    }

    let sdk = metadata
        .workspace_root
        .join("external/logue-sdk/platform/nts-3_kaoss");
    let linker_script = sdk.join("ld/unit.ld");
    if !linker_script.is_file() {
        return Err(CliError::failed(format!(
            "SDK linker script is missing: {}",
            linker_script.display()
        )));
    }
    let mut link_args = vec![archive.as_os_str().to_owned()];
    link_args.extend(strings(&[
        "-mcpu=cortex-m7",
        "-mthumb",
        "-mno-thumb-interwork",
        "-mlittle-endian",
        "-mfloat-abi=hard",
        "-mfpu=fpv4-sp-d16",
        "-nostartfiles",
        "-shared",
        "--entry=0",
        "-specs=nano.specs",
        "-specs=nosys.specs",
        "-Wl,-z,max-page-size=128",
        "-Wl,--gc-sections",
        "-Wl,--no-warn-mismatch",
    ]));
    link_args.push(OsString::from(format!(
        "-Wl,--library-path={}",
        sdk.join("ld").display()
    )));
    link_args.push(OsString::from(format!(
        "-Wl,--script={}",
        linker_script.display()
    )));
    link_args.push(OsString::from(format!("-Wl,-Map={},--cref", map.display())));
    for symbol in EXPORTS {
        link_args.push(OsString::from(format!("-Wl,--undefined={symbol}")));
    }
    link_args.extend(strings(&["-lc", "-lm", "-lgcc", "-o"]));
    link_args.push(elf.as_os_str().to_owned());
    let cc = tool("NTS3_CC", "arm-none-eabi-gcc");
    run_command(&cc, &link_args, &[], Some(&mut transcript))?;

    fs::copy(&elf, &unit).map_err(|error| {
        CliError::failed(format!(
            "cannot copy {} to {}: {error}",
            elf.display(),
            unit.display()
        ))
    })?;
    let strip = tool("NTS3_STRIP", "arm-none-eabi-strip");
    run_command(
        &strip,
        &[unit.as_os_str().to_owned()],
        &[],
        Some(&mut transcript),
    )?;
    report(&output_dir, stem, &unit, &mut transcript)?;
    post_link_sanity(&output_dir, stem)?;
    let probe = run_native_probe(&metadata, &selected, &output_dir, &mut transcript)?;
    let inspection = inspect_to_reports(&unit, Some(probe))?;
    if !inspection.errors.is_empty() {
        return Err(CliError::failed(format!(
            "artifact inspection failed: {}",
            inspection.errors.join("; ")
        )));
    }

    let size = fs::metadata(&unit)
        .map_err(|error| CliError::failed(format!("cannot stat {}: {error}", unit.display())))?
        .len();
    if size == 0 {
        return Err(CliError::failed("the stripped unit is empty"));
    }
    println!("unit: {}", unit.display());
    println!("map: {}", map.display());
    println!(
        "reports: {}.{{memory.json,memory.txt,elf,readelf.txt,nm.txt,size.txt}}",
        output_dir.join(stem).display()
    );
    println!("command transcript: {}", transcript_path.display());
    Ok(())
}

fn run_native_probe(
    metadata: &Metadata,
    selected: &SelectedPackage,
    output_dir: &Path,
    transcript: &mut Transcript,
) -> Result<ProbeReport> {
    let dependency_dir = metadata.target_directory.join("release/deps");
    remove_matching_rlibs(&dependency_dir, &selected.lib_name)?;
    let cargo = tool("NTS3_CARGO", "cargo");
    let cargo_args = strings(&[
        "rustc",
        "--locked",
        "-p",
        &selected.name,
        "--lib",
        "--release",
        "--",
        "--crate-type=rlib",
    ]);
    run_command(
        &cargo,
        &cargo_args,
        &[("CARGO_PROFILE_RELEASE_LTO", OsString::from("false"))],
        Some(transcript),
    )?;
    let plugin_rlib = find_rlib(&dependency_dir, &selected.lib_name)?;

    let probe_dir = output_dir.join(".probe").join(&selected.lib_name);
    fs::create_dir_all(&probe_dir).map_err(|error| {
        CliError::failed(format!("cannot create {}: {error}", probe_dir.display()))
    })?;
    let source = probe_dir.join("probe.rs");
    let executable = probe_dir.join(if cfg!(windows) { "probe.exe" } else { "probe" });
    fs::write(
        &source,
        r#"extern crate plugin;
plugin::__nts3_one_exported_plugin_per_artifact!();
fn main() {
    match plugin::nts3_host_probe() {
        Ok(value) => println!(
            "NTS3_PROBE_V1 {} {} {} {} {} {} {} {} {} {}",
            value.plugin_bytes,
            value.parameters_bytes,
            value.runtime_bytes,
            value.budget_bytes,
            value.requested_bytes,
            value.alignment_padding_bytes,
            value.high_water_bytes,
            value.initialization_allocations,
            value.post_init_allocations,
            value.render_iterations,
        ),
        Err(error) => { eprintln!("native probe failed: {error}"); std::process::exit(1); }
    }
}
"#,
    )
    .map_err(|error| CliError::failed(format!("cannot write native probe: {error}")))?;
    let rustc = tool("NTS3_RUSTC", "rustc");
    let rustc_args = vec![
        OsString::from("--edition=2024"),
        source.as_os_str().to_owned(),
        OsString::from("--extern"),
        OsString::from(format!("plugin={}", plugin_rlib.display())),
        OsString::from("-L"),
        OsString::from(format!("dependency={}", dependency_dir.display())),
        OsString::from("-Cpanic=abort"),
        OsString::from("-o"),
        executable.as_os_str().to_owned(),
    ];
    run_command(&rustc, &rustc_args, &[], Some(transcript))?;
    transcript.record(executable.as_os_str(), &[], &[])?;
    let output = Command::new(&executable).output().map_err(|error| {
        CliError::failed(format!(
            "failed to launch native initialization probe: {error}"
        ))
    })?;
    if !output.status.success() {
        return Err(CliError {
            code: status_code(output.status),
            message: format!(
                "native initialization probe failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    parse_probe_output(&String::from_utf8_lossy(&output.stdout))
}

fn parse_probe_output(output: &str) -> Result<ProbeReport> {
    let line = output
        .lines()
        .find(|line| line.starts_with("NTS3_PROBE_V1 "))
        .ok_or_else(|| CliError::failed("native probe produced no NTS3_PROBE_V1 record"))?;
    let values = line
        .split_whitespace()
        .skip(1)
        .map(|value| {
            value.parse::<u64>().map_err(|_| {
                CliError::failed(format!("native probe produced invalid integer `{value}`"))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if values.len() != 10 {
        return Err(CliError::failed(format!(
            "native probe produced {} fields, expected 10",
            values.len()
        )));
    }
    Ok(ProbeReport {
        plugin_bytes: values[0],
        parameters_bytes: values[1],
        runtime_bytes: values[2],
        budget_bytes: values[3],
        requested_bytes: values[4],
        alignment_padding_bytes: values[5],
        high_water_bytes: values[6],
        initialization_allocations: values[7],
        post_init_allocations: values[8],
        render_iterations: values[9],
    })
}

fn inspect_to_reports(
    path: &Path,
    probe: Option<ProbeReport>,
) -> Result<inspector::InspectionReport> {
    let report = inspector::inspect(path, probe, provenance())
        .map_err(|error| CliError::failed(format!("inspection failed: {error}")))?;
    let (json_path, text_path) = inspector::report_paths(path);
    inspector::write_reports(&report, &json_path, &text_path).map_err(CliError::failed)?;
    println!("memory report: {}", text_path.display());
    println!("memory JSON: {}", json_path.display());
    Ok(report)
}

fn inspect_command(args: &[String]) -> Result<()> {
    if args.len() != 1 {
        return Err(CliError::usage("usage: nts3 inspect <artifact>"));
    }
    let path = PathBuf::from(&args[0]);
    let report = inspect_to_reports(&path, cached_probe(&path))?;
    print!("{}", inspector::human_report(&report));
    if report.errors.is_empty() {
        Ok(())
    } else {
        Err(CliError::failed(format!(
            "artifact rejected: {}",
            report.errors.join("; ")
        )))
    }
}

#[derive(Deserialize)]
struct CachedInspection {
    artifact: CachedArtifact,
    native_probe: Option<ProbeReport>,
}

#[derive(Deserialize)]
struct CachedArtifact {
    sha256: String,
}

fn cached_probe(path: &Path) -> Option<ProbeReport> {
    use sha2::{Digest, Sha256};
    let (json_path, _) = inspector::report_paths(path);
    let cached: CachedInspection = serde_json::from_slice(&fs::read(json_path).ok()?).ok()?;
    let actual = format!("{:x}", Sha256::digest(fs::read(path).ok()?));
    (cached.artifact.sha256 == actual)
        .then_some(cached.native_probe)
        .flatten()
}

fn provenance() -> Provenance {
    Provenance {
        rust: command_line("rustc", &["--version"]),
        cargo: command_line("cargo", &["--version"]),
        gnu_arm_gcc: command_line("arm-none-eabi-gcc", &["--version"]),
        gnu_binutils: command_line("arm-none-eabi-readelf", &["--version"]),
        sdk_commit: source_revision(Path::new("external/logue-sdk")),
        framework_commit: source_revision(Path::new(".")),
        framework_version: env!("CARGO_PKG_VERSION").to_owned(),
        fundsp_version: locked_package_version("fundsp").unwrap_or_else(|| "not present".to_owned()),
        container_base_image: "xiashj/logue-sdk@sha256:e4d85a16c38dc4d34b0e93cabb6378df21729688dbcd88808c84e8d41aa0c9ba".to_owned(),
        cargo_lock_sha256: file_sha256(Path::new("Cargo.lock")),
        sdk_linker_script_sha256: file_sha256(Path::new(
            "external/logue-sdk/platform/nts-3_kaoss/ld/unit.ld",
        )),
    }
}

fn source_revision(root: &Path) -> String {
    let git_marker = root.join(".git");
    let git_dir = if git_marker.is_dir() {
        git_marker
    } else if let Ok(marker) = fs::read_to_string(&git_marker) {
        let Some(relative) = marker.trim().strip_prefix("gitdir: ") else {
            return "unknown".to_owned();
        };
        root.join(relative)
    } else {
        return "unknown".to_owned();
    };
    let Ok(head) = fs::read_to_string(git_dir.join("HEAD")) else {
        return "unknown".to_owned();
    };
    let head = head.trim();
    let Some(reference) = head.strip_prefix("ref: ") else {
        return head.to_owned();
    };
    if let Ok(value) = fs::read_to_string(git_dir.join(reference)) {
        return value.trim().to_owned();
    }
    fs::read_to_string(git_dir.join("packed-refs"))
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let (hash, name) = line.split_once(' ')?;
                (name == reference).then(|| hash.to_owned())
            })
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn file_sha256(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    fs::read(path)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn command_line(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_owned()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn locked_package_version(name: &str) -> Option<String> {
    let contents = fs::read_to_string("Cargo.lock").ok()?;
    let marker = format!("name = \"{name}\"\n");
    let rest = contents.split(&marker).nth(1)?;
    let version = rest.lines().find(|line| line.starts_with("version = \""))?;
    Some(
        version
            .trim_start_matches("version = \"")
            .trim_end_matches('"')
            .to_owned(),
    )
}

fn post_link_sanity(output_dir: &Path, stem: &str) -> Result<()> {
    let readelf = fs::read_to_string(output_dir.join(format!("{stem}.readelf.txt")))
        .map_err(|error| CliError::failed(format!("cannot read ELF report: {error}")))?;
    for invariant in [
        "Class:                             ELF32",
        "Data:                              2's complement, little endian",
        "OS/ABI:                            UNIX - System V",
        "Type:                              DYN (Shared object file)",
        "Machine:                           ARM",
        "Version5 EABI, hard-float ABI",
    ] {
        if !readelf.contains(invariant) {
            return Err(CliError::failed(format!(
                "post-link sanity failed: missing `{invariant}`"
            )));
        }
    }
    let nm = fs::read_to_string(output_dir.join(format!("{stem}.nm.txt")))
        .map_err(|error| CliError::failed(format!("cannot read symbol report: {error}")))?;
    let mut callbacks = std::collections::BTreeMap::new();
    for symbol in EXPORTS {
        if !nm
            .lines()
            .any(|line| line.split_whitespace().last() == Some(symbol))
        {
            return Err(CliError::failed(format!(
                "post-link sanity failed: missing export `{symbol}`"
            )));
        }
        if symbol.starts_with("unit_") && *symbol != "unit_header" {
            let address = readelf
                .lines()
                .find(|line| line.split_whitespace().last() == Some(symbol))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|value| u64::from_str_radix(value, 16).ok())
                .ok_or_else(|| {
                    CliError::failed(format!(
                        "post-link sanity failed: invalid dynamic address for `{symbol}`"
                    ))
                })?;
            if address & 1 == 0 {
                return Err(CliError::failed(format!(
                    "post-link sanity failed: `{symbol}` lacks the Thumb bit"
                )));
            }
            if let Some(previous) = callbacks.insert(address, symbol) {
                return Err(CliError::failed(format!(
                    "post-link sanity failed: callbacks `{previous}` and `{symbol}` alias"
                )));
            }
        }
    }
    if nm.lines().any(|line| {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        fields.len() >= 2 && fields[fields.len() - 2] == "U"
    }) {
        return Err(CliError::failed(
            "post-link sanity failed: undefined dynamic symbol",
        ));
    }
    Ok(())
}

fn rlib_matches(path: &Path, stem: &str) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.starts_with(&format!("lib{stem}-")) && name.ends_with(".rlib"))
}

fn remove_matching_rlibs(directory: &Path, stem: &str) -> Result<()> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry
            .map_err(|error| CliError::failed(format!("cannot read target directory: {error}")))?
            .path();
        if rlib_matches(&path, stem) {
            fs::remove_file(&path).map_err(|error| {
                CliError::failed(format!("cannot remove stale {}: {error}", path.display()))
            })?;
        }
    }
    Ok(())
}

fn find_rlib(directory: &Path, stem: &str) -> Result<PathBuf> {
    let entries = fs::read_dir(directory).map_err(|error| {
        CliError::failed(format!(
            "cannot read Cargo output {}: {error}",
            directory.display()
        ))
    })?;
    let mut matches = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| rlib_matches(path, stem))
        .collect::<Vec<_>>();
    matches.sort();
    if matches.len() != 1 {
        return Err(CliError::failed(format!(
            "expected one Cargo rlib for `{stem}`, found {} in {}",
            matches.len(),
            directory.display()
        )));
    }
    Ok(matches.remove(0))
}

fn report(output_dir: &Path, stem: &str, unit: &Path, transcript: &mut Transcript) -> Result<()> {
    let reports = [
        (
            "NTS3_READELF",
            "arm-none-eabi-readelf",
            vec!["-hAWSlrds"],
            "readelf.txt",
        ),
        ("NTS3_NM", "arm-none-eabi-nm", vec!["-D"], "nm.txt"),
        ("NTS3_SIZE", "arm-none-eabi-size", vec!["-A"], "size.txt"),
    ];
    for (variable, default, fixed_args, suffix) in reports {
        let program = tool(variable, default);
        let path = output_dir.join(format!("{stem}.{suffix}"));
        let mut args = fixed_args
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        args.push(unit.as_os_str().to_owned());
        transcript.record(&program, &args, &[])?;
        let file = fs::File::create(&path).map_err(|error| {
            CliError::failed(format!("cannot create report {}: {error}", path.display()))
        })?;
        let status = Command::new(&program)
            .args(&args)
            .stdout(Stdio::from(file))
            .status()
            .map_err(|error| {
                CliError::failed(format!(
                    "failed to launch {}: {error}",
                    program.to_string_lossy()
                ))
            })?;
        if !status.success() {
            return Err(CliError {
                code: status_code(status),
                message: format!("{} failed", program.to_string_lossy()),
            });
        }
    }
    Ok(())
}

fn doctor(args: &[String]) -> Result<()> {
    if !args.is_empty() {
        return Err(CliError::usage("doctor takes no arguments"));
    }
    let cwd = env::current_dir().map_err(|error| CliError::failed(error.to_string()))?;
    let workspace = cwd.join("Cargo.toml");
    let sdk = cwd.join("external/logue-sdk/platform/nts-3_kaoss/ld/unit.ld");
    if !workspace.is_file() {
        return Err(CliError::failed(format!(
            "in-container check failed (workspace mount): {} is missing",
            workspace.display()
        )));
    }
    if !sdk.is_file() {
        return Err(CliError::failed(format!(
            "in-container check failed (SDK mount): {} is missing",
            sdk.display()
        )));
    }
    check_writable(&cwd.join("target/nts3"), "output mount")?;
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/cargo"));
    check_writable(&cargo_home, "Cargo cache")?;

    let rustc = tool("NTS3_RUSTC", "rustc");
    let rust = capture(&rustc, &["--version"], "Rust version")?;
    if rust.split_whitespace().nth(1) != Some(RUST_VERSION) {
        return Err(CliError::failed(format!(
            "in-container check failed (Rust version): expected {RUST_VERSION}, got `{rust}`"
        )));
    }
    let cargo = tool("NTS3_CARGO", "cargo");
    let cargo_version = capture(&cargo, &["--version"], "Cargo version")?;
    if !cargo_version.starts_with(&format!("cargo {RUST_VERSION} ")) {
        return Err(CliError::failed(format!(
            "in-container check failed (Cargo version): expected {RUST_VERSION}, got `{cargo_version}`"
        )));
    }
    let target_libdir = capture(
        &rustc,
        &["--print", "target-libdir", "--target", TARGET],
        "Rust target",
    )?;
    if !Path::new(&target_libdir).is_dir() {
        return Err(CliError::failed(format!(
            "in-container check failed (Rust target): {target_libdir} is missing"
        )));
    }
    let gcc = capture(
        &tool("NTS3_CC", "arm-none-eabi-gcc"),
        &["--version"],
        "GNU compiler",
    )?;
    if !gcc.contains(GCC_VERSION) {
        return Err(CliError::failed(format!(
            "in-container check failed (GNU compiler): expected {GCC_VERSION}"
        )));
    }
    for (variable, executable) in [
        ("NTS3_LD", "arm-none-eabi-ld"),
        ("NTS3_STRIP", "arm-none-eabi-strip"),
        ("NTS3_READELF", "arm-none-eabi-readelf"),
        ("NTS3_NM", "arm-none-eabi-nm"),
        ("NTS3_SIZE", "arm-none-eabi-size"),
    ] {
        let version = capture(&tool(variable, executable), &["--version"], executable)?;
        if !version.contains(BINUTILS_VERSION) {
            return Err(CliError::failed(format!(
                "in-container check failed ({executable}): expected binutils {BINUTILS_VERSION}"
            )));
        }
    }
    println!("in-container checks passed");
    println!("  Rust/Cargo: {RUST_VERSION}");
    println!("  target: {TARGET}");
    println!("  GNU ARM GCC: {GCC_VERSION}");
    println!("  GNU binutils: {BINUTILS_VERSION}");
    Ok(())
}

fn check_writable(directory: &Path, label: &str) -> Result<()> {
    fs::create_dir_all(directory).map_err(|error| {
        CliError::failed(format!("in-container check failed ({label}): {error}"))
    })?;
    let probe = directory.join(".nts3-doctor-write");
    fs::write(&probe, b"ok").map_err(|error| {
        CliError::failed(format!("in-container check failed ({label}): {error}"))
    })?;
    fs::remove_file(probe)
        .map_err(|error| CliError::failed(format!("in-container check failed ({label}): {error}")))
}

fn new_plugin(args: &[String]) -> Result<()> {
    if args.len() != 1 {
        return Err(CliError::usage("usage: nts3 new <name>"));
    }
    let name = &args[0];
    if !valid_package_name(name) {
        return Err(CliError::usage(
            "plugin name must use lowercase ASCII letters, digits, and single hyphens",
        ));
    }
    let display_name = name
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ");
    if display_name.len() > 19 {
        return Err(CliError::usage(
            "generated unit name exceeds the SDK limit of 19 characters",
        ));
    }
    let root = env::current_dir().map_err(|error| CliError::failed(error.to_string()))?;
    let directory = root.join("plugins").join(name);
    if directory.exists() {
        return Err(CliError::usage(format!(
            "destination already exists: {}",
            directory.display()
        )));
    }
    fs::create_dir_all(directory.join("src")).map_err(|error| {
        CliError::failed(format!("cannot create {}: {error}", directory.display()))
    })?;
    let manifest = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\nnts3 = {{ path = \"../../crates/nts3\" }}\n"
    );
    let unit_id = fnv1a(name.as_bytes());
    let source = template(&display_name, unit_id);
    fs::write(directory.join("Cargo.toml"), manifest)
        .and_then(|_| fs::write(directory.join("src/lib.rs"), source))
        .map_err(|error| CliError::failed(format!("cannot write generated plugin: {error}")))?;
    add_workspace_member(&root, name)?;
    println!("created {}", directory.display());
    println!("check with: ./nts3.sh check -p {name}");
    Ok(())
}

fn add_workspace_member(root: &Path, name: &str) -> Result<()> {
    let path = root.join("Cargo.toml");
    let contents = fs::read_to_string(&path)
        .map_err(|error| CliError::failed(format!("cannot read {}: {error}", path.display())))?;
    let members = contents
        .find("members = [")
        .ok_or_else(|| CliError::failed("root Cargo.toml has no workspace members list"))?;
    let end = contents[members..]
        .find("\n]")
        .map(|offset| members + offset)
        .ok_or_else(|| CliError::failed("workspace members list is not terminated"))?;
    let entry = format!("\n    \"plugins/{name}\",");
    let mut updated = contents;
    updated.insert_str(end, &entry);
    fs::write(&path, updated)
        .map_err(|error| CliError::failed(format!("cannot update {}: {error}", path.display())))
}

fn valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
    })
}

fn template(name: &str, unit_id: u32) -> String {
    format!(
        r##"#![no_std]

use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct Parameters {{
    #[parameter(name = "GAIN", min = 0, max = 100, default = 100, parameter_type = "percent")]
    gain: Parameter,
}}

#[derive(Default)]
struct Plugin;

#[nts3::plugin(
    name = "{name}",
    developer_id = 0x5255_5354,
    unit_id = 0x{unit_id:08X},
    sdram_bytes = 1
)]
impl Nts3Plugin for Plugin {{
    type Parameters = Parameters;

    fn process(&mut self, parameters: &mut Parameters, buffer: &mut StereoBuffer<'_>) {{
        let gain = parameters.gain.normalized();
        for mut frame in buffer.frames_mut() {{
            let [left, right] = frame.input();
            frame.write([left * gain, right * gain]);
        }}
    }}
}}
"##
    )
}

fn strings(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn usage() {
    eprintln!(
        "Usage:\n  ./nts3.sh doctor\n  ./nts3.sh check -p <package>\n  ./nts3.sh build -p <package> [--release] [--verbose]\n  ./nts3.sh inspect <artifact>\n  ./nts3.sh new <name>"
    );
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args
        .next()
        .ok_or_else(|| CliError::usage("missing command"))?;
    let rest = args.collect::<Vec<_>>();
    match command.as_str() {
        "doctor" => doctor(&rest),
        "check" => check(&rest),
        "build" => build(&rest),
        "new" => new_plugin(&rest),
        "inspect" => inspect_command(&rest),
        "-h" | "--help" | "help" => {
            usage();
            Ok(())
        }
        other => Err(CliError::usage(format!("unknown command `{other}`"))),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("nts3: {}", error.message);
        if error.code == 2 {
            usage();
        }
        std::process::exit(error.code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_template_is_stable() {
        assert_eq!(
            template("Test Plug", 0x1234_5678),
            include_str!("../tests/fixtures/new-plugin.rs")
        );
    }

    #[test]
    fn package_names_are_conservative() {
        assert!(valid_package_name("smooth-echo2"));
        for invalid in ["", "Smooth", "a_b", "-a", "a-", "a--b"] {
            assert!(!valid_package_name(invalid), "{invalid}");
        }
    }

    #[test]
    fn native_probe_record_is_versioned_and_exact() {
        let probe = parse_probe_output("noise\nNTS3_PROBE_V1 1 2 3 4 5 6 7 8 0 256\n").unwrap();
        assert_eq!(probe.plugin_bytes, 1);
        assert_eq!(probe.parameters_bytes, 2);
        assert_eq!(probe.runtime_bytes, 3);
        assert_eq!(probe.high_water_bytes, 7);
        assert_eq!(probe.post_init_allocations, 0);
        assert_eq!(probe.render_iterations, 256);
        assert!(parse_probe_output("NTS3_PROBE_V2 1").is_err());
    }
}
