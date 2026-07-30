# Ownership UX Lab: владею, одалживаю, отдаю

Дата: 2026-07-30
Статус: **accepted UX contract / bounded ownership engine implemented**

Этот документ фиксирует UX общей модели владения и показывает реализованный
bounded ownership engine. Он не является обещанием полного lifetime calculus.

Связанные контракты:

- [Borrow-доступ и ресурсы](direct-borrow-resource-contract.md)
- [Memory MVP](memory-model-mvp.md)
- [Task MVP](task-model-mvp.md)
- [Channel lifecycle](channel-lifecycle-contract.md)

## 1. Объяснение без терминологии компилятора

У программы есть вещи и их владельцы.

1. **Владею.** Если я создал ресурс, он мой. Пока он мой, я отвечаю за него.
2. **Одалживаю посмотреть.** Если resource нельзя копировать, `view`
   разрешает функции посмотреть на оригинал, но не менять его и не забирать себе.
3. **Одалживаю изменить.** `direct` разрешает функции временно изменить оригинал,
   но после вызова он снова у владельца.
4. **Отдаю.** `move` передаёт владение другому. После этого старый владелец
   больше не может пользоваться ресурсом.
5. **Ухожу из комнаты.** При выходе из scope всё, чем scope ещё владеет,
   освобождается автоматически.

Короткая формула:

```text
value можно скопировать
resource можно одолжить или отдать
одолженное всегда возвращается после вызова
отданное больше не принадлежит прежнему имени
```

Если для чтения обычной функции нужно понимать больше этой модели, дизайн
считается слишком сложным.

## 2. Слова, которые видит пользователь

| Намерение | Предлагаемая форма | Статус |
|---|---|---|
| Создать владельца | `Canvas frame = canvas(...)` | Implemented |
| Прочитать copyable value | `fn show(Int value)` | Implemented / preferred |
| Посмотреть на resource | `view Canvas frame` | Implemented |
| Временно изменить | `direct Canvas frame` | Implemented |
| Передать read-only borrow в вызов | `inspect(view frame)` | Implemented |
| Передать mutable borrow в вызов | `draw(direct frame)` | Implemented |
| Передать владение | `consume(move frame)` | Implemented |
| Вернуть владение | `return move frame` | Implemented |
| Автоматически освободить | выход из scope | Implemented for current owning resources |

Исходный владелец всегда виден как обычное имя. `view`, `direct` и `move`
обязаны быть видны в call site: опасная граница не должна прятаться в сигнатуре.

## 3. Полигон из десяти сценариев

### Сценарий 1. Обычное значение копируется

Статус: **Implemented**.

```skadi
new Int original = 10
new Int copy = original
copy = copy + 1

output(original)
output(copy)
```

Ожидание: `original` остаётся равен `10`. Ownership-правила не должны добавлять
церемонию к простым value types.

### Сценарий 2. Copyable value читается по значению

Статус: **Implemented**.

```skadi
fn show(Int value) {
    output(value)
}

constant Int answer = 42
show(answer)
```

Ожидание: функция получает независимое значение. Присваивание локальному
`value` не меняет `answer`.

Форма `view Int` технически возможна в текущем bounded slice, но для
`Int` не даёт полезного пользовательского поведения и не должна быть
каноническим стилем. Компилятор может оптимизировать передачу value без
изменения source semantics.

### Сценарий 3. Временно изменить оригинал

Статус: **Implemented**.

```skadi
fn increment(direct Int value) {
    value = value + 1
}

new Int count = 1
increment(direct count)
output(count)
```

Ожидание: после синхронного вызова borrow заканчивается, а `count` снова
полностью доступен owner scope.

### Сценарий 4. Resource живёт до конца scope

Статус: **Implemented per resource**.

```skadi
fn render_preview() {
    Canvas frame = canvas(64, 48)
    frame.clear(Color.terminal_black)
    frame.circle(
        {x = 32.0, y = 24.0},
        12.0,
        Color.terminal_bright_yellow
    )
}
```

Ожидание: `frame` освобождается автоматически при выходе из функции, включая
ранний `return`. Пользователь не пишет обязательный `destroy`.

Ownership state проверяется общей state machine; конкретный cleanup остаётся
типизированным, потому что у каждого resource свой runtime destructor.

### Сценарий 5. Borrowed resource: чтение и изменение разделены

Статус: **Implemented для Canvas**.

```skadi
fn paint(direct Canvas frame) {
    frame.clear(Color.terminal_blue)
}

fn fingerprint(view Canvas frame) returns Int {
    return frame.checksum()
}

Canvas frame = canvas(64, 48)
paint(direct frame)
new Int checksum = fingerprint(view frame)
```

Ожидание: `fingerprint` не может вызвать `clear`, а `paint` может. Ни одна
функция не может сохранить `frame` после возврата.

### Сценарий 6. Lifecycle должен переживать ветвления

Статус: **Implemented для Window и Channel**.

```skadi
Canvas frame = canvas(320, 200)
Window window = windows.open("Preview", 320, 200)
new Bool should_close = true

if should_close {
    window.close()
}

window.present(direct frame) on error {
    pass
}
```

После `if` состояние `window` равно `maybe closed`. Операция разрешена, потому
что возможная ошибка обработана через `on error`. Без обработчика compiler
сообщает, что resource может быть закрыт, и предлагает добавить `on error` либо
перестроить control flow.

Если resource заведомо закрыт, операция с `on error` остаётся допустимой, но
compiler предупреждает, что выполнение гарантированно войдёт в handler.
Закрытое состояние можно обработать; потерю владения после `move` —
нельзя.

### Сценарий 7. Задача остаётся у owner до `wait`

Статус: **Implemented**.

```skadi
fn measure() returns Int {
    return 42
}

Task(Int) work = run measure()
new Int result = wait work
output(result)
```

Ожидание: scope нельзя покинуть с живым Task; после `wait` handle считается
consumed. Повторный `wait work` является понятной ошибкой.

### Сценарий 8. Явно отдать resource функции

Статус: **Implemented**.

```skadi
fn finish(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(64, 48)
finish(move frame)
output(frame.checksum())
```

Последняя строка получает diagnostic:

```text
resource 'frame' was moved to 'finish' and is no longer available
```

`move` нужен с обеих сторон границы: функция обещает принять ownership, а
call site явно показывает, что старое имя станет недоступно.

### Сценарий 9. Фабрика возвращает resource

Статус: **Implemented**.

```skadi
fn make_frame(Int width, Int height) returns Canvas {
    Canvas frame = canvas(width, height)
    frame.clear(Color.terminal_black)
    return move frame
}

Canvas frame = make_frame(320, 200)
```

Ownership переходит вызывающему коду. В месте присваивания дополнительный
`move` не нужен: результат функции является новым временным owner value и не
оставляет доступного старого имени.

### Сценарий 10. Control-flow не должен скрывать частичный move

Статус: **Implemented**.

```skadi
fn finish(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(64, 48)
new Bool hand_off = true

if hand_off {
    finish(move frame)
}

output(frame.checksum())
```

Diagnostic сообщает, что после `if` ресурс доступен не на
всех путях. Исправления должны быть очевидными:

- использовать resource полностью внутри каждой ветки;
- переместить `move` после ветвления;
- создать отдельный resource для принимающей стороны.

Та же логика применяется к циклам: ресурс, отданный в одной итерации, нельзя
молча использовать в следующей. Новый owner создаётся внутри итерации либо
ресурс возвращается явным результатом.

## 4. Проверка «объяснить двенадцатилетнему»

Если переменная содержит обычное copyable value, функции можно отдать копию.
Изменение копии не меняет исходную переменную.

Если переменная владеет resource, его можно передать через `view`, чтобы функция
только посмотрела на него. Одновременно смотреть могут несколько функций.
Через `direct` функция получает право временно пользоваться оригиналом и менять
его; изменяемый доступ в этот момент может быть только один. После синхронного
вызова доступ возвращается владельцу.

`move` отдаёт resource другому владельцу. После этого старой переменной
больше нет в смысле ownership, и пользоваться её именем нельзя. Закрытый
resource отличается от moved: переменная ещё существует, поэтому неудачную
операцию можно явно обработать через `on error`.

Если owner выходит из scope и никому не передал resource, программа освобождает
его автоматически.

Что не нужно объяснять пользователю:

- lifetime-параметры;
- reference counting;
- allocator internals;
- generated C pointers;
- borrow checker implementation;
- платформенный handle.

Итог проверки:

- передача copyable values по значению проходит тест объяснимости;
- `direct` для изменения caller value и `view` для чтения resource
  проходят тест объяснимости;
- scope cleanup проходит тест объяснимости;
- явный `move` является естественным глаголом передачи ownership;
- скрытый implicit move для именованных ресурсов будет хуже объясняться;
- текущие специальные правила `Memory`, `Task`, `Channel`, `Canvas` и `Window`
  должны быть объединены внутри компилятора, но не в пользовательском синтаксисе.

## 5. Инварианты bounded ownership engine

Эти пункты являются текущим bounded contract:

1. У каждого resource ровно один owner.
2. Закрываемый resource имеет состояние `open`, `maybe closed` или `closed`;
   ownership отдельно имеет состояния `owned` и `moved`.
3. Borrow живёт только один синхронный вызов.
4. `view` запрещает логически изменяющие операции.
5. `view` и `direct` нельзя передать через `run`, сохранить или вернуть.
6. Операция над `maybe closed` или `closed` требует `on error`; для заведомо
   закрытой операции выдаётся warning.
7. `move` завершает доступ через исходное имя и не может быть обработан через
   `on error`.
8. Branch merge учитывает состояние resource на всех продолжающихся путях.
9. Loop не переносит moved resource в следующую итерацию неявно.
10. Cleanup выполняется один раз текущим owner в обратном порядке acquisition.
11. Diagnostics называют resource, действие, место передачи и практичное
    исправление.

Каноническое UX-правило:

```text
copyable value: читать по значению, изменять caller через direct
resource: читать через view, изменять через direct, отдавать через move
```

## 6. Что показал полигон

Уже достаточно цельно работают:

- value copy;
- immutable/mutable synchronous borrow;
- Task consume-through-wait;
- deterministic cleanup отдельных resource types;
- запрет borrow через task boundary;
- общий close-lifecycle `open / maybe closed / closed` для Window и Channel;
- восстановление через `on error` без небезопасного обращения к OS handle.

Реализованный ownership slice:

- общая state machine `owned / maybe moved / moved` для `Canvas`, `Window`,
  `Interrupt` и owning `Channel`;
- передача между именованными bindings через `new Resource next = move source`;
- factory return через `return move source`;
- branch merge учитывает только продолжающиеся ветки;
- перенос owner из повторяющегося loop запрещён;
- generated C обнуляет moved-from handle и освобождает ресурс ровно у текущего
  owner.

Ограничения остаются намеренно заметными:

- `Memory` не является movable value: это scope/region capability;
- `Task` завершается специальным `wait`, а не общим `move`;
- borrow остаётся call-scoped и не является first-class reference;
- новые resource types должны явно подключаться к ownership и cleanup engine.
