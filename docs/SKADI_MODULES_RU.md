# Файлы, импорты и видимость

## Относительный import

```skadi
import "./math.skd"

new Int result = math.add(2, 3)
new math.Point origin = {x = 0.0, y = 0.0}
```

Имя qualifier берётся из имени файла без `.skd`. Import graph загружает
`skadi-cli`; core parser получает уже объединённую программу.

Импорту можно дать локальный alias:

```skadi
import "./long_player_model.skd" as player

new player.State state = player.initial_state()
```

Alias виден только внутри файла, где записан import. Один и тот же модуль,
включённый по разным ветвям diamond-графа, загружается один раз по
каноническому пути.

## Правило прямого импорта

Файл видит public symbol только из модуля, который импортирован напрямую.
Транзитивный import не становится неявным re-export. Это удерживает зависимости
видимыми и диагностируется через `SC-MOD-003`.

## Public и local

Top-level `fn`, `struct`, `label` и `tag` экспортируются по умолчанию. Prefix `local`
оставляет symbol внутри файла:

```skadi
local fn parse_line(Text line) returns Int {
    return len(line)
}
```

Коллизии public symbols получают `SC-MOD-002`; отсутствующий файл и цикл
imports — `SC-MOD-001`.

## Текущие границы

Пока отсутствуют:

- imports по module/package name;
- re-export;
- dependency registry;
- отдельная module declaration.

Для переносимости всегда используйте относительные пути проекта, а не абсолютные
пути машины.

## Принятое направление: packages и C-библиотеки

Подключение пакетов и C-библиотек является обязательной частью пути к полной
версии языка, но пока не является рабочим синтаксисом.

Package layer должен добавить зависимости в `Skadi.toml`, локальные и Git
sources, воспроизводимый lock-файл, единое разрешение diamond graph и imports по
имени пакета. Транзитивная зависимость не должна автоматически становиться
видимой: файл по-прежнему явно импортирует то, чем пользуется.

C interoperability вводится отдельным ограниченным ABI-контрактом, а не
разрешением вставлять произвольный C в Skadi. Первый срез должен покрыть:

- функции с fixed-width integers и `f32/f64`;
- C-compatible structs с явным layout;
- opaque owning/borrowed handles;
- buffers с явной длиной;
- include paths, library paths и linker names из `Skadi.toml`;
- явные правила владения для `char*`, buffers, callbacks и resource handles.

Точный declaration syntax будет принят только вместе с проверкой нескольких
реальных библиотек. Автоматическая генерация bindings из простых headers должна
строиться поверх того же ABI-контракта, а не создавать вторую модель FFI.
