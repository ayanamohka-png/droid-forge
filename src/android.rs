use std::process::Command;

/// Универсальная функция для выполнения команд adb/fastboot
pub fn run_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    match Command::new(cmd).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&stderr);
            }

            if output.status.success() {
                Ok(result)
            } else {
                Err(result)
            }
        }
        Err(e) => Err(format!("Ошибка запуска {}: {}", cmd, e)),
    }
}

/// Проверяет подключение устройства через adb
pub fn check_adb() -> (bool, String) {
    match run_command("adb", &["devices"]) {
        Ok(output) => {
            let found = output.contains("device")
                && !output.trim().ends_with("List of devices attached");
            (found, output)
        }
        Err(e) => (false, e),
    }
}

/// Проверяет подключение устройства через fastboot
pub fn check_fastboot() -> (bool, String) {
    match run_command("fastboot", &["devices"]) {
        Ok(output) => {
            let found = !output.trim().is_empty();
            (found, output)
        }
        Err(e) => (false, e),
    }
}

/// Установка APK через adb
pub fn install_apk(file_path: &str) -> Result<String, String> {
    run_command("adb", &["install", "-r", file_path])
}

/// Sideload через adb
pub fn sideload(file_path: &str) -> Result<String, String> {
    run_command("adb", &["sideload", file_path])
}

/// Передать файл на устройство
pub fn push_file(file_path: &str, dest: &str) -> Result<String, String> {
    run_command("adb", &["push", file_path, dest])
}

/// Забрать файл с устройства
pub fn pull_file(remote_path: &str, local_path: &str) -> Result<String, String> {
    run_command("adb", &["pull", remote_path, local_path])
}

/// Перезагрузка через adb
pub fn adb_reboot(mode: &str) -> Result<String, String> {
    if mode.is_empty() {
        run_command("adb", &["reboot"])
    } else {
        run_command("adb", &["reboot", mode])
    }
}

/// Скриншот через adb
pub fn screenshot(output_path: &str) -> Result<String, String> {
    run_command("adb", &["exec-out", "screencap", "-p", ">", output_path])
}

/// Получить логи через logcat
pub fn logcat(output_path: &str) -> Result<String, String> {
    match run_command("adb", &["logcat", "-d"]) {
        Ok(logs) => {
            std::fs::write(output_path, &logs)
                .map_err(|e| format!("Ошибка записи файла: {}", e))?;
            Ok(format!("Логи сохранены в {}\n{} байт", output_path, logs.len()))
        }
        Err(e) => Err(e),
    }
}

/// Получить информацию о телефоне
pub fn get_device_info() -> Result<String, String> {
    let model = run_command("adb", &["shell", "getprop", "ro.product.model"])?;
    let version = run_command("adb", &["shell", "getprop", "ro.build.version.release"])?;

    Ok(format!(
        "Модель: {}\nВерсия Android: {}",
        model.trim(),
        version.trim()
    ))
}

/// Прошивка раздела через fastboot
pub fn flash_partition(partition: &str, image_path: &str) -> Result<String, String> {
    run_command("fastboot", &["flash", partition, image_path])
}

/// Загрузка с образа без прошивки
pub fn boot_image(image_path: &str) -> Result<String, String> {
    run_command("fastboot", &["boot", image_path])
}

/// Разблокировка загрузчика
pub fn unlock_bootloader() -> Result<String, String> {
    run_command("fastboot", &["flashing", "unlock"])
}

/// Блокировка загрузчика
pub fn lock_bootloader() -> Result<String, String> {
    run_command("fastboot", &["flashing", "lock"])
}

/// Стереть раздел
pub fn erase_partition(partition: &str) -> Result<String, String> {
    run_command("fastboot", &["erase", partition])
}

/// Перезагрузка через fastboot
pub fn fastboot_reboot(mode: &str) -> Result<String, String> {
    if mode.is_empty() {
        run_command("fastboot", &["reboot"])
    } else {
        run_command("fastboot", &["reboot", mode])
    }
}

/// Получить переменные fastboot
pub fn get_fastboot_var(var: &str) -> Result<String, String> {
    run_command("fastboot", &["getvar", var])
}
