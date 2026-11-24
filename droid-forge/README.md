# DroidForge 2.0

Нативное десктопное приложение на **Rust + Slint** для работы с **ADB** и **Fastboot** в спокойном стиле **Google Material Design 3** (светлая и тёмная темы) с живым превью подключённого смартфона в режиме Fastboot / ADB / Recovery.

## Что нового в 2.0

- **Интерфейс на Slint (Material Design 3)**:
  - Классический Navigation Rail, сдержанная палитра Google Blue (`#0B57D0` / `#A8C7FA`), тональная иерархия поверхностей и векторные иконки Material Symbols.
  - Переключатель светлой и тёмной темы.
  - Полностью неблокирующее выполнение команд в фоновых потоках (интерфейс не зависает при прошивке и установке APK).
  - Скрытый запуск консольных утилит на Windows (`CREATE_NO_WINDOW` — без мигающих окон `cmd.exe`).
- **Интерактивное отображение смартфона (Device Preview)**:
  - Реалистичный корпус с кнопками питания и громкости и живым экраном **Fastboot / Fastbootd** (`START ▸` на уровне кнопки питания, Bugdroid, `PRODUCT`, `BL`, `SLOT`, `STATE`, `SN`).
  - Отдельные состояния экрана для **ADB** (с уровнем заряда батареи), **Recovery**, **ADB Sideload**, **Unauthorized** и режима ожидания.
  - Автоматическое определение модели и SoC по расширенной базе `devices.json` (Pixel, Xiaomi / Redmi / POCO, OnePlus, Nothing, Meta Quest).
- **Расширенный набор команд ADB**:
  - Установка APK с опциями `-d` (откат версии) и `-g` (выдача разрешений), пакетная установка Split APK (`install-multiple`), `adb sideload`.
  - Управление пакетами: список сторонних приложений (`pm list packages -3`), очистка данных (`pm clear`), отключение (`pm disable-user`), удаление.
  - Отправка и загрузка файлов (`push` / `pull`), исправленный бинарный скриншот PNG (`exec-out screencap -p`), запись экрана (15 сек).
  - Беспроводной Wi-Fi ADB (`tcpip 5555`, `connect`, `disconnect`), подробная информация об устройстве, сохранение и очистка `logcat`, перезапуск сервера ADB.
  - Перезагрузка в систему, Bootloader, Recovery, Fastbootd, EDL (9008) и выключение.
- **Расширенный набор команд Fastboot**:
  - Быстрый выбор разделов (`boot`, `init_boot`, `vendor_boot`, `recovery`, `vbmeta`, `dtbo`, `super`) или ввод произвольного раздела.
  - Прошивка разделов, разовая загрузка образа (`fastboot boot`), прошивка `vbmeta` с `--disable-verity --disable-verification`, установка factory update ZIP (`fastboot update`).
  - Переключение активных слотов A/B (`--set-active=a/b`), стирание раздела (`erase`) и сброс данных (`fastboot -w`).
  - Управление загрузчиком: `flashing unlock`, `flashing unlock_critical`, `oem unlock`, `flashing lock`.
  - Быстрые пресеты и ручной ввод для `fastboot getvar`.
  - Перезагрузка в систему, Bootloader, Fastbootd, Recovery и выключение (`oem poweroff`).
- **Встроенный терминал**:
  - Выполнение произвольных команд `adb` и `fastboot`, просмотр и сохранение журнала операций в `.txt`.

## Сборка из исходников

Требуется установленный [Rust](https://rustup.rs/) (stable) и `platform-tools` (`adb`, `fastboot`).

```bash
cargo build --release
```

Готовый исполняемый файл появится по пути `target/release/droidforge.exe` (Windows) или `target/release/droidforge` (Linux/macOS).  
DroidForge автоматически ищет `adb` и `fastboot` рядом с собой, в `%LOCALAPPDATA%\Android\Sdk\platform-tools`, а также в системном `PATH`.
