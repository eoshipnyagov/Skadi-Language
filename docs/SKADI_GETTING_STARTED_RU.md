# Быстрый старт

Skadi — компилируемый системный язык с читаемым синтаксисом, явными recovery и
ownership boundaries и предсказуемым понижением в C.

## Идея языка

Skadi старается соединить:

- простую запись прикладного кода;
- видимую стоимость системных операций;
- управляемую память без скрытого garbage collector;
- message-passing-first многопоточность;
- один понятный toolchain от проекта до native binary.

Язык сознательно не стремится собрать все популярные возможности. Generics,
decorators, pipelines, implicit async runtime и свободное operator overloading
не входят в текущую модель только потому, что существуют в других языках.

## Установка

Готовые installers и архивы выпускаются для Windows, Linux и macOS. Полные
команды, проверка checksum, update и uninstall:

[Установка Skadi](installation.md)

После установки проверьте окружение:

```powershell
skadi-cli --version
skadi-cli doctor
```

Skadi генерирует C, поэтому для `build` и `run` нужен подходящий C compiler.
Команда `check` работает без него.

## Первый проект

В рабочей директории:

```powershell
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli run
```

Проект содержит:

```text
hello_skadi/
  Skadi.toml
  src/
    main.skd
```

`src/main.skd`:

```skadi
new Text greeting = concat("Hello", " from Skadi")
output(greeting)

new Angle quarter_turn = 90deg
output(rad_to_deg(quarter_turn))
```

## Повседневный цикл

```powershell
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
```

- `check` проверяет imports, syntax и semantics;
- `format` приводит `.skd` к канонической записи;
- `build` создаёт C и native executable;
- `run` собирает и запускает программу.

Для интерактивной работы:

```powershell
skadi-cli tui
```

CLI остаётся каноническим интерфейсом для scripts и CI; TUI — полноценный
keyboard-first интерфейс для ручной работы.

## Минимум языка

```skadi
fn sum_positive(Int List values) returns Int {
    new Int total = 0

    iterate values as value {
        if value > 0 {
            total = total + value
        }
    }

    return total
}

new Int List samples = [3, -1, 8]
output(sum_positive(samples))
```

Здесь показаны typed function, `List`, канонический цикл `iterate`, условие,
присваивание и builtin `output`.

## Явные ошибки

```skadi
label ErrorCode {
    Ok
    InvalidValue
}

danger fn require_positive(Int value) returns Int {
    if value <= 0 {
        return error InvalidValue
    }
    return value
}

new Int checked = 0
checked = require_positive(10) on error {
    output("invalid value")
}
```

Recovery виден в месте вызова; `danger fn` нельзя вызвать и забыть проверить.

## Системные возможности `v1.2`

Текущая линия добавляет experimental, но исполняемые end-to-end возможности:

- experimental [Memory regions](memory.md);
- call-scoped borrow и явная передача resource ownership через `move`;
- native [Task и Channel](concurrency.md);
- [Time и Duration](time-duration.md);
- [ByteSize](byte-size.md);
- [Angle](angle.md);
- [Vec2/Vec3/Vec4](vectors.md).

Experimental означает, что API ещё может уточняться, а не то, что это
parser-only макет.

## Куда идти дальше

1. [Справочник CLI/TUI](cli-reference.md)
2. [Быстрая справка по всему языку](language-quick-reference.md)
3. [Полная справка по темам](language-reference.md)
4. [Как писать и чего избегать](practices.md)
5. [Showcase-программы](showcases.md)
