use crate::devices::DeviceDatabase;
use std::path::PathBuf;
use std::process::{Command, Output};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    Disconnected,
    Adb,
    AdbUnauthorized,
    AdbRecovery,
    AdbSideload,
    Fastboot,
    Fastbootd,
}

impl ConnectionMode {
    pub fn as_id(self) -> i32 {
        match self {
            Self::Disconnected => 0,
            Self::Adb => 1,
            Self::AdbUnauthorized => 2,
            Self::AdbRecovery => 3,
            Self::AdbSideload => 4,
            Self::Fastboot => 5,
            Self::Fastbootd => 6,
        }
    }

    pub fn is_adb_ready(self) -> bool {
        matches!(self, Self::Adb | Self::AdbRecovery | Self::AdbSideload)
    }

    pub fn is_fastboot(self) -> bool {
        matches!(self, Self::Fastboot | Self::Fastbootd)
    }
}

#[derive(Debug, Clone)]
pub struct DeviceSnapshot {
    pub mode: ConnectionMode,
    pub serial: String,
    pub codename: String,
    pub display_name: String,
    pub soc: String,
    pub android_version: String,
    pub api_level: String,
    pub battery_percent: i32,
    pub bootloader_version: String,
    pub active_slot: String,
    /// -1 = unknown, 0 = locked, 1 = unlocked
    pub bootloader_state: i32,
    pub raw_summary: String,
}

impl Default for DeviceSnapshot {
    fn default() -> Self {
        Self {
            mode: ConnectionMode::Disconnected,
            serial: String::new(),
            codename: String::new(),
            display_name: "Нет устройства".into(),
            soc: String::new(),
            android_version: String::new(),
            api_level: String::new(),
            battery_percent: -1,
            bootloader_version: String::new(),
            active_slot: String::new(),
            bootloader_state: -1,
            raw_summary: "Устройства ADB и Fastboot не обнаружены".into(),
        }
    }
}

fn resolve_tool(tool: &str) -> PathBuf {
    #[cfg(windows)]
    let exe_name = format!("{tool}.exe");
    #[cfg(not(windows))]
    let exe_name = tool.to_string();

    // 1. Рядом с исполняемым файлом DroidForge
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let local = dir.join(&exe_name);
            if local.is_file() {
                return local;
            }
        }
    }

    // 2. Стандартный путь Android SDK platform-tools (Windows)
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let sdk_tool = PathBuf::from(local_app_data)
                .join("Android")
                .join("Sdk")
                .join("platform-tools")
                .join(&exe_name);
            if sdk_tool.is_file() {
                return sdk_tool;
            }
        }
    }

    PathBuf::from(tool)
}

fn execute_raw(tool: &str, args: &[&str]) -> Result<Output, String> {
    let bin = resolve_tool(tool);
    let mut command = Command::new(&bin);
    command.args(args);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command
        .output()
        .map_err(|e| format!("Не удалось запустить `{tool}` ({}): {e}", bin.display()))
}

pub fn run_command(tool: &str, args: &[&str]) -> Result<String, String> {
    let output = execute_raw(tool, args)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut combined = String::new();
    if !stdout.trim().is_empty() {
        combined.push_str(stdout.trim_end());
    }
    if !stderr.trim().is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(stderr.trim_end());
    }

    if output.status.success() {
        Ok(if combined.is_empty() {
            "Готово".into()
        } else {
            combined
        })
    } else {
        Err(if combined.is_empty() {
            format!("Команда `{tool}` завершилась с кодом {:?}", output.status.code())
        } else {
            combined
        })
    }
}

pub fn run_command_owned(tool: &str, args: &[String]) -> Result<String, String> {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_command(tool, &refs)
}

/// Парсинг вывода `fastboot getvar <name>` -> значение переменной
fn parse_fastboot_var(raw: &str, var_name: &str) -> Option<String> {
    let prefix = format!("{var_name}:");
    for line in raw.lines() {
        let trimmed = line.trim();
        let cleaned = trimmed.strip_prefix("(bootloader)").unwrap_or(trimmed).trim();
        if let Some(rest) = cleaned.strip_prefix(&prefix) {
            let val = rest.trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn fastboot_var_value(var: &str) -> Option<String> {
    let out = run_command("fastboot", &["getvar", var]).ok()?;
    parse_fastboot_var(&out, var)
}

fn adb_getprop(prop: &str) -> String {
    run_command("adb", &["shell", "getprop", prop])
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn adb_battery_level() -> i32 {
    if let Ok(out) = run_command("adb", &["shell", "dumpsys", "battery"]) {
        for line in out.lines() {
            let trimmed = line.trim();
            if let Some(val) = trimmed.strip_prefix("level:") {
                if let Ok(n) = val.trim().parse::<i32>() {
                    return n.clamp(0, 100);
                }
            }
        }
    }
    -1
}

/// Полный опрос подключённых устройств ADB и Fastboot + сопоставление с devices.json
pub fn probe_device(db: &DeviceDatabase) -> DeviceSnapshot {
    // 1. Сначала проверяем Fastboot (приоритет при прошивке)
    if let Ok(fb_out) = run_command("fastboot", &["devices"]) {
        for line in fb_out.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].contains("fastboot") {
                let serial = parts[0].to_string();
                let codename = fastboot_var_value("product").unwrap_or_default();
                let bootloader_version = fastboot_var_value("version-bootloader").unwrap_or_default();
                let active_slot = fastboot_var_value("current-slot")
                    .map(|s| s.trim_start_matches('_').to_uppercase())
                    .unwrap_or_default();
                let is_userspace = fastboot_var_value("is-userspace")
                    .map(|v| v.eq_ignore_ascii_case("yes"))
                    .unwrap_or(false);
                let unlocked = fastboot_var_value("unlocked")
                    .map(|v| {
                        if v.eq_ignore_ascii_case("yes") || v == "1" {
                            1
                        } else if v.eq_ignore_ascii_case("no") || v == "0" {
                            0
                        } else {
                            -1
                        }
                    })
                    .unwrap_or(-1);

                let known = db.lookup(&codename);
                let display_name = match known {
                    Some(k) => format!("{} {}", k.brand, k.model),
                    None if !codename.is_empty() => format!("Android ({codename})"),
                    None => "Fastboot-устройство".into(),
                };
                let soc = known.map(|k| k.soc.clone()).unwrap_or_default();
                let mode = if is_userspace {
                    ConnectionMode::Fastbootd
                } else {
                    ConnectionMode::Fastboot
                };

                let mode_str = if is_userspace { "Fastbootd" } else { "Fastboot" };
                return DeviceSnapshot {
                    mode,
                    serial: serial.clone(),
                    codename: codename.clone(),
                    display_name: display_name.clone(),
                    soc,
                    android_version: String::new(),
                    api_level: String::new(),
                    battery_percent: -1,
                    bootloader_version,
                    active_slot,
                    bootloader_state: unlocked,
                    raw_summary: format!("[{mode_str}] {display_name} (SN: {serial})"),
                };
            }
        }
    }

    // 2. Проверяем ADB
    if let Ok(adb_out) = run_command("adb", &["devices"]) {
        for line in adb_out.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let serial = parts[0].to_string();
            let state = parts[1];

            let mode = match state {
                "device" => ConnectionMode::Adb,
                "unauthorized" => ConnectionMode::AdbUnauthorized,
                "recovery" => ConnectionMode::AdbRecovery,
                "sideload" => ConnectionMode::AdbSideload,
                _ => continue,
            };

            if mode == ConnectionMode::Adb {
                let model = adb_getprop("ro.product.model");
                let brand = adb_getprop("ro.product.manufacturer");
                let codename = adb_getprop("ro.product.device");
                let android_version = adb_getprop("ro.build.version.release");
                let api_level = adb_getprop("ro.build.version.sdk");
                let active_slot = adb_getprop("ro.boot.slot_suffix")
                    .trim_start_matches('_')
                    .to_uppercase();
                let battery_percent = adb_battery_level();

                let known = db.lookup(&codename);
                let display_name = if let Some(k) = known {
                    format!("{} {}", k.brand, k.model)
                } else if !model.is_empty() {
                    if !brand.is_empty() && !model.to_lowercase().starts_with(&brand.to_lowercase()) {
                        format!("{brand} {model}")
                    } else {
                        model
                    }
                } else {
                    "Android-устройство".into()
                };
                let soc = known
                    .map(|k| k.soc.clone())
                    .unwrap_or_else(|| adb_getprop("ro.board.platform"));

                return DeviceSnapshot {
                    mode,
                    serial: serial.clone(),
                    codename,
                    display_name: display_name.clone(),
                    soc,
                    android_version: android_version.clone(),
                    api_level,
                    battery_percent,
                    bootloader_version: adb_getprop("ro.bootloader"),
                    active_slot,
                    bootloader_state: -1,
                    raw_summary: format!(
                        "[ADB] {display_name} • Android {android_version} (SN: {serial})"
                    ),
                };
            }

            let label = match mode {
                ConnectionMode::AdbUnauthorized => "Ожидает подтверждения USB",
                ConnectionMode::AdbRecovery => "Устройство в Recovery",
                ConnectionMode::AdbSideload => "Устройство в ADB Sideload",
                _ => "Android-устройство",
            };

            return DeviceSnapshot {
                mode,
                serial: serial.clone(),
                display_name: label.into(),
                raw_summary: format!("[ADB] {label} (SN: {serial})"),
                ..Default::default()
            };
        }
    }

    DeviceSnapshot::default()
}

/* ───────────────────────── ADB КОМАНДЫ ───────────────────────── */

pub fn install_apk(file_path: &str, downgrade: bool, grant_perms: bool) -> Result<String, String> {
    let mut args: Vec<String> = vec!["install".into(), "-r".into()];
    if downgrade {
        args.push("-d".into());
    }
    if grant_perms {
        args.push("-g".into());
    }
    args.push(file_path.into());
    run_command_owned("adb", &args)
}

pub fn install_multiple_apks(paths: &[String]) -> Result<String, String> {
    let mut args: Vec<String> = vec!["install-multiple".into(), "-r".into()];
    args.extend(paths.iter().cloned());
    run_command_owned("adb", &args)
}

pub fn list_third_party_packages() -> Result<String, String> {
    let out = run_command("adb", &["shell", "pm", "list", "packages", "-3"])?;
    let mut pkgs: Vec<String> = out
        .lines()
        .filter_map(|l| l.trim().strip_prefix("package:").map(str::to_string))
        .collect();
    pkgs.sort();
    if pkgs.is_empty() {
        Ok("Сторонние пакеты не найдены".into())
    } else {
        Ok(format!("Найдено пакетов: {}\n{}", pkgs.len(), pkgs.join("\n")))
    }
}

pub fn uninstall_package(package: &str) -> Result<String, String> {
    run_command("adb", &["uninstall", package.trim()])
}

pub fn clear_app_data(package: &str) -> Result<String, String> {
    run_command("adb", &["shell", "pm", "clear", package.trim()])
}

pub fn disable_package(package: &str) -> Result<String, String> {
    run_command("adb", &["shell", "pm", "disable-user", "--user", "0", package.trim()])
}

pub fn sideload(file_path: &str) -> Result<String, String> {
    run_command("adb", &["sideload", file_path])
}

pub fn push_file(file_path: &str, dest: &str) -> Result<String, String> {
    let target = if dest.trim().is_empty() {
        "/sdcard/Download/"
    } else {
        dest.trim()
    };
    run_command("adb", &["push", file_path, target])
}

pub fn pull_file(remote_path: &str, local_path: &str) -> Result<String, String> {
    run_command("adb", &["pull", remote_path.trim(), local_path])
}

pub fn adb_reboot(mode: &str) -> Result<String, String> {
    if mode.is_empty() {
        run_command("adb", &["reboot"])
    } else {
        run_command("adb", &["reboot", mode])
    }
}

pub fn adb_power_off() -> Result<String, String> {
    run_command("adb", &["shell", "reboot", "-p"])
}

/// Исправленный скриншот: читаем бинарный PNG из stdout `adb exec-out screencap -p`
pub fn screenshot(output_path: &str) -> Result<String, String> {
    let output = execute_raw("adb", &["exec-out", "screencap", "-p"])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Ошибка screencap: {}", err.trim()));
    }
    if output.stdout.is_empty() {
        return Err("Получен пустой буфер изображения".into());
    }
    std::fs::write(output_path, &output.stdout)
        .map_err(|e| format!("Ошибка записи PNG: {e}"))?;
    Ok(format!(
        "Скриншот сохранён: {output_path} ({} КБ)",
        output.stdout.len() / 1024
    ))
}

pub fn screenrecord_15s() -> Result<String, String> {
    let remote = "/sdcard/droidforge_record.mp4";
    run_command("adb", &["shell", "screenrecord", "--time-limit", "15", remote])?;
    Ok(format!("Видео (15 сек) сохранено на устройстве: {remote}"))
}

pub fn logcat_save(output_path: &str) -> Result<String, String> {
    let logs = run_command("adb", &["logcat", "-d"])?;
    std::fs::write(output_path, &logs).map_err(|e| format!("Ошибка записи файла: {e}"))?;
    Ok(format!("Logcat сохранён: {output_path} ({} байт)", logs.len()))
}

pub fn logcat_clear() -> Result<String, String> {
    run_command("adb", &["logcat", "-c"])
}

pub fn wireless_tcpip(port: &str) -> Result<String, String> {
    let p = if port.trim().is_empty() { "5555" } else { port.trim() };
    run_command("adb", &["tcpip", p])
}

pub fn wireless_connect(address: &str) -> Result<String, String> {
    let addr = address.trim();
    if addr.is_empty() {
        return Err("Введите IP:порт устройства (например 192.168.1.42:5555)".into());
    }
    run_command("adb", &["connect", addr])
}

pub fn wireless_disconnect() -> Result<String, String> {
    run_command("adb", &["disconnect"])
}

pub fn restart_adb_server() -> Result<String, String> {
    let _ = run_command("adb", &["kill-server"]);
    run_command("adb", &["start-server"])
}

pub fn get_device_info_detailed() -> Result<String, String> {
    let model = adb_getprop("ro.product.model");
    let brand = adb_getprop("ro.product.manufacturer");
    let device = adb_getprop("ro.product.device");
    let android = adb_getprop("ro.build.version.release");
    let sdk = adb_getprop("ro.build.version.sdk");
    let abi = adb_getprop("ro.product.cpu.abi");
    let build = adb_getprop("ro.build.display.id");
    let security = adb_getprop("ro.build.version.security_patch");
    let battery = adb_battery_level();

    Ok(format!(
        "Производитель: {brand}\nМодель: {model} ({device})\nAndroid: {android} (API {sdk})\nПатч безопасности: {security}\nСборка: {build}\nАрхитектура: {abi}\nБатарея: {}%",
        if battery >= 0 { battery.to_string() } else { "—".into() }
    ))
}

/* ───────────────────────── FASTBOOT КОМАНДЫ ───────────────────────── */

pub fn flash_partition(partition: &str, image_path: &str) -> Result<String, String> {
    let part = partition.trim();
    if part.is_empty() {
        return Err("Укажите имя раздела".into());
    }
    run_command("fastboot", &["flash", part, image_path])
}

pub fn flash_vbmeta_disable_verity(image_path: &str) -> Result<String, String> {
    run_command(
        "fastboot",
        &[
            "--disable-verity",
            "--disable-verification",
            "flash",
            "vbmeta",
            image_path,
        ],
    )
}

pub fn boot_image(image_path: &str) -> Result<String, String> {
    run_command("fastboot", &["boot", image_path])
}

pub fn fastboot_update_zip(zip_path: &str) -> Result<String, String> {
    run_command("fastboot", &["update", zip_path])
}

pub fn set_active_slot(slot: &str) -> Result<String, String> {
    let arg = format!("--set-active={}", slot.trim().to_lowercase());
    run_command("fastboot", &[&arg])
}

pub fn unlock_bootloader() -> Result<String, String> {
    run_command("fastboot", &["flashing", "unlock"])
}

pub fn unlock_bootloader_critical() -> Result<String, String> {
    run_command("fastboot", &["flashing", "unlock_critical"])
}

pub fn oem_unlock() -> Result<String, String> {
    run_command("fastboot", &["oem", "unlock"])
}

pub fn lock_bootloader() -> Result<String, String> {
    run_command("fastboot", &["flashing", "lock"])
}

pub fn erase_partition(partition: &str) -> Result<String, String> {
    let part = partition.trim();
    if part.is_empty() {
        return Err("Укажите имя раздела".into());
    }
    run_command("fastboot", &["erase", part])
}

pub fn wipe_userdata() -> Result<String, String> {
    run_command("fastboot", &["-w"])
}

pub fn fastboot_reboot(mode: &str) -> Result<String, String> {
    match mode {
        "" => run_command("fastboot", &["reboot"]),
        "bootloader" => run_command("fastboot", &["reboot-bootloader"]),
        other => run_command("fastboot", &["reboot", other]),
    }
}

pub fn fastboot_power_off() -> Result<String, String> {
    run_command("fastboot", &["oem", "poweroff"])
}

pub fn get_fastboot_var(var: &str) -> Result<String, String> {
    let v = if var.trim().is_empty() { "all" } else { var.trim() };
    run_command("fastboot", &["getvar", v])
}

/* ───────────────────────── ПРОИЗВОЛЬНАЯ КОМАНДА ───────────────────────── */

pub fn run_custom_command(tool: &str, raw_args: &str) -> Result<String, String> {
    let tokens = split_shell_args(raw_args);
    if tokens.is_empty() {
        return Err("Введите аргументы команды".into());
    }
    run_command_owned(tool, &tokens)
}

fn split_shell_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';

    for ch in input.chars() {
        if in_quotes {
            if ch == quote_char {
                in_quotes = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            in_quotes = true;
            quote_char = ch;
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}
