# Управление памятью

Memory model `v1.2` — исполняемый experimental region runtime с fixed,
growing, child и static областями. Это bounded-модель, а не полная
lifetime/borrow theory из design draft.

## Регион

```skadi
label ErrorCode {
    Ok
    OutOfMemory
}

Memory scratch = memory(8kb) on error {
    output("cannot create memory")
}
```

`memory` принимает `ByteSize`. `Memory` является capability: его нельзя
копировать, класть в `List/struct`, передавать через `Channel` или возвращать как
обычное значение.

## Размещение

```skadi
place in scratch {
    new Text preview = concat("frame-", "preview")
    output(preview)
} on error {
    output("memory budget exceeded")
}
```

Внутри `place in` динамический payload (`Text`, `List` и содержащие их struct)
связан с активным регионом. Semantic запрещает опасный escape в более долгоживущий
owner.

## Очистка

```skadi
scratch.clear()
```

После `clear()` старые region-owned значения использовать нельзя. Очистка
активного региона внутри его `place in` также запрещена.

## Рост, дочерняя и статическая память

```skadi
Memory cache_memory = memory(64kb, allow grow, allow drop)

place in cache_memory {
    Memory scratch_memory = memory.child(8kb)
    place in scratch_memory {
        new Text preview = "temporary"
        output(preview)
    }
}

Memory device_memory = memory.static(4kb)
```

- `allow grow` создаёт дополнительные chunks. Ранее размещённые значения не
  перемещаются и не получают висячие адреса.
- `allow drop` является явным разрешением владельца для будущих bounded
  стратегий. Текущий runtime хранит флаг, но никогда самопроизвольно не удаляет
  живые значения.
- `memory.child(size)` доступна только внутри `place in parent_memory`. Она
  резервирует fixed-capacity участок в parent и не поддерживает `allow grow`.
- `memory.static(size)` требует положительный literal вроде `4kb`, не использует
  heap для своего буфера и разрешена только на корневом уровне программы.
- `clear()` parent-region также делает его child-regions недоступными.

Полный рабочий пример:
`examples/memory/positive/06_extended_regions.skd`.

## Владение ресурсами

`Canvas`, `Window`, `Interrupt` и owning `Channel` имеют одного владельца.
Оригинал можно временно передать через `view`/`direct` либо окончательно отдать:

```skadi
fn consume(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(32, 24)
consume(move frame)
```

После `move frame` старое имя недоступно. Factory возвращает ресурс явно:

```skadi
fn make_frame() returns Canvas {
    Canvas frame = canvas(32, 24)
    return move frame
}
```

Передача в одной ветке создаёт `maybe moved` на объединённом пути. Передача
owner из повторяющегося loop запрещена. Обычные значения копируются без
`move`; `Memory` и `Task` используют собственные capability/lifecycle правила.

Полный пользовательский контракт: [Владение и передача ресурсов](ownership.md).

## Что пока отсутствует

- embedded allocator contract;
- полный lifetime/borrow calculus и first-class references;
- автоматическая reclamation-стратегия для `allow drop`;
- автоматическое управление произвольными OS resources.

Подробные positive/negative сценарии находятся в
[Memory examples](../internal/memory-model-examples.md), а исходный замысел — во
[внутреннем Memory Draft](../internal/memory-model-draft.md).
