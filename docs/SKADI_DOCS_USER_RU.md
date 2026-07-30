# Пользовательская документация Skadi

Текущая распространяемая версия — `v1.2.0-rc.1`: stable base `v1.1` плюс
experimental, но исполняемые Memory, Task/Channel, Time/Duration, ByteSize,
Angle и Vector MVP.

## 1. Начать

- [Быстрый старт](getting-started.md) — идея языка, установка, первый проект.
- [Установка](installation.md) — Windows, Linux, macOS, checksums и uninstall.
- [Переход с v1.1](v1-2-migration.md) — совместимость и новые systems types.

## 2. Пользоваться toolchain

- [Справочник CLI/TUI](cli-reference.md) — важные команды, полный перечень,
  diagnostics, targets, TUI и planned work.
- [Короткий CLI-рецепт](cli-quick-start.md) — минимальная последовательность
  команд без языкового руководства.

## 3. Найти форму языка

- [Быстрая справка](language-quick-reference.md) — полная таблица syntax,
  types, constants, builtins, statuses и future reservations.
- [Полная справка](language-reference.md) — тематический каталог.
- [Как писать и чего избегать](practices.md) — канонический стиль и антипримеры.

## 4. Изучить по темам

- [Типы и выражения](language-basics.md)
- [Ветвления и циклы](control-flow.md)
- [Функции и ошибки](functions-errors.md)
- [Struct, Text и List](data-model.md)
- [Модули](modules.md)
- [I/O и файлы](io-files.md)
- [Математика](math.md)
- [Memory](memory.md)
- [Владение и передача ресурсов](ownership.md)
- [Task и Channel](concurrency.md)
- [Time/Duration](time-duration.md)
- [ByteSize](byte-size.md)
- [Angle](angle.md)
- [Vectors](vectors.md)

## 5. Платформы, будущее и примеры

- [Embedded status](embedded.md)
- [Canvas / Visual Core](canvas.md)
- [Короткие примеры](language-examples.md)
- [Showcase-программы](showcases.md)
- [Skadi для AI-assisted разработки](ai-guide.md)

Внутренние compiler contracts, historical plans и design drafts находятся в
[разделе разработки](../internal/index.md).
