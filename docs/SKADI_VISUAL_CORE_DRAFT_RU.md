# Skadi Visual Core (Draft RU)

Дата: 2026-06-04
Статус: draft / design reference
Назначение: зафиксировать опорную модель будущего visual-core слоя Skadi до полноценной реализации syntax/runtime/backend.

Связанный рабочий документ:

- [Visual Core MVP Contract](visual-core-mvp.md)

## 1. Основная идея

Skadi должен считать визуальный вывод не случайной внешней библиотекой, а одной из естественных системных возможностей языка.

Это особенно хорошо ложится на те области, под которые язык уже концептуально подходит:

- игры;
- embedded;
- операторские панели;
- симуляции;
- runtime-инструменты;
- визуальные утилиты, где важно явно контролировать update/draw/present цикл.

Главная формула Visual Core:

```text
Canvas is a core drawing abstraction.
Backends are platform modules.
```

По-русски:

```text
Canvas — это ядровая абстракция рисования.
Конкретные окна, дисплеи и framebuffer-поверхности — платформенные модули.
```

Это позволяет получить один и тот же стиль кода для:

- desktop window;
- framebuffer;
- OLED/TFT display;
- offscreen image;
- простого software renderer.

## 2. Что входит в Visual Core как идея

Вокруг `Canvas` в будущем может существовать связанный набор базовых типов:

```text
Canvas
Color
Vec2
Vec3
Vec4
Rect
Size
Image
Matrix2D
PixelFormat
```

Это не должно автоматически превращать язык в игровой движок или GUI framework.

Visual Core отвечает за:

- базовую поверхность рисования;
- примитивы 2D-отрисовки;
- цвет;
- координатные и геометрические типы;
- image/framebuffer story;
- transform/present model.

Visual Core не должен тащить в ядро:

- scene graph;
- ECS;
- animation system;
- physics;
- layout engine;
- retained UI framework;
- shader language;
- asset manager;
- full font shaping;
- большой graphics-engine runtime.

## 3. `Canvas` как центральная абстракция

`Canvas` — это поверхность рисования, а не конкретная платформа.

Пользовательский код должен выглядеть примерно так:

```scadi
fn draw_status(Canvas canvas, Float temperature, Bool alarm) {
    canvas.clear(Color.black)

    canvas.text(Vec2(4, 4), "Temperature")
    canvas.text(Vec2(4, 18), text(temperature) + " C")

    canvas.rect(Rect(4, 40, 100, 10), Color.gray)

    new Int bar_width = clamp(Int(temperature * 2), 0, 100)
    canvas.fill_rect(Rect(4, 40, bar_width, 10), Color.green)

    if alarm {
        canvas.text(Vec2(4, 56), "ALARM", Color.red)
    }
}
```

Смысл в том, что `draw_status` не знает, рисует ли он:

- в окно;
- в offscreen image;
- в OLED display;
- в software framebuffer.

Это решает backend.

## 4. Backends должны быть отдельным слоем

Visual Core не должен вшивать платформу в язык.

Embedded-стиль:

```scadi
new screen = display.ssd1306(i2c, 128, 64) on error {
    output("Display not found")
    return
}

new canvas = screen.canvas()

draw_status(canvas, temperature, alarm)
screen.present()
```

Desktop/game-window стиль:

```scadi
new window = graphics.window(800, 600, "Panel") on error {
    output("Cannot create window")
    return
}

new canvas = window.canvas()

draw_status(canvas, temperature, alarm)
window.present()
```

Это даёт одну mental model и для embedded, и для desktop.

## 5. Immediate-mode как базовый подход

Для первого рабочего Visual Core логично брать immediate-mode, а не retained-mode.

Базовый паттерн:

```scadi
canvas.clear(Color.black)
draw_world(canvas, world)
draw_ui(canvas, ui)
window.present()
```

Почему это особенно хорошо подходит Skadi:

- меньше скрытого состояния;
- легче объяснять lifetime и стоимость операций;
- проще память;
- естественно сочетается с game loop;
- хорошо подходит для embedded и deterministic UI/tooling;
- retained UI можно строить поверх этого позже.

## 6. Базовые типы будущего слоя

### `Vec2`

Базовый 2D-вектор / позиция.

```scadi
new pos = Vec2(10, 20)
```

### `Vec3`, `Vec4`

Нужны не только графике, но и математике, симуляциям, цвету, физике и game logic.

### `Rect`

Базовая 2D-геометрия для UI и drawing primitives.

```scadi
new area = Rect(10, 20, 100, 50)
```

### `Size`

Размер без позиции.

```scadi
new size = Size(128, 64)
```

### `Color`

Базовый визуальный тип:

```scadi
new red = Color.rgb(255, 0, 0)
new transparent = Color.rgba(0, 0, 0, 128)
```

Полезны и встроенные цвета:

```text
Color.black
Color.white
Color.red
Color.green
Color.blue
Color.gray
Color.transparent
```

### `Image`

Bitmap/offscreen surface/framebuffer object.

### `Matrix2D`

Базовая 2D-трансформация для смещений, масштаба, поворота и локальных координатных систем.

### `PixelFormat`

Будущий общий способ говорить о формате пикселей:

```text
PixelFormat.Mono1
PixelFormat.Gray8
PixelFormat.RGB565
PixelFormat.RGB888
PixelFormat.RGBA8888
```

## 7. Базовые Canvas-операции v0

Для первого среза достаточно малого, но цельного API:

```scadi
canvas.clear(Color color)

canvas.pixel(Vec2 pos, Color color)
canvas.line(Vec2 a, Vec2 b, Color color)

canvas.rect(Rect rect, Color color)
canvas.fill_rect(Rect rect, Color color)

canvas.circle(Vec2 center, Float radius, Color color)
canvas.fill_circle(Vec2 center, Float radius, Color color)

canvas.text(Vec2 pos, Text text)
canvas.text(Vec2 pos, Text text, Color color)

canvas.image(Vec2 pos, Image image)

canvas.push_transform(Matrix2D transform)
canvas.pop_transform()
```

Этого уже достаточно для:

- статусных экранов;
- простых dashboard/panel сценариев;
- debug overlay;
- embedded drawing;
- 2D prototypes;
- небольших visual tools.

## 8. Что можно добавить позже

Осознанно более поздние расширения:

```scadi
canvas.triangle(...)
canvas.fill_triangle(...)

canvas.round_rect(...)
canvas.fill_round_rect(...)

canvas.polyline(...)
canvas.polygon(...)
canvas.fill_polygon(...)

canvas.push_clip(Rect area)
canvas.pop_clip()

canvas.measure_text(Text text) returns Size
canvas.text_style(...)
```

То есть Visual Core v0 должен быть маленьким, а не сразу “полным графическим стеком”.

## 9. Transform model

Базовый и наиболее честный слой — матричный:

```scadi
canvas.push_transform(Matrix2D.translate(Vec2(100, 50)))
canvas.fill_circle(Vec2(0, 0), 20, Color.red)
canvas.pop_transform()
```

Поверх него позже могут появиться более удобные sugar-формы, но фундаментом лучше оставить:

```text
Matrix2D
push_transform
pop_transform
```

## 10. Canvas и memory model

Visual Core очень хорошо сочетается с уже намеченной memory model.

Пример frame-memory:

```scadi
fn game_loop(Window window, World world) {
    Memory frame_memory = memory(16mb) on error {
        output("Cannot allocate frame memory")
        return
    }

    new canvas = window.canvas()

    loop {
        frame_memory.clear()

        place in frame_memory on error {
            output("Frame memory overflow")
            continue
        } {
            canvas.clear(Color.black)

            draw_world(canvas, world)
            draw_ui(canvas, world.ui)

            window.present()
        }
    }
}
```

Это хорошо подходит под идею:

```text
всё временное за кадр живёт в frame_memory и умирает разом
```

Отсюда важный проектный принцип:

> базовые Canvas-операции не должны неожиданно прятать тяжёлые аллокации и неявную memory-магии.

## 11. Canvas и task/channel model

Visual Core так же хорошо сочетается с task model.

Типовой embedded/game-like сценарий:

- отдельная задача собирает события/данные;
- главный цикл рисует;
- связь идёт через `Channel`;
- сам `Canvas` остаётся обычной абстракцией рисования, а не concurrency-runtime объектом.

Это хорошо поддерживает общую философию языка:

- без shared mutable chaos по умолчанию;
- с явной передачей данных;
- с читаемым main loop.

## 12. Coordinate system

Для первого visual-core среза разумно считать систему координат простой и предсказуемой:

```text
origin: top-left
x grows right
y grows down
```

Это естественно для:

- UI;
- framebuffer;
- embedded displays;
- большинства базовых Canvas API.

Если какому-то приложению нужна другая система координат, её можно выразить через `Matrix2D`.

## 13. Present / flush

Сама отрисовка и показ результата — это не одно и то же.

В разных backend-ах это может выглядеть так:

```scadi
screen.present()
window.present()
framebuffer.flush()
```

Общий смысл:

```text
present = показать готовый кадр
flush   = отправить буфер на устройство
```

Для унификации как основной термин лучше держать именно `present()`.

## 14. Visual Core и MWP

Для первого демонстратора будущего visual слоя достаточно:

```text
Vec2
Rect
Color
Canvas
Image или framebuffer
clear
pixel
line
rect
fill_rect
text
present
```

Desktop backend для MWP может быть очень простым:

- software framebuffer;
- render в BMP/PPM;
- или минимальное окно через лёгкий C/runtime backend.

Для visual-demo не нужен сразу GPU-движок.

## 15. Что не входит в MWP

Не нужно обещать слишком рано:

- OpenGL/Vulkan/DirectX;
- shader system;
- full image codecs;
- full font shaping;
- retained UI;
- layout engine;
- scene graph;
- SVG;
- anti-aliasing as required baseline;
- GPU batching;
- materials/camera/animation stack.

Иначе маленький и полезный Visual Core мгновенно превращается в бесконечный graphics roadmap.

## 16. Compiler / semantic implications

Если этот слой когда-нибудь пойдёт в реализацию, компилятору и runtime придётся учитывать хотя бы такие моменты:

1. `Canvas`, `Color`, `Vec*`, `Rect`, `Size`, `Image`, `Matrix2D` должны иметь чёткий type contract.
2. Базовые Canvas-вызовы должны быть проверяемы по сигнатурам и арности как обычные builtins/method-like operations.
3. Стоимость drawing operations не должна маскироваться неявной тяжёлой runtime-магией.
4. Временные visual-данные должны хорошо стыковаться с `Memory`.
5. Передача visual state между `Task`-ами должна подчиняться тем же безопасным правилам, что и остальная value/message model.

## 17. Сильные стороны идеи

Visual Core очень органично сочетается с философией Skadi:

- visual output становится first-class, а не случайной библиотекой;
- embedded и games получают общую базу;
- drawing code остаётся читаемым;
- нет тяжёлого GUI framework по умолчанию;
- нет скрытого retained scene graph;
- это хорошо стыкуется с memory model;
- это хорошо стыкуется с task/channel model;
- visual demos дают очень наглядный результат для будущих language milestones.

Главное достоинство:

> Skadi может стать языком не только для детерминированных систем, но и для детерминированных визуальных систем.

## 18. Риски и точки аккуратности

Есть несколько мест, где легко перегнуть:

### 18.1. Нельзя превратить Visual Core в движок

Если в ядро слишком рано попадут scene graph, UI framework, layout или большой asset/runtime stack, язык начнёт размываться.

### 18.2. Нельзя скрывать дорогие операции

Text rendering, image decode, font shaping и любые тяжёлые visual-path операции должны быть либо простыми, либо явными отдельными слоями.

### 18.3. Нужно заранее решить allocation story

Очень важно ответить на вопрос:

```text
что именно Canvas / Image / text drawing имеют право аллоцировать неявно?
```

Для Skadi наиболее естественный ответ:

```text
как можно меньше и как можно предсказуемее
```

### 18.4. Нужен чёткий boundary между core и backend

`Canvas` как абстракция должен быть стабильнее, чем platform backends, иначе visual layer быстро станет слишком platform-shaped.

## 19. Рекомендуемый путь внедрения

Если когда-нибудь доходить до реализации practically-first, хороший порядок такой:

1. Зафиксировать словарь:

   - `Canvas`
   - `Color`
   - `Vec2`
   - `Rect`
   - `Image`
   - `Matrix2D`
   - `present`
2. Принять immediate-mode как базовую модель.
3. Реализовать очень маленький `Canvas v0`.
4. Взять software/backend-first демонстратор, а не сразу тяжёлый GPU path.
5. Только потом обсуждать clipping, richer shapes, text measurement и более крупные visual extensions.

## 20. Итоговая оценка

Как future-track идея для Skadi это очень сильное направление.

Почему:

- оно бьётся с общей философией языка;
- оно даёт наглядную прикладную ценность;
- оно хорошо дружит с memory model;
- оно хорошо дружит с task/channel model;
- оно открывает и embedded, и operator-panel, и small-game, и tooling сценарии;
- оно позволяет показать язык не только как “compiler toy”, а как среду для реально видимого результата.

Главная рекомендация:

> не пытаться сделать сразу “универсальную graphics platform”, а зафиксировать маленький и честный Canvas-first MWP, если и когда этот трек дойдёт до реализации.
