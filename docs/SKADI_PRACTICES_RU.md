# Как писать на Skadi

## Предпочитайте

Канонический цикл и типы:

```skadi
new Bool ready = true
new Char marker = 'x'

iterate values as value {
    output(value)
}
```

Явные зависимости и возврат:

```skadi
import "./geometry.skd"

fn distance_to_origin(geometry.Point point) returns Float {
    return geometry.length(point)
}
```

Явное recovery:

```skadi
value = parse(source) on error {
    output("parse failed")
}
```

Единый рабочий цикл:

```powershell
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
```

## Не стоит

Не используйте lexer-only reservations:

```skadi
fixed Int limit = 10
direct import "./x.skd"
```

Не пишите C-style loop и legacy return syntax в новом коде:

```skadi
for (i = 0; i < 10; i++) {
    output(i)
}

fn old_style() Int {
    return 1
}
```

Не придумывайте error forms поверх обычного I/O:

```text
new Text data = read("input.txt") on error {
    output("failed")
}
```

Не передавайте mutable/capability state между tasks:

```skadi
new Int List values = [1, 2]
Channel(Int List) queue = channel(4)
queue.send(values)
```

## Почему

Стиль Skadi стремится держать стоимость, ownership, recovery и зависимости
видимыми. Если короткая запись скрывает важную системную границу, лучше выбрать
чуть более явную форму.

Полный список различий между рабочими и отложенными формами находится в
[быстрой справке](language-quick-reference.md).
