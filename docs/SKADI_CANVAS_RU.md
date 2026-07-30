# Canvas и Visual Core

Статус: **experimental Canvas v0** в ветке `v1.2`.

Canvas v0 — небольшой immediate-mode 2D-слой:

```text
Canvas рисует в программный framebuffer; платформенный Window показывает кадр.
```

Это не UI framework и не игровой движок. Текущий слой нужен, чтобы проверить
синтаксис, модель ресурсов, базовый rasterizer и отделение платформенного
backend от кода рисования.

## Быстрый пример

```skadi
Canvas frame = canvas(320, 200)
Window window = windows.open("Skadi Canvas", 320, 200)

new Color background = color_hex("#1d1f21", 255)
new Rect panel = rect(24.0, 24.0, 272.0, 152.0)
new Vec2 center = {x = 160.0, y = 100.0}

frame.clear(background)
frame.fill_rect(panel, Color.terminal_blue)
frame.circle(center, 48.0, Color.terminal_bright_yellow)
frame.fill_circle(center, 12.0, color(213, 78, 83, 192))

while window.is_open() {
    window.present(direct frame)
    sleep(16ms)
}
```

`Canvas` и `Window` являются ресурсами: их нельзя копировать, возвращать из
функции или передавать по значению. Для изменения Canvas из функции используйте
`direct Canvas`, для чтения — `view Canvas`.

## Типы и конструкторы

| Форма | Результат | Назначение |
|---|---|---|
| `color(r, g, b, a)` | `Color` | RGBA-компоненты `0..255` |
| `color_hex("#rrggbb", a)` | `Color` | Шесть hex-цифр и alpha `0..255` |
| `rect(x, y, width, height)` | `Rect` | Прямоугольник с `Float`-координатами |
| `canvas(width, height)` | `Canvas` | Программный RGBA framebuffer |
| `windows.open(title, width, height)` | `Window` | Win32-окно для показа Canvas |

`Color` и `Rect` — обычные копируемые значения. `Canvas` владеет выделенным
пиксельным буфером, а `Window` — платформенным окном. Компилятор освобождает оба
ресурса детерминированно.

## Рисование

| Метод Canvas | Действие |
|---|---|
| `clear(color)` | Заполнить весь кадр |
| `pixel(position, color)` | Нарисовать пиксель |
| `line(from, to, color)` | Нарисовать линию |
| `rect(area, color)` | Нарисовать рамку |
| `fill_rect(area, color)` | Заполнить прямоугольник |
| `circle(center, radius, color)` | Нарисовать окружность |
| `fill_circle(center, radius, color)` | Заполнить круг |
| `checksum()` | Получить стабильную контрольную сумму framebuffer |

Методы используют source-over alpha blending. Координаты округляются
детерминированно, а пиксели за границами Canvas безопасно отсекаются.
Отрицательный радиус является runtime-ошибкой; прямоугольник с неположительным
размером ничего не рисует.

## Цвета

Короткие имена `Color.red`, `Color.green`, `Color.blue` и другие используют
мягкие, пригодные для интерфейсов оттенки, а не чистые RGB-крайности.

Полная 16-цветная палитра:

| Базовые | Яркие |
|---|---|
| `Color.terminal_black` | `Color.terminal_bright_black` |
| `Color.terminal_red` | `Color.terminal_bright_red` |
| `Color.terminal_green` | `Color.terminal_bright_green` |
| `Color.terminal_yellow` | `Color.terminal_bright_yellow` |
| `Color.terminal_blue` | `Color.terminal_bright_blue` |
| `Color.terminal_magenta` | `Color.terminal_bright_magenta` |
| `Color.terminal_cyan` | `Color.terminal_bright_cyan` |
| `Color.terminal_white` | `Color.terminal_bright_white` |

Дополнительно доступен `Color.transparent`.

## Window backend

```skadi
while window.is_open() {
    window.present(direct frame)
    sleep(16ms)
}
```

В Canvas v0 `windows.open` реализован только для Win32. Headless Canvas,
рисование и `checksum()` не зависят от оконной системы и работают на текущих
host-целях. Размеры Window и Canvas при `present` должны совпадать.

`present()` и `is_open()` обрабатывают очередь сообщений Win32. Поэтому
длительную работу нельзя помещать между двумя итерациями presentation loop:
иначе операционная система справедливо сочтёт окно не отвечающим.

После условного `window.close()` вызов `present()` требует обработчик:

```skadi
window.present(direct frame) on error {
    pass
}
```

Закрытое окно остаётся безопасным sentinel-handle до автоматического cleanup;
handler не обращается к уже освобождённому Win32 handle.

## Что пока не входит

- события клавиатуры и мыши;
- непрерывный application loop и frame timing;
- text и fonts;
- images и codecs;
- transforms, clipping regions и `Matrix2D`;
- resize и масштабирование с сохранением логического размера;
- Linux/macOS/embedded display backends;
- GPU API, scene graph, retained UI и layout engine.

Технические решения и дальнейшие ограничения зафиксированы во
[внутреннем Canvas v0 contract](../internal/visual-core-mvp.md).
