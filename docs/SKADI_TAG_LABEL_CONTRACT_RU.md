# Контракт `tag` и `label`

Статус: **Implemented stable contract**.

## 1. Разделение сущностей

`tag` задаёт закрытый nominal-набор символических вариантов. Числовое
представление не является частью его контракта и недоступно программе.

```skadi
tag Status {
    Ready
    Busy
    Failed
}
```

`label` задаёт закрытый nominal-набор вариантов с обязательными явными
целочисленными discriminants.

```skadi
label ErrorCode {
    Ok = 0
    Missing = 10
    Invalid = 20
}
```

Порог по количеству вариантов не используется: форма декларации определяет
контракт сразу и не меняется после добавления очередного варианта.

## 2. Инварианты

- имя типа и имена вариантов уникальны в своей декларации;
- discriminants `label` уникальны;
- alias одного discriminant несколькими именами в первом slice запрещён;
- canonical value form вне контекста ожидаемого типа: `Status.Ready`;
- внутри `when value { is Ready { ... } }` разрешена краткая форма, поскольку
  тип subject однозначно задаёт namespace;
- `tag` и `label` являются nominal types и не смешиваются с `Int`;
- преобразование `label -> Int` не является неявным;
- порядок вариантов `tag` не является ABI или serialization contract.

## 3. `ErrorCode`

`label ErrorCode` является обычным числовым `label` с дополнительным
error-flow контрактом:

- вариант `Ok = 0` обязателен;
- `Ok` должен быть первым;
- `return error Name` принимает только вариант `ErrorCode`;
- остальные discriminants могут быть любыми уникальными целыми значениями.

## 4. Scope и modules

`local tag` и `local label` не экспортируются. Публичные типы и варианты
участвуют в тех же правилах direct-import visibility, что `struct` и `fn`.

## 5. Отложено

- aliases вариантов;
- flags/bitmask labels;
- string serialization names;
- open/extensible tags;
- автоматические числовые discriminants.
