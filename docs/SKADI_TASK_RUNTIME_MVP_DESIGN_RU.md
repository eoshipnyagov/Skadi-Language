# Skadi v1.2: Task Runtime MVP Design

Дата: 2026-07-12
Статус: approved design target / implementation contract.

## 1. Назначение

Этот документ определяет минимальный исполняемый backend для уже реализованного
Task/Channel frontend. Он является мостом между языковым контрактом
[Task Model MVP](task-model-mvp.md) и generated C runtime.

Цель первого backend slice:

```text
run -> independent work -> wait
run -> stop request -> stopping -> wait
Task(T) -> result transfer through wait
```

Channel runtime следует отдельным вторым slice, но его ABI и lifecycle границы
учитываются здесь заранее.

## 2. Границы MVP

В runtime MVP входят:

- одна native worker execution unit на одну `Task`;
- `run`, `wait`, `Task(T)`, `stop`, `stopping`;
- Win32 backend для Windows;
- pthread backend для Linux и других поддерживаемых POSIX host targets;
- thread-local current-task и active-memory contexts;
- bounded `Channel(T)` с blocking `send/receive` во втором slice;
- compile-time запреты для неподдержанных ownership форм;
- process-fatal runtime diagnostics для внутренних ошибок, которые текущий
  синтаксис ещё не умеет обработать через `on error`.

Не входят:

- thread pool или work-stealing scheduler;
- async/await, futures и coroutines;
- detached tasks;
- task groups и structured-concurrency sugar;
- hard kill;
- `select`, timeouts, non-blocking channel operations и channel close;
- shared mutable values, mutex/atomic API на уровне языка;
- RTOS и bare-metal backend;
- настройка affinity, priority и stack size.

### 2.1. Сознательно отложенные design decisions

Следующие механизмы не требуются для `v1.2` и не должны задерживать Task/Channel
runtime MVP:

- явный `detach` или другой fire-and-forget lifecycle;
- `channel.close` и семантика чтения из закрытого канала;
- отменяемый или прерываемый `receive`;
- связь `stop` с автоматическим пробуждением channel operations.

Это не окончательный отказ от возможностей. Решения по ним принимаются после
появления работающих `run -> wait`, `stop -> wait` и bounded `send/receive`, когда
поведение можно оценить на showcases и stress tests. До этого действуют строгие
правила:

- detached tasks отсутствуют;
- каждый `run` имеет owning handle и завершается через `wait`;
- Channel не закрывается неявно;
- завершение consumer обеспечивается протоколом сообщений;
- `stop` не прерывает блокирующую channel operation.

## 3. Модель исполнения

Для desktop MVP одна Skadi Task отображается на один native thread.

```text
Skadi run
    -> generated task context
    -> sk_task_start(...)
    -> Win32 thread или pthread
    -> generated worker trampoline
    -> Skadi function body
```

Это backend detail, а не обещание языка. Публичная семантика остаётся Task-based,
поэтому позднее runtime может перейти на pool, RTOS task или cooperative scheduler
без изменения canonical syntax.

## 4. Состояния и lifecycle

Минимальная state machine:

```text
Created -> Running -> Completed -> Joined
                    -> Failed    -> Joined

Running --stop--> StopRequested -> Completed -> Joined
```

Правила:

- `run` возвращает ровно один owning handle;
- handle нельзя копировать, переприсваивать, хранить в struct/list или возвращать;
- `stop` не потребляет handle и может быть вызван не более одного раза;
- `wait` выполняется ровно один раз и потребляет runtime resource handle;
- после `wait` native thread joined, task context освобождён;
- проигнорированный результат `run` является semantic error уже в текущем
  frontend foundation;
- frontend должен доказать `wait` handle на всех путях до выхода из owning scope;
- если all-path lifecycle доказать нельзя, программа отклоняется;
- backend не делает detach и не содержит скрытый process registry;
- завершение процесса не считается корректной заменой `wait`.

`wait Task(T)` копирует result value из task context в вызывающий scope, после
чего уничтожает runtime context. Семантически это move result к ожидающей стороне.

## 5. Runtime ABI generated C

Backend генерирует общий platform-neutral слой приблизительно следующей формы:

```c
typedef enum {
    SK_TASK_CREATED,
    SK_TASK_RUNNING,
    SK_TASK_STOP_REQUESTED,
    SK_TASK_COMPLETED,
    SK_TASK_FAILED,
    SK_TASK_JOINED
} SkTaskState;

typedef struct SkTask SkTask;
typedef void (*SkTaskEntry)(SkTask *task, void *context);

struct SkTask {
    SkPlatformThread thread;
    SkPlatformMutex lock;
    SkTaskState state;
    bool stop_requested;
    int runtime_error;
    void *context;
};
```

Обязательные внутренние операции:

```c
bool sk_task_start(SkTask *task, SkTaskEntry entry, void *context);
void sk_task_request_stop(SkTask *task);
bool sk_task_is_stopping(void);
bool sk_task_join(SkTask *task);
void sk_task_destroy(SkTask *task);
```

Точные поля являются implementation detail. Стабильный контракт задают lifecycle,
visibility и cleanup rules, а не бинарная совместимость generated C между версиями.

## 6. Generated context и trampoline

Для каждого task entry backend генерирует typed context:

```c
typedef struct {
    InputType input;
    ResultType result;
} SkTaskContext_load_data;
```

`run load_data(value)` lowering:

1. вычисляет аргументы в вызывающем thread;
2. создаёт task handle и context на обычном runtime heap;
3. копирует value-safe аргументы в context;
4. запускает generated trampoline;
5. возвращает owning `Task(ResultType)` handle.

Trampoline:

1. устанавливает thread-local current task;
2. устанавливает thread-local active Memory в `NULL`;
3. вызывает Skadi function;
4. сохраняет result в context;
5. переводит состояние в `Completed`;
6. очищает thread-local current task перед выходом.

Если native thread создать нельзя, `run` печатает coded runtime diagnostic и
завершает процесс с ненулевым кодом. Recoverable `run ... on error` требует
отдельного языкового решения и не добавляется скрыто в MVP.

## 7. `stop` и `stopping`

`stop task` атомарно публикует cooperative stop request. Он:

- не прерывает инструкцию;
- не освобождает stack или context;
- не выполняет cleanup от имени worker;
- не заменяет последующий `wait`.

`stopping` читает stop flag текущей task через thread-local current-task pointer.
Вне task context это уже запрещается semantic layer.

Visibility stop flag обеспечивается platform synchronization primitive или C11
atomic с acquire/release semantics. `volatile` без синхронизации недостаточен.

MVP не обещает, что `stop` прервёт блокирующий файловый I/O или channel operation.
Программа обязана строить протокол завершения так, чтобы worker снова дошёл до
проверки `stopping`. Channel cancellation/close является отдельным будущим контрактом.

## 8. Аргументы и результаты

Через task boundary разрешаются только task-safe values:

- scalar numeric values, `Bool`, `Char`;
- value-like structs, чьи поля также task-safe;
- `Text`/`Path`, если payload не принадлежит локальной `Memory` и рассматривается
  как immutable на время работы task;
- `Channel(T)` как специальный shared capability во втором backend slice.

Запрещаются:

- `Memory` handle;
- `Task` handle;
- `List` до появления определённой move/deep-copy семантики;
- значения с dynamic payload из локальной или очищаемой `Memory`;
- struct/list, рекурсивно содержащие capability или region-owned payload;
- mutable alias, доступный одновременно caller и worker.

До включения backend semantic layer должен проверять эти правила отдельно для
аргументов `run` и результата `Task(T)`. Простого совпадения типов недостаточно.

Аргументы вычисляются до запуска thread. Это фиксирует порядок side effects в
вызывающем коде и не позволяет worker видеть частично собранный context.

## 9. Связь с Memory runtime

Прежний process-global active-region pointer был несовместим с concurrency и уже
заменён thread-local storage:

```c
#if defined(_MSC_VER)
#define SK_THREAD_LOCAL __declspec(thread)
#else
#define SK_THREAD_LOCAL _Thread_local
#endif

static SK_THREAD_LOCAL SkMemoryRegion *sk_active_region = NULL;
static SK_THREAD_LOCAL SkTask *sk_current_task = NULL;
```

Каждая task начинает с `sk_active_region == NULL`. Локальная `Memory`, созданная
в worker, принадлежит только этому worker. Передача `Memory` между tasks в MVP
запрещена.

`Memory.clear()` не синхронизируется автоматически с tasks. Owner не имеет права
очищать region, пока task может читать принадлежащий ему payload; frontend MVP
решает это жёстким запретом region-owned payload через task boundary.

## 10. Platform layer

### Windows

- `CreateThread` / `WaitForSingleObject` / `CloseHandle`;
- `CRITICAL_SECTION` для mutex;
- `CONDITION_VARIABLE` для Channel slice;
- interlocked operations либо lock-protected state для stop visibility.

### POSIX

- `pthread_create` / `pthread_join`;
- `pthread_mutex_t`;
- `pthread_cond_t` для Channel slice;
- lock-protected state либо C11 atomics.

Generated source выбирает backend через `_WIN32`. CLI добавляет `-pthread` для
GCC/Clang на POSIX host и Linux target. Для MSVC и MinGW Win32 primitives не
требуют отдельного pthread runtime.

### 10.1. ESP32 / RTOS roadmap

ESP32, ESP-IDF, FreeRTOS и bare-metal не входят в текущий backend и не считаются
поддержанными targets. Наличие pthread compatibility layer в ESP-IDF само по себе
не делает generated POSIX C готовым embedded port.

Рекомендуемый порядок реализации:

1. ESP-IDF target profiles для Xtensa и RISC-V chips;
2. toolchain discovery, build и flash boundaries в `skadi-cli`;
3. минимальный bring-up через ESP-IDF pthread compatibility;
4. прямой FreeRTOS backend для явных stack size, priority, core affinity и static
   allocation;
5. отказ от безусловного heap allocation task context и Channel buffer в пользу
   profile-controlled static или region-backed storage;
6. emulator smoke и hardware-in-the-loop regression suite.

Перед портом должны быть зафиксированы resource exhaustion policy, максимальное
число tasks, размеры stack, priority model, pinning и источник памяти Channel.
Текущий desktop backend не даёт hard real-time guarantees и не является
достаточным контрактом для production firmware.

## 11. Channel runtime, второй slice

`Channel(T)` lowering создаёт typed wrapper над общей bounded queue:

```text
buffer[capacity]
head / tail / count
mutex
not_empty condition
not_full condition
```

Контракт:

- capacity должна быть больше нуля;
- FIFO order сохраняется;
- `send` ждёт `not_full`, когда очередь заполнена;
- `receive` ждёт `not_empty`, когда очередь пуста;
- payload копируется по value-safe representation;
- channel owner создаётся в caller и должен пережить все tasks-пользователи;
- освобождение channel возможно только после `wait` всех tasks, которым он передан;
- channel handle нельзя копировать обычным присваиванием, но generated task context
  может хранить внутреннюю ссылку на него как capability transfer.

В MVP нет `close`. Следовательно, завершение consumer должно обеспечиваться
протоколом сообщений, известным программе, либо гарантированным количеством
сообщений. `stop` сам по себе не выводит task из `receive`.

## 12. Ошибки и diagnostics

Классы diagnostics:

- `SC-SEM-070`: lifecycle task handle, unsupported task entry и task boundary rules;
- `SC-SEM-071`: запрещённое использование Task capability как обычного value;
- `SC-SEM-080`: Channel type, message safety и channel lifecycle rules;
- `SC-CG-301`: зарезервирован для будущей принятой frontend-формы без lowering;
  текущий Task/Channel MVP не использует этот gate;
- `SC-RT-301`: native task creation failed;
- `SC-RT-302`: task join failed;
- `SC-RT-303`: invalid task runtime state;
- `SC-RT-304`: task stop synchronization failed;
- `SC-RT-311`: channel allocation/init failed;
- `SC-RT-312`: invalid channel capacity;
- `SC-RT-313`: channel synchronization failed.

Runtime diagnostics печатаются в stderr в единой форме и завершают процесс, если
текущий syntax contract не предоставляет recoverable boundary.

## 13. Порядок реализации

### Slice A: Task core

1. добавить task-safe boundary и all-path handle lifecycle checks - выполнено;
2. сделать `sk_active_region` thread-local - выполнено;
3. добавить platform thread wrappers - выполнено для Win32/pthread;
4. генерировать context и trampoline - выполнено для void и result-bearing tasks;
5. lowering `run`, `wait` - выполнено;
6. lowering `stop`, `stopping` - выполнено для Win32/pthread;
7. удалить общий gate для поддержанного task-only surface - выполнено;
8. добавить shape, native runtime и official CLI tests - выполнено для Task slice.

### Slice B: Task results

1. typed result slot - выполнено;
2. move/copy-out при `wait` - выполнено;
3. scalar и value-like struct e2e - выполнено;
4. Text result transfer e2e - выполнено.

### Slice C: Channel core

1. platform condition-variable wrappers - выполнено для Win32/pthread;
2. generic bounded queue storage - выполнено;
3. typed generated `send/receive` wrappers - выполнено;
4. channel lifetime и value-safe semantic checks - выполнено для MVP;
5. producer/consumer и backpressure tests - выполнено для capacity `1` FIFO.

### Slice D: hardening

1. repeated stress loops - выполнено для 1000/10000-message Channel paths;
2. ThreadSanitizer на поддерживаемом CI target - обязательный Ubuntu GCC job добавлен;
3. Windows MinGW/MSVC и Linux GCC/Clang matrix - systems build/run job добавлен;
4. CLI/TUI diagnostics и showcases - CLI smoke и два concurrency showcases добавлены;
5. docs/status synchronization - выполняется вместе с каждым runtime slice.

## 14. Acceptance gate

Task runtime MVP считается реализованным, когда:

- task-only `run/wait`, result task и cooperative stop работают end-to-end;
- task resources всегда joined и освобождаются;
- Memory active context thread-local;
- task-unsafe values отвергаются frontend;
- generated C компилируется GCC, Clang и поддержанным Windows compiler;
- runtime tests проходят под sanitizer там, где sanitizer доступен;
- docs и syntax status больше не называют реализованный slice frontend-only.

Channel runtime MVP считается реализованным отдельно, когда bounded FIFO,
blocking semantics, ownership rules и stress tests проходят тот же release gate.
