# Skadi Systems Additions MVP Contract (RU)

Дата: 2026-07-19
Статус: living implementation reference; Memory, Task/Channel и первый
Time/Duration slice реализованы как experimental `v1.2` runtime MVP.
Назначение: зафиксировать ближайший practical-first контракт для набора системных дополнений к Skadi, которые логично развивают оси `Memory`, `Task/Channel`, `Canvas`.

Связанный design-документ:

- [Systems Additions Draft](systems-additions-draft.md)

## 1. Назначение

Этот документ не расширяет stable `v1.1` scope и не объявляет перечисленные ниже
типы реализованными в `v1.2`.

Он нужен, чтобы заранее определить:

- какие системные дополнения действительно выглядят фундаментальными для Skadi;
- что из них ближе всего к реальному MVP;
- что должно остаться future-track, а не разрастись в ядро прямо сейчас.

## 2. Identity

Этот future-track не про "много мелких удобств".

Его задача:

```text
make system cost, context, time, and resource rules visible
```

По-русски:

```text
сделать видимыми время, стоимость, контекст выполнения и жизненный цикл ресурсов
```

## 3. Что считать ближайшим MVP-ядром

Из всего набора идей наиболее реалистичный ближайший implementation-facing минимум такой:

```text
Time / Duration
basic unit literals for time/memory/angle
resource lifecycle rules
interrupt-context restrictions
small diagnostics/debug/check surface
project-level policy hooks
```

То есть не весь document cluster целиком, а только самый фундаментальный системный слой.

## 4. Priority order

Рекомендуемый порядок будущего проектирования и реализации:

1. `Time / Duration / units`
2. `Resource lifecycle`
3. `Interrupt context rules`
4. `Project tooling / policy layer`
5. `Static diagnostics for architectural rules`
6. `Data-oriented containers`
7. `Device abstractions`

Это важнее и безопаснее, чем сразу браться за весь широкий стек идей.

## 5. Time / Duration contract

Реализационный статус: первый bounded slice выполнен. Поддерживаются nominal
`Time/Duration`, integer `ms/s/min`, monotonic `now/elapsed` и blocking
`sleep/delay` на Win32/POSIX. Расширения этого раздела остаются future work.

Если этот слой пойдёт в реализацию, минимальный контракт должен быть таким:

### Required types / concepts

```text
Time
Duration
```

### Required literals

```text
ms
s
min
```

### Required operations

```text
now()
elapsed(Time)
delay(Duration)
sleep(Duration)
```

### Design rule

Временные значения должны быть семантически явными:

```scadi
delay(500ms)
sleep(1s)
```

а не завязаны на голые числа без единиц.

## 6. Units contract

Для первого среза units layer должен быть очень маленьким.

### Required early units

```text
time: ms, s, min
memory: b, kb, mb
angle: deg, rad
```

### Optional near-future unit

```text
Hz
```

### Out of early scope

```text
full physical units algebra
generic dimensional analysis system
large family of engineering units
```

## 7. Resource lifecycle contract

После memory model это почти обязательная ось.

Базовый контракт:

```text
resources close at scope end
```

Под ресурсами понимаются как минимум:

- files;
- ports;
- windows;
- displays;
- sensors;
- network/serial handles.

Skadi не должен получиться языком, где память управляется аккуратно, а системные ресурсы текут хаотично.

## 8. Interrupt-context contract

Если interrupt/event bridge когда-нибудь идёт в реализацию, минимальные semantic restrictions должны быть жёсткими.

В interrupt context:

```text
allocation = forbidden
blocking = forbidden
slow/general I/O = forbidden
non-interrupt-safe calls = forbidden
```

Это должно быть частью языка/semantic analysis, а не просто советом в docs.

## 9. Execution-context contract

Полный context system пока не обязателен, но future contract полезно зафиксировать заранее.

Ближайшие meaningful contexts:

```text
normal
task
interrupt
render
```

Для раннего этапа достаточно:

- хотя бы interrupt-context diagnostics;
- возможно later warnings для render-blocking behavior.

То есть context model можно вводить постепенно, а не сразу как большой отдельный subsystem.

## 10. Diagnostics / debug contract

Ближайший полезный слой:

```text
check
assert
debug { ... }
```

Это нужно не как "сахар", а как инструмент для difficult-to-debug domains.

Целевая философия:

```text
Skadi diagnostics should explain architectural mistakes, not only type mistakes
```

## 11. Policy-layer contract

Очень перспективная идея — project-level policy knobs.

Наиболее естественный ранний кандидат:

```text
hidden_allocation = allow | warn | error
```

Это особенно хорошо сочетается с:

- memory model;
- text / canvas future operations;
- embedded targets;
- strict deterministic project profiles.

## 12. Device abstraction contract

Device abstractions важны, но не должны быть ранним giant scope.

Правильный contract:

```text
core knows common device categories
platform provides concrete implementations
```

То есть:

- `gpio`, `i2c`, `spi`, `uart`, `display`, `timer` могут быть vocabulary-layer;
- конкретные драйверы и board details не должны жить в ядре языка.

## 13. Data-oriented containers contract

Это хороший future-track, но не ближайший core priority.

Кандидаты:

```text
Ring(T)
Pool(T)
Grid(T)
Matrix(T)
```

Полезно заранее зафиксировать только две вещи:

1. Они должны решать реальные systems/game/embedded задачи.
2. Их нельзя путать с visual/math constructs вроде `Matrix2D`.

## 14. Prelude contract

Prelude должен оставаться маленьким.

Туда естественно попадают:

- системные примитивы;
- время;
- память;
- задачи/каналы;
- базовая визуальная геометрия;
- минимальные diagnostics helpers.

Туда не должны незаметно попадать:

- web stacks;
- database frameworks;
- enterprise utils;
- большие runtime subsystems.

## 15. What is explicitly out of scope

Сюда пока не надо тащить:

```text
full async/await
actor runtime
GC
exceptions
reflection
macro system
complex units algebra
lock-free primitive stack
advanced borrow checker
big plugin/runtime framework
```

Если какая-то идея полезна, но не фундаментальна для deterministic visual systems, она должна сначала жить вне core.

## 16. Recommended implementation discipline

Если этот future-track пойдёт в реальную работу, дисциплина должна быть такой:

1. Сначала vocabulary and semantics.
2. Потом очень маленький проверяемый subset.
3. Потом diagnostics.
4. Только потом расширения и convenience layers.

Важно не допустить, чтобы systems additions превратились в неструктурированный список разрозненных фич.

## 17. Итог

Этот future-track полезен как design filter.

Он помогает отвечать на вопрос:

```text
усиливает ли новая идея системную идентичность Skadi
или просто раздувает язык?
```

Именно в этом его главная ценность для следующих версий после `v1.1`.
