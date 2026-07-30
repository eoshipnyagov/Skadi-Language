# Владение и передача ресурсов

Skadi разделяет обычные значения и owning resources.

- `Int`, `Float`, `Bool`, `Char`, specialized numeric types и небольшие
  value-структуры копируются по значению.
- `Canvas`, `Window`, `Interrupt` и owning `Channel` имеют ровно одного
  владельца.
- `Memory` является region capability и не передаётся через `move`.
- `Task` имеет отдельный lifecycle: owner обязан завершить его через `wait`.

## Посмотреть, изменить или отдать

```skadi
fn fingerprint(view Canvas frame) returns Int {
    return frame.checksum()
}

fn paint(direct Canvas frame) {
    frame.clear(Color.terminal_blue)
}

fn consume(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(32, 24)
new Int before = fingerprint(view frame)
paint(direct frame)
consume(move frame)
```

- `view` даёт read-only borrow на время синхронного вызова.
- `direct` даёт exclusive mutable borrow на время вызова.
- `move` окончательно передаёт ownership. Старое имя после вызова недоступно.

Маркер виден и в сигнатуре, и в call site. Это не оптимизационная подсказка, а
часть поведения программы.

## Новое имя и factory

```skadi
Canvas first = canvas(32, 24)
Canvas second = move first
```

Теперь owner называется `second`; использовать `first` нельзя.

Factory возвращает ресурс только явно:

```skadi
fn make_frame() returns Canvas {
    Canvas frame = canvas(32, 24)
    frame.clear(Color.terminal_black)
    return move frame
}

Canvas frame = make_frame()
```

Результат функции уже является новым owner value, поэтому в assignment
дополнительный `move` не нужен.

## Control flow

```skadi
if hand_off {
    consume(move frame)
}

output(frame.checksum())
```

Такой код отклоняется: после `if` `frame` существует не на всех продолжающихся
путях. Передавайте ресурс после ветвления либо полностью завершайте работу с ним
в каждой ветке.

Move owner из повторяющегося `while`, `loop` или `iterate` также запрещён:
следующая итерация не должна неявно получать уже отданный ресурс. Создайте новый
owner внутри итерации, верните ресурс или передайте его после цикла.

## Cleanup и ошибки

Если scope всё ещё владеет ресурсом при выходе, generated runtime освобождает
его автоматически в обратном порядке создания. После `move` cleanup выполняет
только новый owner.

Закрытый ресурс и moved resource различаются:

- операцию над `closed`/`maybe closed` можно обработать через `on error`;
- потеря ownership после `move` является статической ошибкой и не
  восстанавливается через `on error`.

Полный набор проверочных сценариев находится во внутреннем
[Ownership UX Lab](../internal/ownership-ux-lab.md). Рабочий пример:
`examples/ownership/01_move_canvas_factory.skd`.
