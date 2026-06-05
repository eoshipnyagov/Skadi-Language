# Skadi Visual Core MVP Contract (RU)

Дата: 2026-06-04
Статус: MVP contract / future implementation reference
Назначение: зафиксировать минимальный practical-first контракт для будущего visual-core слоя Skadi, без расползания в большой graphics framework.

Связанный design-документ:

- [Visual Core Draft](visual-core-draft.md)

## 1. Назначение

Этот документ не обещает немедленную реализацию в `v1.1`.

Он нужен, чтобы заранее зафиксировать:

- что вообще считается Visual Core в Skadi;
- что могло бы войти в первый рабочий срез;
- чего сознательно не надо обещать слишком рано;
- как этот трек должен стыковаться с памятью, задачами и backend story.

## 2. MVP identity

Visual Core MVP для Skadi — это:

```text
Canvas-first, immediate-mode, backend-agnostic 2D drawing core
```

По-русски:

```text
небольшое immediate-mode 2D-ядро рисования,
где Canvas — центральная абстракция,
а конкретные окна/дисплеи/backend-реализации живут отдельно.
```

## 3. Что входит в MVP

Минимально допустимый состав:

```text
Vec2
Rect
Color
Canvas
Image или framebuffer-like surface
clear
pixel
line
rect
fill_rect
text
present
```

Допустимое расширение первого среза, если реализация получается чистой:

```text
Size
Matrix2D
circle
fill_circle
image
push_transform / pop_transform
```

## 4. Public model

Будущий пользовательский код должен концептуально сводиться к такому паттерну:

```scadi
canvas.clear(Color.black)
draw_world(canvas, world)
draw_ui(canvas, ui)
window.present()
```

Ключевой смысл:

- `Canvas` рисует;
- backend показывает;
- код отрисовки не зависит от платформы напрямую.

## 5. Core invariants

Если Visual Core будет реализовываться, базовый контракт должен быть таким:

1. `Canvas` — это абстракция рисования, а не UI framework и не game engine.
2. Visual Core остаётся immediate-mode по умолчанию.
3. Backend-объекты вроде `window`, `screen`, `framebuffer` создают/дают `canvas`, но не ломают общую mental model.
4. Drawing API должен быть маленьким и цельным, а не большим списком “на всякий случай”.
5. Базовые drawing operations не должны скрывать тяжёлую и неожиданную runtime-магии.

## 6. Memory contract

Visual Core должен уважать memory model Skadi.

Практический MVP-контракт:

1. Базовые операции вроде `pixel`, `line`, `rect`, `fill_rect`, `clear` не должны требовать скрытых динамических аллокаций как нормы.
2. Если операция рисования требует временной памяти, это должно быть:

   - либо явно документировано;
   - либо позже увязано с `Memory`/`place in`.
3. Frame-oriented visual-data сценарии считаются естественными для Skadi.

Рекомендуемая целевая философия:

```text
temporary drawing data should compose naturally with frame_memory
```

## 7. Task / channel contract

Visual Core должен естественно сочетаться с task model, но не зависеть от неё как от обязательного runtime-слоя.

Практический MVP-контракт:

1. `Canvas` сам по себе не является concurrency-framework объектом.
2. Visual state может подаваться через `Channel`.
3. Главный loop может рисовать, пока отдельные задачи собирают данные/события.
4. Shared mutable state не должен быть default story для visual update path.

## 8. Backend contract

Для первого рабочего среза backend story должна быть очень консервативной.

Разрешённая стратегия:

- software framebuffer;
- render в PPM/BMP;
- очень простой desktop/window backend;
- embedded-style surface abstraction.

Нежелательная стратегия для первого среза:

- сразу строить сложный GPU backend;
- тащить OpenGL/Vulkan/DirectX как обязательную базу;
- делать visual MVP зависимым от тяжёлого platform stack.

## 9. C/backend implementation direction

Если этот трек когда-нибудь пойдёт в текущий `Skadi -> C` backend, практичный старт может выглядеть так:

```text
Canvas  -> software framebuffer abstraction
Color   -> rgba-like struct
Vec2    -> struct {float x, y}
Rect    -> struct {float x, y, w, h}
Image   -> pixel buffer
present -> file dump or minimal host presentation layer
```

То есть цель не “написать сразу renderer мечты”, а показать, что визуальный код Skadi уже можно выразить чисто и предсказуемо.

## 10. What is out of scope for MVP

Осознанно не входит:

```text
scene graph
retained UI
layout engine
asset manager
shader language
GPU batching
materials
animation stack
camera system
full font shaping
SVG
full image codec story
OpenGL/Vulkan/DirectX as required baseline
```

## 11. Compiler / semantic implications

Для будущей реализации нужно будет обеспечить хотя бы такие вещи:

1. Чёткие type contracts для `Canvas`, `Color`, `Vec2`, `Rect`, `Image`, `Matrix2D`.
2. Проверку сигнатур drawing operations.
3. Понятную lowering-story для Canvas API.
4. Согласованность с memory rules.
5. Согласованность с task/channel model там, где visual data передаётся между задачами.

## 12. Recommended implementation order

Если Visual Core когда-нибудь перейдёт из future-track в практическую реализацию, разумный порядок такой:

1. Зафиксировать vocabulary и public model.
2. Выбрать маленький `Canvas v0`.
3. Поднять software/backend-first demonstrator.
4. Только потом расширять shapes/transform/text features.
5. Ещё позже обсуждать richer backends и более сложный visual stack.

## 13. Итог

Visual Core не должен считаться частью текущего `v1.1` scope.

Но как future-track для Skadi это важный архитектурный контракт:

- он хорошо стыкуется с memory model;
- он хорошо стыкуется с task model;
- он усиливает прикладную ценность языка;
- он даёт очень наглядное направление для `v2+` и отдельных MWP-демонстраторов.
