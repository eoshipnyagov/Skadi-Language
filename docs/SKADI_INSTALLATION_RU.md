# Установка Skadi CLI

Текущая контрольная версия: `v1.2.0-rc.1`.

Установщик размещает готовый `skadi-cli` в пользовательской директории,
проверяет SHA-256 архива и сохраняет манифест установки. Rust и Cargo для
обычной работы после установки не нужны.

!!! note
    Skadi генерирует C-код, поэтому для `build` и `run` нужен внешний C-компилятор.
    Установщик намеренно не изменяет системный toolchain. Команда
    `skadi-cli doctor` покажет, что найдено и чего не хватает.

## Windows

Откройте PowerShell:

```powershell
$installer = Join-Path $env:TEMP "skadi-install.ps1"
Invoke-WebRequest `
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/install.ps1 `
  -OutFile $installer
& $installer -Version 1.2.0-rc.1
```

По умолчанию бинарник устанавливается в:

```text
%LOCALAPPDATA%\Programs\Skadi\bin\skadi-cli.exe
```

Установщик добавляет эту директорию в пользовательский `PATH`, не меняя
системный `PATH`. Перезапустите терминал и проверьте окружение:

```powershell
skadi-cli --version
skadi-cli doctor
```

Для `build` и `run` подойдёт один из вариантов:

- MSVC Build Tools с `cl`;
- MSYS2/MinGW-w64 с `gcc`.

## Linux

```bash
curl -fsSL \
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/install.sh \
  | sh -s -- --version 1.2.0-rc.1
```

По умолчанию устанавливается статически собранный musl-бинарник:

```text
~/.local/bin/skadi-cli
```

Поддерживаемые архитектуры: `x86_64` и `aarch64`. Для системной установки:

```bash
sh install.sh --version 1.2.0-rc.1 --system
```

Установщик использует `sudo` только при явном `--system`. Для пользовательской
установки права администратора не нужны.

Установите C toolchain средствами своего дистрибутива:

```bash
# Debian / Ubuntu
sudo apt install build-essential

# Fedora
sudo dnf group install "Development Tools"

# Arch Linux
sudo pacman -S base-devel

# Alpine
sudo apk add build-base
```

## macOS

Команда установки такая же, как в Linux:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/install.sh \
  | sh -s -- --version 1.2.0-rc.1
```

Доступны отдельные архивы для Intel (`x86_64`) и Apple Silicon (`aarch64`).
Для C toolchain установите Command Line Tools:

```bash
xcode-select --install
```

RC-бинарники пока не подписаны Apple Developer ID и не notarized. Установщик
не отключает Gatekeeper и не удаляет quarantine-атрибуты. Если корпоративная
политика запрещает неподписанные CLI-инструменты, соберите `skadi-cli` из
исходников или дождитесь подписанного релиза.

## Первый проект

В рабочей директории:

```bash
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format --check
skadi-cli build
skadi-cli run
```

Интерактивный путь:

```bash
skadi-cli tui
```

## Явная версия и обновление

Повторный запуск установщика с новой `--version` атомарно заменяет управляемый
бинарник и обновляет манифест. Пользовательские проекты и `Skadi.toml` не
затрагиваются.

Без `--version` установщик запрашивает последний опубликованный GitHub Release.
Для prerelease/RC всегда указывайте версию явно.

## Офлайн-установка

Скачайте подходящий архив и `SHA256SUMS` со страницы GitHub Releases, затем
передайте их установщику.

Windows:

```powershell
.\install.ps1 `
  -Version 1.2.0-rc.1 `
  -ArchivePath .\skadi-cli-v1.2.0-rc.1-x86_64-pc-windows-msvc.zip `
  -ChecksumPath .\SHA256SUMS
```

Linux/macOS:

```bash
sh install.sh \
  --version 1.2.0-rc.1 \
  --archive ./skadi-cli-v1.2.0-rc.1-x86_64-unknown-linux-musl.tar.gz \
  --checksum-file ./SHA256SUMS
```

Имя локального архива должно соответствовать версии и текущей платформе.

## Удаление

Windows:

```powershell
$uninstaller = Join-Path $env:TEMP "skadi-uninstall.ps1"
Invoke-WebRequest `
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/uninstall.ps1 `
  -OutFile $uninstaller
& $uninstaller
```

Linux/macOS:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/uninstall.sh \
  | sh
```

Удаление опирается на манифест и удаляет только бинарник и PATH-блок, созданные
установщиком. Проекты, исходные файлы и пользовательская конфигурация
сохраняются.

## Установка из исходников

Этот путь нужен разработчикам компилятора:

```bash
cargo build --release -p skadi-cli
cargo run -p skadi-cli -- --version
```

Для обычной работы используйте готовый release archive и команду `skadi-cli`.
