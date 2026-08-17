# Полная справка по языку

Справочник разделён по темам. Для одного экрана со всеми формами, типами,
functions и builtins используйте [быструю справку](language-quick-reference.md).

## Основы

- [Базовые типы, литералы и выражения](language-basics.md)
- [Ветвления и циклы](control-flow.md)
- [Функции, видимость и обработка ошибок](functions-errors.md)
- [Struct, Text и List](data-model.md)
- [Файлы, импорты и видимость](modules.md)
- [Ввод, вывод и файловая система](io-files.md)
- [Математика](math.md)

## Systems surface `v1.2`

- [Управление памятью](memory.md)
- [Владение и передача ресурсов](ownership.md)
- [Многопоточность: Task и Channel](concurrency.md)
- [Time и Duration](time-duration.md)
- [ByteSize](byte-size.md)
- [Angle](angle.md)
- [Vec2, Vec3 и Vec4](vectors.md)
- [Платформенный Int и битовые операции](bits.md)

Эти возможности работают end-to-end, но пока имеют статус `Experimental`: их
public API ещё не заморожен.

## Практика и точный статус

- [Как писать и чего избегать](practices.md)
- [Статус каждой syntax surface](syntax-status.md)
- [Короткие примеры](language-examples.md)
- [Showcase-программы](showcases.md)

## Платформы и будущее

- [Embedded: платформенный статус](embedded.md)
- [Canvas и Visual Core](canvas.md)
- [Что Skadi сознательно не поддерживает](../internal/v1-non-goals.md)

Future-примеры всегда помечены как некомпилируемые. Они не должны смешиваться с
рабочей справкой текущего компилятора.
