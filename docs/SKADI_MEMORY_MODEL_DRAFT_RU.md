# Skadi Memory Model (Draft RU)

Дата: 2026-06-04
Статус: draft / design reference
Назначение: зафиксировать опорную модель памяти Skadi до полноценной реализации syntax/runtime/backend.

Связанные рабочие документы:

- [Memory MVP Contract](memory-model-mvp.md)
- [Memory Examples and Negative Cases](memory-model-examples.md)

Если нужен минимальный набор current-примеров без скрытого контекста и без broad-design допущений, сначала стоит читать examples-документ.

## 1. Зачем Skadi отдельная memory model

Skadi не стремится повторить модель `C/C++` с постоянным ручным `free`, но и не хочет скрывать память за garbage collector.

Цель модели:

> предсказуемая память без GC и без постоянного ручного освобождения каждого объекта.

Это особенно важно для сценариев, ради которых язык выглядит естественным:

- игры;
- embedded;
- firmware;
- realtime-ish системы;
- симуляции;
- runtime/tooling-код, где lifetime данных должен быть читаем из структуры программы.

Идея модели не в том, чтобы дать "максимум магии", а в том, чтобы сделать lifetime архитектурной частью программы.

## 2. Главная формула модели

Короткая версия:

```text
Scope owns temporary values.
Return transfers ownership.
Memory owns groups of values.
place in selects allocation memory.
clear destroys a whole memory group.
allow marks controlled relaxation.
```

По-русски:

```text
Scope владеет временным.
return передаёт владение.
Memory владеет группой.
place in выбирает место размещения.
clear уничтожает группу.
allow явно разрешает послабление.
```

## 3. Базовое правило: scope-память по умолчанию

По умолчанию всё, что создаётся внутри блока, живёт до конца этого блока.

```scadi
fn process() {
    new buffer = List(u8)
    new text = Text("hello")

    // buffer и text доступны здесь
}

// buffer и text уничтожены здесь
```

Базовое правило:

```text
Создано внутри scope -> уничтожено при выходе из scope.
```

Плюсы:

- не нужен `free` в обычном коде;
- не нужен GC;
- lifetime читается из кода;
- легко понимать, где данные умирают.

## 4. `return` передаёт владение наружу

Если значение явно возвращается из функции, оно не уничтожается вместе с локальным scope.

```scadi
fn make_numbers() returns Int List {
    new Int List numbers = []

    numbers.push(1)
    numbers.push(2)
    numbers.push(3)

    return numbers
}
```

Смысл:

```text
numbers создан внутри функции,
но return передаёт владение вызывающему коду.
```

Пример:

```scadi
fn main() {
    new values = make_numbers()

    // values теперь владеет списком
}
```

Это позволяет не тащить регионы и специальные memory-механизмы в каждый обычный сценарий.

## 5. Вложенные динамические данные принадлежат владельцу

Если структура содержит динамические поля, они принадлежат самой структуре.

```scadi
struct LevelData {
    Text source
}

fn load_level(Path path) returns LevelData {
    new Text file_text = read(path)
    new LevelData result = {source = file_text}
    return result
}
```

Смысл:

```text
Level владеет entities и textures.
Когда уничтожается Level, уничтожаются и его внутренние динамические данные.
```

Это даёт простую ownership-tree модель: программист думает не о каждом `malloc`, а о владельцах данных.

## 6. `Memory` как явный регион памяти

Для более сложных сценариев вводится отдельная сущность `Memory`.

```scadi
Memory level_memory = memory(256mb)
Memory frame_memory = memory(8mb)
Memory cache_memory = memory(64mb)
```

`Memory` — это явная управляемая область памяти.

Она нужна, когда данные должны:

- жить как группа;
- уничтожаться все вместе;
- быть ограничены по размеру;
- не фрагментировать общую память;
- иметь явную политику выделения.

Это делает память не скрытым runtime-эффектом, а явным ресурсом программы.

## 7. `place in` — явное размещение в конкретной памяти

Если нужно создавать динамические данные внутри выбранной `Memory`, используется `place in`.

```scadi
place in level_memory {
    new mesh = Mesh(...)
    new texture = Texture(...)
}
```

Смысл:

```text
Все динамические данные, созданные внутри блока,
размещаются в указанной Memory.
```

`place in` отвечает за место размещения, а не за передачу владения.

## 8. `return` и `place in` решают разные задачи

Важно не смешивать их роли:

```text
return отвечает за передачу владения.
place in отвечает за место размещения.
```

Простой случай:

```scadi
fn make_list() returns Int List {
    new Int List values = []
    return values
}
```

Региональный случай:

```scadi
struct LoadedText {
    Text content
}

fn load_text(Memory assets_memory, Path path) returns LoadedText {
    place in assets_memory {
        new Text file_text = read(path)
        new LoadedText result = {content = file_text}
        return result
    }
}
```

Региональная память не должна навязываться всему языку. Она существует для тех случаев, где обычного scope-lifetime уже недостаточно.

## 9. Размер `Memory` задаётся явно

Базовый вид:

```scadi
Memory level_memory = memory(256mb)
Memory frame_memory = memory(8mb)
Memory sensor_memory = memory(4kb)
```

Смысл:

- размер памяти виден прямо в коде;
- memory-budget становится частью архитектуры;
- поведение лучше подходит для embedded и игр.

## 10. Ошибка при создании `Memory`

Создание памяти может закончиться ошибкой.

```scadi
Memory level_memory = memory(512mb) on error {
    return LoadStatus.OutOfMemory
}
```

Недостаток памяти должен считаться обычным ожидаемым состоянием, а не скрытой катастрофой.

Skadi не должен молча падать или запускать скрытый runtime recovery.

## 11. Ошибка при переполнении `Memory`

Если внутри выбранной памяти не осталось места, аллокации внутри `place in` должны попадать в общий `on error`.

```scadi
place in level_memory {
    new texture = load_texture(Path("huge_texture.png"))
    new mesh = load_mesh(Path("level.mesh"))
} on error {
    level_memory.clear()
    return LoadStatus.OutOfMemory
}
```

Смысл:

```text
Любая нехватка памяти внутри блока попадает в общий on error.
Автоматический rollback уже созданного внутри блока не гарантируется.
Если нужен чистый регион, это делается явно через Memory.clear().
```

Это лучше, чем требовать `on error` после каждой отдельной аллокации.

## 12. `Memory` не растёт по умолчанию

По умолчанию:

```scadi
Memory level_memory = memory(256mb)
```

означает:

```text
ровно 256 MB, не больше
```

Если память закончилась — это ошибка.

Это хорошо для детерминированности: программа не начинает внезапно потреблять больше памяти, чем ожидалось.

## 13. `allow grow` — явное разрешение роста

Если региону разрешено расширяться, это должно быть явно указано.

```scadi
Memory editor_memory = memory(64mb, allow grow)
Memory tool_memory = memory(128mb, allow grow)
```

Смысл:

```text
Эта память может попытаться расшириться,
если свободного места больше не хватает.
```

Хорошо подходит для:

- редакторов;
- desktop-инструментов;
- debug-сборок;
- нестрогих runtime-сценариев.

Для embedded и строго детерминированного runtime фиксированный размер предпочтительнее.

## 14. `allow drop` — разрешение на сброс некритичных данных

Для кэшей, логов и вторичных данных может существовать политика `allow drop`.

```scadi
Memory cache_memory = memory(64mb, allow drop)
```

или на уровне конкретных данных:

```scadi
allow drop List(LogEntry) logs
```

Смысл:

```text
Эти данные не критичны.
Система имеет право выбросить их при нехватке памяти.
```

Это превращает memory pressure в явно описанную политику.

## 15. `allow` как единый маркер ослабления строгих правил

Хорошая линия дизайна:

```scadi
allow grow
allow drop
```

`allow` означает:

```text
Я явно разрешаю runtime отступить от строгой детерминированности
в указанном направлении.
```

Примеры:

```scadi
Memory cache = memory(64mb, allow drop)
Memory editor = memory(128mb, allow grow)
Memory temp = memory(32mb, allow drop, allow grow)
```

Это должна быть единая семантическая модель, а не набор разрозненных исключений.

## 16. Static memory для embedded

Для embedded-сценариев полезна статическая память:

```scadi
fixed Memory sensor_memory = memory.static(8kb)
```

Смысл:

```text
Память резервируется статически,
а не берётся из heap во время выполнения.
```

Плюсы:

- нет runtime allocation failure на старте;
- размер известен заранее;
- легче контролировать RAM;
- легче анализировать поведение прошивки.

## 17. Дочерние `Memory`

Возможен более продвинутый механизм дочерних регионов:

```scadi
Memory app_memory = memory(64mb)

place in app_memory {
    Memory level_memory = memory.child(32mb)
    Memory frame_memory = memory.child(8mb)
}
```

Смысл:

```text
Дочерние регионы выделяются из родительской памяти.
```

Это даёт дерево памяти:

```text
app_memory
|- level_memory
|- frame_memory
`- cache_memory
```

Такой стиль особенно полезен для игр и runtime-систем с явным memory budget.

## 18. Очистка `Memory`

`Memory` может умирать автоматически при выходе из scope:

```scadi
fn run_level() {
    Memory level_memory = memory(256mb)

    place in level_memory {
        new level = load_level_data()
        game_loop(level)
    }
}
```

Также возможна явная очистка:

```scadi
frame_memory.clear()
```

Типичный frame allocator:

```scadi
Memory frame_memory = memory(16mb)

loop {
    place in frame_memory {
        update_ui()
        build_frame_commands()
        simulate_particles()
    }

    render()
    frame_memory.clear()
}
```

Это естественная модель для игр: всё временное за кадр умирает одним действием.

## 19. Главный safety rule

Главная опасность региональной памяти — вернуть значение, чьи динамические данные пережили свою `Memory`.

Плохо:

```scadi
struct LoadedText {
    Text content
}

fn bad() returns LoadedText {
    Memory temp_memory = memory(4mb)

    place in temp_memory {
        new Text file_text = read(Path("asset.txt"))
        new LoadedText result = {content = file_text}
        return result
    }
}
```

Такой код должен быть запрещён компилятором.

Правило:

```text
Нельзя вернуть значение, если его динамические данные размещены в Memory,
которая умрёт раньше возвращаемого значения.
```

Это центральный safety-инвариант всей модели.

## 20. Упрощённое правило для MVP

Для первого реализуемого демонстратора можно взять упрощённый вариант:

```text
Значения, созданные в place in memory,
можно возвращать только если эта memory была передана в функцию извне.
```

Разрешено:

```scadi
struct LoadedText {
    Text content
}

fn load_text(Memory assets_memory, Path path) returns LoadedText {
    place in assets_memory {
        new Text file_text = read(path)
        new LoadedText result = {content = file_text}
        return result
    }
}
```

Запрещено:

```scadi
struct LoadedText {
    Text content
}

fn load_text(Path path) returns LoadedText {
    Memory temp_memory = memory(4mb)

    place in temp_memory {
        new Text file_text = read(path)
        new LoadedText result = {content = file_text}
        return result
    }
}
```

Это простое правило:

- легко объяснить;
- относительно легко реализовать;
- уже даёт реальную защиту от dangling-region ошибок.

## 21. Минимальная модель для первого рабочего среза

Для первого работающего демонстратора достаточно следующего:

```text
1. Локальные значения живут до конца scope.
2. return передаёт владение наружу.
3. Memory создаётся явно: memory(size).
4. place in Memory размещает динамические данные в выбранной памяти.
5. Memory имеет fixed capacity по умолчанию.
6. Если места нет — это on error.
7. Memory.clear() уничтожает всё содержимое региона.
8. allow grow и allow drop существуют как явные политики.
```

Для MVP не обязательны:

- полноценный borrow checker;
- сложные lifetimes;
- generational regions;
- compacting memory;
- GC;
- ref counting;
- сложный escape analysis.

## 22. Примеры

### 22.1. Загрузка уровня

```scadi
state LoadStatus {
    Ok
    FileError
    ParseError
    OutOfMemory
}

struct Level {
    List(Entity) entities
    List(Texture) textures
    NavMesh navmesh
}

struct LevelLoadResult {
    Level level
    LoadStatus status
}

fn load_level(Memory level_memory, Path path) returns LevelLoadResult {
    new file = fs.read(path) on error {
        return {level = Level.empty(), status = LoadStatus.FileError}
    }

    new tokens = parse_level(file) on error {
        return {level = Level.empty(), status = LoadStatus.ParseError}
    }

    place in level_memory {
        new level_value = Level {
            entities = List(Entity),
            textures = List(Texture),
            navmesh = NavMesh()
        }

        for token in tokens {
            level_value.entities.append(make_entity(token))
        }

        return {level = level_value, status = LoadStatus.Ok}
    } on error {
        level_memory.clear()
        return {level = Level.empty(), status = LoadStatus.OutOfMemory}
    }
}
```

Почему это хорошо:

- временные данные парсинга живут локально;
- данные уровня живут в `level_memory`;
- нехватка памяти обрабатывается явно;
- уровень можно удалить одним `clear`.

### 22.2. Память кадра

```scadi
fn game_loop(Level level) {
    Memory frame_memory = memory(16mb) on error {
        output("Cannot allocate frame memory")
        return
    }

    loop {
        place in frame_memory {
            new visible_entities = collect_visible(level.entities)
            new draw_commands = build_draw_commands(visible_entities)
            new ui_commands = build_ui()

            render(draw_commands, ui_commands)
        } on error {
            frame_memory.clear()
            output("Frame memory overflow")
            continue
        }

        frame_memory.clear()
    }
}
```

Это естественная game-dev модель:

```text
всё временное за кадр умерло после кадра
```

### 22.3. Embedded sensor buffer

```scadi
fn sensor_main() {
    fixed Memory sensor_memory = memory.static(4kb)

    loop {
        sensor_memory.clear()

        place in sensor_memory {
            new samples = List(Sample)

            for i in 0..128 {
                samples.append(read_sample())
            }

            process_samples(samples)
        } on error {
            sensor_memory.clear()
            output("Sensor memory overflow")
            continue
        }

        sleep(10ms)
    }
}
```

Плюсы:

- память фиксирована;
- размер известен заранее;
- нет heap-сюрпризов;
- поведение при переполнении явно задано.

## 23. Compiler rules / semantic rules

Если эта модель принимается как опорная, компилятор в перспективе должен уметь проверять хотя бы следующие инварианты:

1. Значение нельзя вернуть, если оно зависит от умершей `Memory`.
2. Значение нельзя сохранить в более долгоживущий owner, если его dynamic payload живёт в более короткой `Memory`.
3. `place in` должно менять не lifetime значения как такового, а место размещения его динамических данных.
4. `Memory.clear()` должно считаться точкой уничтожения всех объектов, размещённых в этом регионе.
5. `allow grow` и `allow drop` — это не синтаксический шум, а реальные runtime-policy флаги.

## 24. Что пока не надо обещать

На этом этапе лучше не обещать слишком рано:

- полноценный borrow checker;
- общий lifetime calculus в стиле Rust;
- автоматический deep escape analysis для любых случаев;
- сложные правила aliasing и partial borrows;
- transparent move semantics для всех структурных случаев;
- сильную интеграцию с concurrency/runtime model до фиксации базовой memory model.

Иначе дизайн быстро станет красивым в тексте, но тяжёлым для реализации и объяснения.

## 25. Сильные стороны модели

У этой модели очень хорошие свойства для Skadi:

- читаемость;
- явность memory budget;
- отсутствие скрытого GC;
- отсутствие обязательного ручного `free`;
- естественная применимость к games/embedded/tooling;
- понятная архитектурная граница между "обычным кодом" и "region-oriented кодом".

Главное достоинство:

> lifetime становится не математической головоломкой, а видимой частью структуры программы.

## 26. Риски и точки аккуратности

При этом есть несколько мест, где нужно быть осторожными.

### 26.1. `allow drop` легко сделать слишком магическим

Если `allow drop` будет означать "runtime может внезапно убрать мои данные когда угодно", модель станет трудно предсказуемой.

Практичнее трактовать его как:

- право runtime/cache-layer сбрасывать данные только в явно определённых safe points;
- или как политику для специальных типов/контейнеров, а не для любого объекта подряд.

### 26.2. `allow grow` ослабляет детерминированность

Это нормально, если ослабление видно в коде. Но важно, чтобы:

- fixed-size memory была поведением по умолчанию;
- `allow grow` не размывал embedded/game identity языка.

### 26.3. Возврат из `place in` — самый опасный участок

Именно здесь модель либо останется читаемой, либо быстро превратится в "почти Rust, но без формализма Rust".

Поэтому MVP-ограничение "возвращать можно только из memory, переданной извне" выглядит очень разумным.

### 26.4. Нужно заранее определить, что именно "живёт в Memory"

Нужен чёткий ответ на вопрос:

- в `Memory` размещается только dynamic payload (`List`, `Text`, heap-поля структур)?
- или весь объект целиком?

Для MVP я бы рекомендовал придерживаться более простой формулы:

```text
Memory управляет размещением динамических данных и region-owned объектов,
а не всей возможной value-semantics языка сразу.
```

## 27. Рекомендуемый путь внедрения

Если двигаться practically-first, то хороший порядок такой:

1. Зафиксировать терминологию:

   - scope ownership
   - return transfer
   - Memory region
   - place in
   - clear
   - allow grow / allow drop
2. Принять MVP safety rule для возврата из `place in`.
3. Реализовать только fixed-capacity `Memory` + `clear` + `on error`.
4. Считать `allow grow` и `allow drop` пока design-level flags, даже если runtime initially частично stubbed.
5. Только после этого переходить к child memory и более сложным policy cases.

## 28. Итоговая оценка

Как опорная архитектурная модель для Skadi это очень сильная идея.

Почему я считаю её хорошей:

- она соответствует духу языка;
- она не прячет важное;
- она даёт детерминированность по умолчанию;
- она расширяется от простого scope-lifetime к region-based сценарию без полного разрыва модели;
- она достаточно практична для игр и embedded;
- она позволяет начать с MVP, не обещая сразу невозможную сложность.

Главная рекомендация:

> не пытаться сделать сразу "идеальную общую memory theory", а закрепить простой и жёсткий MVP-контракт, который уже полезен и уже проверяем компилятором.

В таком виде memory model выглядит не просто хорошей, а очень перспективной именно для Skadi.
