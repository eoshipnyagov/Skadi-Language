# Skadi Systems Additions (Draft RU)

Дата: 2026-06-04
Статус: draft / design reference
Назначение: зафиксировать набор будущих системных дополнений к Skadi, которые естественно вырастают из уже намеченных осей языка: `Memory`, `Task/Channel`, `Canvas`.

Связанный рабочий документ:

- [Systems Additions MVP Contract](systems-additions-mvp.md)

## 1. Зачем нужен этот документ

После memory model, task model и visual core становится видно, что вокруг них естественно формируется ещё один слой language-design решений.

Речь не о "добавить побольше фич", а о том, чтобы определить, какие дополнительные системные свойства органично поддерживают философию Skadi.

Этот слой касается прежде всего:

- времени;
- единиц измерения;
- жизненного цикла ресурсов;
- device abstraction;
- interrupt/event boundary;
- execution contexts;
- явных маркеров стоимости и риска;
- встроенной диагностики и policy-level статического анализа;
- data-oriented контейнеров;
- project/tooling-aware конфигурации.

## 2. Общая формула

Короткая версия:

```text
Memory explains where data lives.
Task/Channel explains how work flows.
Canvas explains how state is shown.
Time/Units explain how systems exist in time and magnitude.
Resources/Contexts/Diagnostics explain what is safe, costly, and allowed.
```

По-русски:

```text
Memory объясняет, где живут данные.
Task/Channel объясняет, как течёт работа.
Canvas объясняет, как система показывает состояние.
Time/Units объясняют, как система существует во времени и величинах.
Resources/Contexts/Diagnostics объясняют, что безопасно, дорого и допустимо.
```

## 3. `Time` / `Duration` как first-class слой

Для embedded, games, операторских панелей, firmware и симуляций время не может быть случайной библиотечной темой.

Естественные базовые сущности:

```text
Time
Duration
Timer
Rate
```

Хороший стиль:

```scadi
Duration sample_period = 100ms
Time started = now()

delay(500ms)

if elapsed(started) > 2s {
    output("timeout")
}
```

Главная польза:

- убираются "магические числа";
- единицы времени читаются прямо в коде;
- легче писать timeout, polling, debounce, loop-rate и frame logic.

## 4. Units literals

Поддержка единиц измерения очень хорошо ложится на системный характер языка, но должна вводиться осторожно.

Наиболее естественные ранние единицы:

```text
time: ms, s, min
memory: b, kb, mb
angle: deg, rad
rate: Hz
```

Возможные более поздние:

```text
V, mA, A, mm, cm, m, C
```

Это полезно, потому что код становится семантически явнее:

```scadi
delay(1s)
memory(32mb)
rotate(90deg)
set_pwm(1kHz)
```

А не:

```scadi
delay(1000)
set_pwm(1000)
rotate(90)
```

## 5. Device abstraction

Skadi целится в embedded и hardware-adjacent сценарии, поэтому такие понятия, как:

```text
gpio
i2c
spi
uart
timer
display
```

естественно лежат близко к ядру языка.

Но важна граница:

```text
язык знает общие device-абстракции,
платформа реализует конкретные драйверы
```

То есть Skadi может знать `display`, `gpio`, `i2c`, но не обязан тащить в ядро каждый конкретный чип, контроллер или дисплей.

## 6. Interrupt / event bridge

Для embedded это одна из самых сильных будущих тем.

Принцип:

```text
interrupt context cannot allocate, block, or call non-interrupt-safe functions
```

По-русски:

```text
в interrupt-контексте нельзя аллоцировать,
нельзя блокироваться
и нельзя вызывать функции, не являющиеся interrupt-safe
```

Хорошие примеры:

```scadi
on interrupt timer0 {
    ticks += 1
}
```

```scadi
on gpio.button.press {
    input_events.try_send(InputEvent.Button)
}
```

Плохие примеры:

```scadi
on interrupt timer0 {
    new buffer = List(u8)
    channel.receive()
    fs.write(path, data)
}
```

Это не просто синтаксис, а сильная semantic-защита от типичных embedded-ошибок.

## 7. Execution contexts

Очень сильная идея для будущего semantic layer — явные контексты выполнения.

Например:

```text
normal
task
interrupt
render
```

Идея не в том, чтобы завести новую runtime-магии, а в том, чтобы компилятор знал:

- где допустимы блокирующие операции;
- где допустимы аллокации;
- где допустим I/O;
- где нужно предупреждать о "тяжёлом" поведении.

Типовой пример:

```scadi
fn draw_ui(Canvas canvas) {
    new data = sensor_channel.receive()
}
```

В будущем это может быть корректным кодом синтаксически, но плохим архитектурно:

```text
warning: blocking receive inside render function
```

## 8. Resource lifecycle

Memory — не единственный ресурс.

Skadi должен так же аккуратно относиться к:

- файлам;
- портам;
- окнам;
- дисплеям;
- сенсорам;
- network connections;
- serial devices.

Хорошая общая формула:

```text
Resources close at scope end.
Memory clears at scope end.
Tasks must be waited/stopped.
```

Это делает lifetime системных объектов столь же читаемым, как lifetime данных.

## 9. Explicit cost / danger markers

У языка уже есть направление через `danger fn`.

В будущем оно может получить более точные разновидности:

```text
danger(memory)
danger(thread)
danger(device)
danger(interrupt)
```

Или более мягкие свойства/аннотации уровня:

```text
blocking
alloc
slow
```

Важно не перегнуть. Главная идея здесь не в том, чтобы заспамить сигнатуры, а в том, чтобы делать стоимость и риск операций видимыми.

## 10. Diagnostics / debug / check

Такой язык очень естественно просит встроенный системный diagnostics layer:

```scadi
check(condition)
assert(condition)

debug {
    output("frame time: " + text(frame_time))
}
```

Будущий смысл:

- `check` как тестово-валидационная или runtime-проверка;
- `assert` как runtime/debug assertion;
- `debug` как debug-only участок;
- `trace` как диагностический вывод.

На уровне языка и tooling это особенно ценно, потому что Skadi целится в difficult-to-debug domains.

## 11. Data-oriented containers

Помимо `List`, в будущем для системных/игровых сценариев естественно смотрятся:

```text
Ring(T)
Pool(T)
Grid(T)
Matrix(T)
```

Идея:

- `Ring` для cyclic buffers;
- `Pool` для fixed-capacity object sets;
- `Grid` для tile/panel/screen-like дискретных структур;
- `Matrix(T)` для табличных и числовых данных.

Очень важно не смешивать это с `Matrix2D` из visual core:

```text
Matrix2D — пространственная трансформация
Matrix(T) — таблица данных
Grid(T)   — дискретная сетка
```

## 12. Project / target awareness

Для embedded и platform-specific сценариев целевая платформа важна.

Но target knowledge лучше держать:

- в project config;
- в CLI/tooling;
- в build/runtime metadata;

а не в повседневном пользовательском коде.

То есть логика скорее такая:

```text
target awareness belongs to project/tooling first
```

## 13. Static analysis as a core philosophy

Очень сильная мысль документа — Skadi должен анализировать не только типы, но и архитектурные ограничения.

Примеры будущих полезных diagnostics:

```text
error: allocation inside interrupt context
error: blocking receive inside interrupt context
error: value allocated in local Memory escapes function
error: task handle dropped without wait or stop

warning: Channel(LogEntry) may block in render loop
warning: frame_memory peak usage close to capacity
warning: allow grow used in embedded target
warning: hidden allocation in strict policy mode
```

То есть статический анализ здесь — не украшение, а существенная часть идентичности языка.

## 14. No hidden allocation mode

Для строгих embedded/system целей очень естественно смотрится project-level policy:

```text
hidden_allocation = allow
hidden_allocation = warn
hidden_allocation = error
```

Эта идея очень хорошо совпадает с общей философией Skadi:

> стоимость операций должна быть видимой

Особенно это важно для:

- text concatenation;
- image/text drawing paths;
- convenience helpers, которые могут скрывать heap behavior.

## 15. Small standard prelude

Важный принцип против распухания языка:

Prelude должен содержать primitives of systems thinking, а не удобства для любого домена.

В prelude естественно смотрятся:

```text
числа
Bool / Char / Text / Path
Time / Duration
Memory
Task / Channel
Canvas
Color
Vec2 / Vec3 / Vec4
Rect / Size
Matrix2D
basic math
check / test
```

А вот большой web/UI/enterprise/general-purpose стек туда тащить не надо.

## 16. Возможные четыре опорных столпа

После всех этих документов язык всё яснее описывается через 4 большие оси:

```text
Memory        — где и как живут данные
Task/Channel  — как выполняется работа и идут сообщения
Canvas        — как система показывает состояние
Time/Units    — как система существует во времени и физических величинах
```

Всё остальное либо поддерживает эти оси, либо должно оставаться библиотекой/tooling-слоем.

## 17. Что не надо добавлять слишком рано

Документ очень правильно предупреждает о риске распухания.

Слишком рано не стоит тащить в ядро:

- full async/await;
- actor system;
- GC;
- exceptions;
- reflection;
- macro system;
- scene graph;
- big UI layout engine;
- complex units algebra;
- lock-free primitive stack;
- advanced borrow checker;
- plugin runtime.

Если идея полезна, но не базовая для deterministic visual systems, она должна сначала жить вне core.

## 18. Сильные стороны этого направления

Этот слой хорош тем, что он:

- делает язык более честным;
- не уводит его в generic-language territory;
- усиливает embedded/game/operator-panel/tooling идентичность;
- делает видимыми стоимость, риск и контекст операций;
- хорошо стыкуется с уже намеченными memory/task/canvas tracks.

Главная польза:

> Skadi начинает описывать не просто синтаксис программ, а форму системной программы как таковой.

## 19. Главный риск

Главный риск здесь один: раздувание ядра языка.

Поэтому полезный критерий такой:

```text
feature belongs to Skadi core only if it is foundational
for deterministic visual systems
```

Если фича:

- просто удобна;
- дублирует библиотечный уровень;
- даёт вторую форму записи без сильной новой ценности;
- скрывает стоимость;

то её лучше не включать в core.

## 20. Рекомендуемый порядок дальнейшего проектирования

Если выбирать, что из этого обсуждать после memory/task/canvas, разумный порядок такой:

1. `Time / Duration / units`
2. `Resource lifecycle`
3. `Interrupt context rules`
4. `Project tooling / target policy`
5. `Static diagnostics / policy analysis`
6. `Data-oriented containers`
7. `Device abstractions`

Именно в таком порядке эти идеи лучше всего усиливают уже существующую архитектурную основу.

## 21. Итоговая оценка

Как future-track это очень полезный слой документации.

Он важен не потому, что всё это надо немедленно кодить, а потому, что он помогает удержать направление языка:

- не в сторону “ещё один общий язык”;
- а в сторону системного языка для deterministic visual systems.

Главная рекомендация:

> все эти идеи надо воспринимать не как список срочных фич, а как фильтр будущих решений: что действительно усиливает Skadi, а что только раздувает его.
