# Skadi Visual Core: Canvas v0 Contract

Дата: 2026-07-30
Статус: **accepted / experimental implementation**

## 1. Цель

Canvas v0 проверяет минимальный practical-first visual core без превращения
Skadi в graphics framework:

```text
portable software Canvas + separate platform presenter
```

Публичное API должно оставаться маленьким, предсказуемым и совместимым с
ресурсной моделью языка.

## 2. Типы

| Тип | Категория | Представление v0 |
|---|---|---|
| `Color` | value-safe | RGBA8 |
| `Rect` | value-safe | четыре `f64`: `x`, `y`, `width`, `height` |
| `Canvas` | linear resource | размеры и owning RGBA framebuffer |
| `Window` | linear resource | platform presentation handle |
| `Input` | value-safe | снимок клавиатуры и мыши за кадр |
| `MouseInput` | value-safe | позиция, движение, прокрутка и кнопки |
| `ButtonState` | value-safe | независимые `down`, `pressed`, `released` |
| `Key`, `MouseButton` | nominal values | платформенно-независимые имена клавиш и кнопок |

`Color` и `Rect` копируются по значению и разрешены в struct/List/Task/Channel.
`Canvas` и `Window` не являются task-safe или channel-safe.

Для `Canvas` и `Window` запрещены:

- копирование и повторное присваивание;
- хранение в `constant`;
- возврат из функции;
- параметр по значению;
- передача через Task/Channel.

Ресурсный параметр обязан быть `edit` или `view`. Изменяющие методы
Canvas требуют mutable receiver. `Window.present` принимает
`edit Canvas`, делая borrow явным в месте вызова.

## 3. Конструкторы

```skadi
new Color rgba = color(129, 162, 190, 255)
new Color hex = color_hex("#81a2be", 255)
new Rect area = rect(8.0, 8.0, 64.0, 32.0)
Canvas frame = canvas(320, 200)
Window window = windows.open("Title", 320, 200)
```

Контракты:

- компоненты `color` и alpha имеют тип `Int` и runtime-диапазон `0..255`;
- `color_hex` принимает `rrggbb` или `#rrggbb`, ровно шесть hex-цифр;
- `rect` принимает numeric-аргументы и хранит их как `f64`;
- размеры Canvas/Window имеют тип `Int` и должны быть положительными;
- ошибка visual runtime использует код `SC-RT-330`.

## 4. Цветовая палитра

Короткие алиасы `Color.black/red/green/yellow/blue/magenta/cyan/white` используют
мягкие оттенки терминальной палитры. Они намеренно не равны чистым
`#ff0000`, `#00ff00` и `#0000ff`.

Полный стабильный vocabulary Canvas v0:

```text
Color.terminal_black
Color.terminal_red
Color.terminal_green
Color.terminal_yellow
Color.terminal_blue
Color.terminal_magenta
Color.terminal_cyan
Color.terminal_white
Color.terminal_bright_black
Color.terminal_bright_red
Color.terminal_bright_green
Color.terminal_bright_yellow
Color.terminal_bright_blue
Color.terminal_bright_magenta
Color.terminal_bright_cyan
Color.terminal_bright_white
Color.transparent
```

Конкретные RGB-значения являются частью visual regression baseline v0, но могут
быть пересмотрены до объявления API stable.

## 5. Rasterizer

Canvas v0 реализует:

```text
clear
pixel
line
rect
fill_rect
circle
fill_circle
checksum
```

Инварианты:

- software rendering и одинаковый алгоритм на host-платформах;
- source-over alpha blending;
- координаты `Vec2`/`Rect` округляются одинаковым half-away-from-zero правилом;
- drawing за границами безопасно клиппируется;
- отрицательный radius вызывает `SC-RT-330`;
- неположительный размер Rect является no-op;
- drawing operation не выполняет скрытых аллокаций;
- `checksum()` существует для deterministic headless/CI regression tests.

## 6. Backend

Canvas не знает об окне. `Window` показывает уже готовый framebuffer:

```skadi
window.present(edit frame)
```

Первый backend:

- Win32 window creation;
- 32-bit software framebuffer presentation через GDI;
- обработка минимальной очереди сообщений в `present`/`is_open`;
- явные `is_open()` и `close()`;
- deterministic cleanup при выходе из scope.

Headless Canvas не должен включать или линковать Window backend. Официальный CLI
добавляет `gdi32` только для Windows toolchain; будущие backend-библиотеки должны
подключаться аналогично.

На не-Windows target вызов `windows.open` пока завершается `SC-RT-330`.

### 6.1. Win32 Input slice

`Window` хранит живое состояние ввода, а `window.input` отдаёт копируемый
`Input` текущего кадра. В пользовательском цикле нет отдельного `poll`:
`is_open()` и `present()` обслуживают Win32 messages. Переходы клавиш и
кнопок накапливаются между кадрами; первое чтение после `present()` публикует
их как один снимок. Повторное чтение в том же кадре не стирает edge flags.

`input.key(Key.W)` и `mouse.button(MouseButton.Left)` возвращают
`ButtonState`. `down` отражает последнее состояние, `pressed` и `released`
независимо фиксируют хотя бы один переход за кадр. Нажатие и отпускание
между кадрами выставляет оба edge flags. `MouseInput` содержит `position`,
`delta` и `wheel`; координаты относятся к клиентской области Window, начало
слева сверху. Потеря фокуса снимает удерживаемые состояния.

Win32 WndProc хранит указатель на heap-allocated `SkWindowState`: возвращение
или `move` значения `Window` не меняет адрес состояния, а owner cleanup
освобождает его после уничтожения HWND. `Canvas` не зависит от Input.

## 7. Отложено

- ordered event stream, layout-aware text input, touchscreen и gamepad;
- resize, scaling и DPI policy;
- `Image`, text/fonts и codecs;
- transforms, clipping stack и `Matrix2D`;
- Linux/macOS оконные backend;
- embedded display adapters;
- GPU acceleration;
- scene graph, UI layout и retained-mode abstractions.

Эти пункты расширяют backend и visual vocabulary, но не должны менять базовую
формулу: Canvas рисует, backend показывает.

## 8. Release gate

Canvas v0 считается пригодным для текущей experimental-линии, если:

- frontend проверяет типы, методы и resource/borrow правила;
- generated C компилируется для headless Canvas;
- Win32 lowering содержит отдельный presenter и правильные link flags;
- deterministic scene закреплена контрольной суммой;
- showcase использует `color_hex`, палитру, Rect и Circle;
- пользовательская RU/EN документация синхронизирована с реализацией.
