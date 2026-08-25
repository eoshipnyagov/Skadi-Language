# Контракт `view` / `edit` доступа и ресурсов

Статус: **Accepted design, bounded MVP**.

Практическая проверка читаемости и будущей передачи владения:
[Ownership UX Lab](ownership-ux-lab.md).

## 1. Ментальная модель

У значения есть один owner. Owner может временно дать оригинал функции:

- посмотреть через `view`;
- изменить через `edit`;
- окончательно передать через `move`.

Функция не может сохранить borrowed value, вернуть его или передать задаче,
которая переживёт вызов.

## 2. Синтаксис

```skadi
fn show(view Player player) {
    output(player.name)
}

fn damage(edit Player player, Int amount) {
    player.health = player.health - amount
}

show(view player)
damage(edit player, 10)
```

`view` и `edit` видны в declaration и call site. Это намеренно делает передачу
оригинала заметной с обеих сторон API.

## 3. Bounded MVP rules

- borrow живёт ровно один синхронный вызов;
- `edit T` является exclusive mutable borrow;
- `constant` binding и `view` parameter нельзя передать через `edit`;
- `view T` является shared read-only borrow;
- одновременно разрешён один mutable borrow либо несколько read-only borrows;
- borrowed reference не является first-class value;
- borrow нельзя записать в variable, `struct`, `List` или `Channel`;
- borrow нельзя вернуть;
- borrow нельзя передать через `run`;
- lifetime-аннотации в source не нужны.

На C-границе тот же контракт используется формой `view Buffer(T)` /
`edit Buffer(T)`: typed `T List` раскрывается в pointer + length только на время
синхронного `external fn` вызова. Сам `Buffer(T)` не является storable type.

## 4. Resource handles

`File`, `Port`, `Window`, `Display` и похожие types являются linear owning
capabilities:

- не копируются;
- автоматически закрываются в конце scope;
- могут быть закрыты раньше через `.close()`;
- операция над `maybe closed` или `closed` требует `on error`;
- заведомо закрытая операция с handler допустима, но получает warning;
- `move` и нарушение borrow-прав не являются runtime-ошибками и не обходятся
  через `on error`;
- cleanup идёт в обратном порядке acquisition;
- owning resource передаётся функции через явный `move` в сигнатуре и call site;
- factory возвращает resource только как `return move resource`.

Текущие однооперационные `read`/`write` не требуют немедленного появления
долгоживущего `File` handle.

Opaque C handle объявляется через `external resource Name`. Он использует те же
`view`/`edit`/`move` правила, но не имеет автоматического cleanup: frontend не
знает destructor внешней библиотеки, поэтому owner должен быть явно передан
consuming external-функции на каждом пути до выхода из scope.

`.close()` не является универсальным resource API. Сейчас он предметно
реализован у `Window` и `Channel`; будущие `File`, `Port` и `Socket` смогут
использовать тот же lifecycle engine. `Task`, `Interrupt`, `Memory` и `Canvas`
сохраняют собственные естественные операции `wait`/`stop`, `clear` либо только
автоматический cleanup.

## 5. Keywords

- canonical immutable marker называется `constant`, не `const`;
- `view` относится к read-only borrow model и не делает сам resource глобально immutable;
- `edit` относится к borrow model;
- `view`, `edit` и `move` остаются полностью зарезервированными
  словами, а не contextual keywords;
- `fixed` и `const` являются обычными identifiers, а не declaration modifiers;
- если embedded потребует static storage, будет введён отдельный `static`
  contract, а не переиспользован `fixed`;
- `allow grow/drop` является contextual policy только внутри `memory(...)`.
