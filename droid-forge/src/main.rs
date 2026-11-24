#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod android;
mod devices;

use android::DeviceSnapshot;
use devices::DeviceDatabase;
use slint::{ComponentHandle, SharedString, Weak};
use std::sync::Arc;

slint::include_modules!();

fn append_log(ui: &AppWindow, prefix: &str, title: &str, body: &str) {
    let mut current = ui.get_log_text().to_string();
    if current.len() > 64_000 {
        let mut keep_from = current.len() - 32_000;
        while keep_from < current.len() && !current.is_char_boundary(keep_from) {
            keep_from += 1;
        }
        let cut = current[keep_from..]
            .find('\n')
            .map(|idx| keep_from + idx + 1)
            .unwrap_or(keep_from);
        current = current[cut..].to_string();
    }

    if !current.is_empty() && !current.ends_with('\n') {
        current.push('\n');
    }

    let trimmed_body = body.trim();
    if trimmed_body.contains('\n') {
        current.push_str(&format!("[{prefix}] {title}\n{trimmed_body}\n"));
    } else {
        current.push_str(&format!("[{prefix}] {title}: {trimmed_body}\n"));
    }
    ui.set_log_text(current.into());

    let first_line = trimmed_body.lines().next().unwrap_or("Готово");
    ui.set_last_log_line(format!("[{prefix}] {title} — {first_line}").into());
}

fn apply_snapshot(ui: &AppWindow, snap: &DeviceSnapshot) {
    ui.set_connection_mode(snap.mode.as_id());
    ui.set_adb_ready(snap.mode.is_adb_ready());
    ui.set_fastboot_ready(snap.mode.is_fastboot());
    ui.set_device_name(snap.display_name.clone().into());
    ui.set_codename(snap.codename.clone().into());
    ui.set_serial(snap.serial.clone().into());
    ui.set_soc(snap.soc.clone().into());
    ui.set_android_version(snap.android_version.clone().into());
    ui.set_api_level(snap.api_level.clone().into());
    ui.set_battery_percent(snap.battery_percent);
    ui.set_bootloader_version(snap.bootloader_version.clone().into());
    ui.set_active_slot(snap.active_slot.clone().into());
    ui.set_bootloader_state(snap.bootloader_state);
}

fn spawn_refresh(weak: Weak<AppWindow>, db: Arc<DeviceDatabase>, verbose_log: bool) {
    if let Some(ui) = weak.upgrade() {
        ui.set_is_busy(true);
        ui.set_busy_label("Поиск устройства…".into());
    }

    std::thread::spawn(move || {
        let snap = android::probe_device(&db);
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                apply_snapshot(&ui, &snap);
                ui.set_is_busy(false);
                ui.set_busy_label(SharedString::new());
                if verbose_log {
                    append_log(&ui, "INFO", "Статус устройства", &snap.raw_summary);
                } else {
                    ui.set_last_log_line(snap.raw_summary.into());
                }
            }
        });
    });
}

fn spawn_task<F>(
    weak: Weak<AppWindow>,
    busy_label: impl Into<String>,
    log_title: impl Into<String>,
    refresh_after: Option<Arc<DeviceDatabase>>,
    work: F,
) where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    let busy_str = busy_label.into();
    let title = log_title.into();

    if let Some(ui) = weak.upgrade() {
        ui.set_is_busy(true);
        ui.set_busy_label(busy_str.into());
    }

    std::thread::spawn(move || {
        let result = work();
        let next_snapshot = refresh_after.map(|db| android::probe_device(&db));

        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                match result {
                    Ok(out) => append_log(&ui, "OK", &title, &out),
                    Err(err) => append_log(&ui, "ERR", &title, &err),
                }
                if let Some(snap) = next_snapshot {
                    apply_snapshot(&ui, &snap);
                }
                ui.set_is_busy(false);
                ui.set_busy_label(SharedString::new());
            }
        });
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let db = Arc::new(DeviceDatabase::load());

    // Первичное определение устройства при запуске
    spawn_refresh(ui.as_weak(), Arc::clone(&db), false);

    // ─── Обновление статуса ───
    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_refresh_device(move || {
            spawn_refresh(weak.clone(), Arc::clone(&db), true);
        });
    }

    // ─── ADB: APK & Sideload ───
    {
        let weak = ui.as_weak();
        ui.on_pick_apk(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Android Package", &["apk"])
                .pick_file()
            {
                if let Some(ui) = weak.upgrade() {
                    ui.set_apk_path(path.display().to_string().into());
                }
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_install_apk(move || {
            let Some(ui) = weak.upgrade() else { return };
            let path = ui.get_apk_path().to_string();
            if path.trim().is_empty() {
                return;
            }
            let downgrade = ui.get_apk_downgrade();
            let grant = ui.get_apk_grant_perms();
            spawn_task(weak.clone(), "Установка APK…", "ADB Install", None, move || {
                android::install_apk(&path, downgrade, grant)
            });
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_pick_and_install_split_apk(move || {
            if let Some(paths) = rfd::FileDialog::new()
                .add_filter("Split APK", &["apk"])
                .pick_files()
            {
                if paths.is_empty() {
                    return;
                }
                let list: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
                spawn_task(
                    weak.clone(),
                    "Установка Split APK…",
                    "ADB Install-Multiple",
                    None,
                    move || android::install_multiple_apks(&list),
                );
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_pick_sideload_zip(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("ZIP Archive", &["zip"])
                .pick_file()
            {
                if let Some(ui) = weak.upgrade() {
                    ui.set_sideload_path(path.display().to_string().into());
                }
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_run_sideload(move || {
            let Some(ui) = weak.upgrade() else { return };
            let path = ui.get_sideload_path().to_string();
            if path.trim().is_empty() {
                return;
            }
            spawn_task(weak.clone(), "ADB Sideload…", "ADB Sideload", None, move || {
                android::sideload(&path)
            });
        });
    }

    // ─── ADB: Пакеты ───
    {
        let weak = ui.as_weak();
        ui.on_list_packages(move || {
            spawn_task(
                weak.clone(),
                "Получение списка…",
                "Сторонние пакеты (-3)",
                None,
                android::list_third_party_packages,
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_clear_package_data(move || {
            let Some(ui) = weak.upgrade() else { return };
            let pkg = ui.get_package_name().to_string();
            if pkg.trim().is_empty() {
                return;
            }
            spawn_task(
                weak.clone(),
                "Очистка данных…",
                format!("pm clear {pkg}"),
                None,
                move || android::clear_app_data(&pkg),
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_disable_package(move || {
            let Some(ui) = weak.upgrade() else { return };
            let pkg = ui.get_package_name().to_string();
            if pkg.trim().is_empty() {
                return;
            }
            spawn_task(
                weak.clone(),
                "Отключение пакета…",
                format!("pm disable-user {pkg}"),
                None,
                move || android::disable_package(&pkg),
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_uninstall_package(move || {
            let Some(ui) = weak.upgrade() else { return };
            let pkg = ui.get_package_name().to_string();
            if pkg.trim().is_empty() {
                return;
            }
            spawn_task(
                weak.clone(),
                "Удаление пакета…",
                format!("adb uninstall {pkg}"),
                None,
                move || android::uninstall_package(&pkg),
            );
        });
    }

    // ─── ADB: Файлы и экран ───
    {
        let weak = ui.as_weak();
        ui.on_pick_push_file(move || {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                if let Some(ui) = weak.upgrade() {
                    ui.set_push_local_path(path.display().to_string().into());
                }
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_run_push_file(move || {
            let Some(ui) = weak.upgrade() else { return };
            let local = ui.get_push_local_path().to_string();
            let remote = ui.get_push_remote_path().to_string();
            if local.trim().is_empty() {
                return;
            }
            spawn_task(weak.clone(), "Отправка файла…", "ADB Push", None, move || {
                android::push_file(&local, &remote)
            });
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_run_pull_file(move || {
            let Some(ui) = weak.upgrade() else { return };
            let remote = ui.get_pull_remote_path().to_string();
            if remote.trim().is_empty() {
                return;
            }
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                let local = dir.display().to_string();
                spawn_task(weak.clone(), "Загрузка файла…", "ADB Pull", None, move || {
                    android::pull_file(&remote, &local)
                });
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_take_screenshot(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG Image", &["png"])
                .set_file_name("screenshot.png")
                .save_file()
            {
                let out = path.display().to_string();
                spawn_task(weak.clone(), "Снимок экрана…", "Скриншот PNG", None, move || {
                    android::screenshot(&out)
                });
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_record_screen(move || {
            spawn_task(
                weak.clone(),
                "Запись экрана (15 с)…",
                "ADB Screenrecord",
                None,
                android::screenrecord_15s,
            );
        });
    }

    // ─── ADB: Wi-Fi и диагностика ───
    {
        let weak = ui.as_weak();
        ui.on_wifi_tcpip(move || {
            spawn_task(weak.clone(), "Включение TCP/IP…", "ADB TCP/IP 5555", None, || {
                android::wireless_tcpip("5555")
            });
        });
    }

    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_wifi_connect(move || {
            let Some(ui) = weak.upgrade() else { return };
            let addr = ui.get_wifi_address().to_string();
            spawn_task(
                weak.clone(),
                "Подключение Wi-Fi ADB…",
                format!("ADB Connect {addr}"),
                Some(Arc::clone(&db)),
                move || android::wireless_connect(&addr),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_wifi_disconnect(move || {
            spawn_task(
                weak.clone(),
                "Отключение Wi-Fi ADB…",
                "ADB Disconnect",
                Some(Arc::clone(&db)),
                android::wireless_disconnect,
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_adb_device_info(move || {
            spawn_task(
                weak.clone(),
                "Чтение свойств…",
                "Сведения об устройстве",
                None,
                android::get_device_info_detailed,
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_save_logcat(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Log File", &["txt", "log"])
                .set_file_name("logcat.txt")
                .save_file()
            {
                let out = path.display().to_string();
                spawn_task(weak.clone(), "Сохранение Logcat…", "ADB Logcat", None, move || {
                    android::logcat_save(&out)
                });
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_clear_logcat(move || {
            spawn_task(weak.clone(), "Очистка Logcat…", "Logcat -c", None, android::logcat_clear);
        });
    }

    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_restart_adb(move || {
            spawn_task(
                weak.clone(),
                "Перезапуск ADB…",
                "Перезапуск сервера ADB",
                Some(Arc::clone(&db)),
                android::restart_adb_server,
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_adb_reboot(move |mode: SharedString| {
            let m = mode.to_string();
            let title = if m.is_empty() {
                "ADB Reboot".to_string()
            } else {
                format!("ADB Reboot ({m})")
            };
            spawn_task(weak.clone(), "Перезагрузка…", title, None, move || {
                android::adb_reboot(&m)
            });
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_adb_power_off(move || {
            spawn_task(
                weak.clone(),
                "Выключение…",
                "ADB Power Off",
                None,
                android::adb_power_off,
            );
        });
    }

    // ─── FASTBOOT ───
    {
        let weak = ui.as_weak();
        ui.on_pick_fb_image(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Partition Image", &["img", "bin", "mbn", "elf"])
                .pick_file()
            {
                if let Some(ui) = weak.upgrade() {
                    ui.set_fb_image_path(path.display().to_string().into());
                }
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_flash_partition(move || {
            let Some(ui) = weak.upgrade() else { return };
            let part = ui.get_fb_partition().to_string();
            let img = ui.get_fb_image_path().to_string();
            if part.trim().is_empty() || img.trim().is_empty() {
                return;
            }
            spawn_task(
                weak.clone(),
                format!("Прошивка {part}…"),
                format!("Fastboot flash {part}"),
                None,
                move || android::flash_partition(&part, &img),
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_boot_image(move || {
            let Some(ui) = weak.upgrade() else { return };
            let img = ui.get_fb_image_path().to_string();
            if img.trim().is_empty() {
                return;
            }
            spawn_task(weak.clone(), "Загрузка образа…", "Fastboot boot", None, move || {
                android::boot_image(&img)
            });
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_flash_vbmeta_no_verity(move || {
            let Some(ui) = weak.upgrade() else { return };
            let img = ui.get_fb_image_path().to_string();
            if img.trim().is_empty() {
                return;
            }
            spawn_task(
                weak.clone(),
                "Прошивка vbmeta…",
                "Fastboot flash vbmeta (без verity)",
                None,
                move || android::flash_vbmeta_disable_verity(&img),
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_pick_fb_zip(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Fastboot Update ZIP", &["zip"])
                .pick_file()
            {
                if let Some(ui) = weak.upgrade() {
                    ui.set_fb_update_zip(path.display().to_string().into());
                }
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_run_fb_update(move || {
            let Some(ui) = weak.upgrade() else { return };
            let zip = ui.get_fb_update_zip().to_string();
            if zip.trim().is_empty() {
                return;
            }
            spawn_task(weak.clone(), "Обновление ZIP…", "Fastboot update", None, move || {
                android::fastboot_update_zip(&zip)
            });
        });
    }

    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_set_active_slot(move |slot: SharedString| {
            let s = slot.to_string();
            spawn_task(
                weak.clone(),
                format!("Смена слота на {}…", s.to_uppercase()),
                format!("Fastboot --set-active={s}"),
                Some(Arc::clone(&db)),
                move || android::set_active_slot(&s),
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_erase_partition(move || {
            let Some(ui) = weak.upgrade() else { return };
            let part = ui.get_fb_partition().to_string();
            if part.trim().is_empty() {
                return;
            }
            spawn_task(
                weak.clone(),
                format!("Стирание {part}…"),
                format!("Fastboot erase {part}"),
                None,
                move || android::erase_partition(&part),
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_wipe_userdata(move || {
            spawn_task(
                weak.clone(),
                "Сброс данных (-w)…",
                "Fastboot -w",
                None,
                android::wipe_userdata,
            );
        });
    }

    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_unlock_bootloader(move || {
            spawn_task(
                weak.clone(),
                "Разблокировка…",
                "Fastboot flashing unlock",
                Some(Arc::clone(&db)),
                android::unlock_bootloader,
            );
        });
    }

    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_unlock_critical(move || {
            spawn_task(
                weak.clone(),
                "Unlock Critical…",
                "Fastboot flashing unlock_critical",
                Some(Arc::clone(&db)),
                android::unlock_bootloader_critical,
            );
        });
    }

    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_oem_unlock(move || {
            spawn_task(
                weak.clone(),
                "OEM Unlock…",
                "Fastboot oem unlock",
                Some(Arc::clone(&db)),
                android::oem_unlock,
            );
        });
    }

    {
        let weak = ui.as_weak();
        let db = Arc::clone(&db);
        ui.on_lock_bootloader(move || {
            spawn_task(
                weak.clone(),
                "Блокировка загрузчика…",
                "Fastboot flashing lock",
                Some(Arc::clone(&db)),
                android::lock_bootloader,
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_fb_reboot(move |mode: SharedString| {
            let m = mode.to_string();
            let title = if m.is_empty() {
                "Fastboot reboot".to_string()
            } else {
                format!("Fastboot reboot ({m})")
            };
            spawn_task(weak.clone(), "Перезагрузка…", title, None, move || {
                android::fastboot_reboot(&m)
            });
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_fb_power_off(move || {
            spawn_task(
                weak.clone(),
                "Выключение…",
                "Fastboot oem poweroff",
                None,
                android::fastboot_power_off,
            );
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_run_getvar(move || {
            let Some(ui) = weak.upgrade() else { return };
            let var = ui.get_fb_getvar_name().to_string();
            spawn_task(
                weak.clone(),
                format!("getvar {var}…"),
                format!("Fastboot getvar {var}"),
                None,
                move || android::get_fastboot_var(&var),
            );
        });
    }

    // ─── Терминал ───
    {
        let weak = ui.as_weak();
        ui.on_run_custom_command(move || {
            let Some(ui) = weak.upgrade() else { return };
            let tool = ui.get_custom_tool().to_string();
            let args = ui.get_custom_args().to_string();
            if args.trim().is_empty() {
                return;
            }
            let title = format!("{tool} {args}");
            spawn_task(weak.clone(), "Выполнение команды…", title, None, move || {
                android::run_custom_command(&tool, &args)
            });
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_clear_log(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_log_text(SharedString::new());
                ui.set_last_log_line("Журнал очищен".into());
            }
        });
    }

    {
        let weak = ui.as_weak();
        ui.on_save_log(move || {
            let Some(ui) = weak.upgrade() else { return };
            let content = ui.get_log_text().to_string();
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Text Log", &["txt", "log"])
                .set_file_name("droidforge_log.txt")
                .save_file()
            {
                match std::fs::write(&path, content) {
                    Ok(()) => append_log(
                        &ui,
                        "OK",
                        "Сохранение журнала",
                        &format!("Сохранено в {}", path.display()),
                    ),
                    Err(e) => append_log(&ui, "ERR", "Сохранение журнала", &e.to_string()),
                }
            }
        });
    }

    ui.run()
}
