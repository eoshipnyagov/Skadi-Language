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

## Локальные package-зависимости

Проект может дать имя соседнему или vendored Skadi-пакету через
`[dependencies]`:

```toml
[dependencies]
physics = "../physics"
ui = "vendor/ui"
```

Путь задаётся относительно директории текущего `Skadi.toml`. Корень
зависимости обязан быть директорией с собственным `Skadi.toml`. После этого
модуль импортируется через имя зависимости и путь внутри неё:

```skadi
import "physics/src/vector.skd" as vectors

new vectors.Body body = vectors.make_body()
```

Первый компонент строки import — имя из `[dependencies]`; qualifier по
умолчанию всё равно берётся из имени файла (`vector` в примере), а `as` даёт
локальный alias. Импорт с `.` или `..` внутри package path запрещён, и
канонический путь не может выйти из корня зависимости. Неизвестная зависимость,
неверный package root и попытка выхода получают `SC-MOD-004` внутри общей
module-stage диагностики.

Прямые импорты, diamond-deduplication, `local` и правила коллизий одинаковы для
локальных файлов и package imports. Зависимость пакета не становится
автоматически видимой приложению.

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

- re-export;
- Git/registry dependencies и version constraints;
- transitive dependency resolution и lock-файл;
- отдельная module declaration.

Для переносимости всегда используйте относительные пути проекта, а не абсолютные
пути машины.

## Packages и C-библиотеки

Первый local-path package resolver реализован. Ограниченный C ABI уже доступен
через bodyless `external fn` и `[native]` в `Skadi.toml`; полный контракт и пример
описаны на странице [C ABI и native C](c-abi.md).

Следующий package layer должен добавить Git/registry sources, version
constraints, transitive resolution и воспроизводимый lock-файл. Эти контракты
не имитируются раньше времени: локальный resolver не создаёт пустой lock-файл.

C interoperability вводится отдельным ограниченным ABI-контрактом, а не
разрешением вставлять произвольный C в Skadi. Первый реализованный срез
покрывает функции с fixed-width integers, `f32/f64`, `Bool`, `Char`, `void`,
by-value `external struct`, opaque owning `external resource`, native C
sources, library paths и linker names.

Следующие срезы должны покрыть:

- custom/packed layout и проверку C headers;
- borrowed handles с lifetime, принадлежащим внешней библиотеке;
- долгоживущие buffers и ownership beyond call-scoped `Buffer(T)`;
- include/header contract;
- явные правила владения для `char*`, buffers, callbacks и resource handles.

Текущий owning resource syntax проверен native smoke, но остаётся experimental
до испытания на нескольких реальных библиотеках. Автоматическая генерация bindings из простых headers должна
строиться поверх того же ABI-контракта, а не создавать вторую модель FFI.
