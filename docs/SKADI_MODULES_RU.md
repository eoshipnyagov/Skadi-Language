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
