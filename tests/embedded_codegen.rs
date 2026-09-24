use v01::codegen::{
    CTarget, CodegenOptions, EmbeddedCodegenOptions, RuntimeAllocation,
    transpile_program_to_c_with_options,
};
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

#[test]
fn esp_idf_codegen_uses_skadi_surface_over_freertos_and_gptimer() {
    let source = r#"
external fn board_set_led(Bool enabled)

fn consume(Channel(Int) ticks) returns Int {
    new Int value = ticks.receive()
    board_set_led(true)
    return value
}

Channel(Int) ticks = channel(2)
Task(Int) worker = run consume(ticks)
Interrupt timer = interrupts.periodic(10ms)
on interrupt timer {
    ticks.try_send(1)
}
Interrupt button = interrupts.gpio(0, InterruptEdge.Falling, GpioPull.Up)
on interrupt button {
    ticks.try_send(1)
}
new Int result = wait worker
ticks.close()
"#;
    let tokens = lex(source).expect("lex embedded source");
    let program = parse_program(&tokens).expect("parse embedded source");
    semantic_analyze(&program).expect("semantic embedded source");
    let generated = transpile_program_to_c_with_options(
        &program,
        CodegenOptions {
            target: CTarget::EspIdf,
            ..CodegenOptions::default()
        },
    )
    .c_code;

    assert!(generated.contains("void app_main(void)"), "{generated}");
    assert!(
        generated.contains("xTaskCreatePinnedToCore("),
        "{generated}"
    );
    assert!(generated.contains("task->active_controls"), "{generated}");
    assert!(generated.contains("xQueueCreate("), "{generated}");
    assert!(generated.contains("xQueueSendFromISR("), "{generated}");
    assert!(
        generated.contains("SKADI_CHANNEL_POLL_TICKS"),
        "{generated}"
    );
    assert!(generated.contains("channel->active_senders"), "{generated}");
    assert!(
        generated.contains("sk_channel_begin_send_from_isr"),
        "{generated}"
    );
    assert!(generated.contains("gptimer_new_timer("), "{generated}");
    assert!(
        generated.contains("sk_interrupt_gpio(0, InterruptEdge_Falling, GpioPull_Up)"),
        "{generated}"
    );
    assert!(generated.contains("gpio_isr_handler_add("), "{generated}");
    assert!(
        generated.contains("static void SK_INTERRUPT_ATTR sk_interrupt_handler_0"),
        "{generated}"
    );
    assert!(!generated.contains("pthread_"), "{generated}");
    assert!(!generated.contains("int main(void)"), "{generated}");
}

#[test]
fn gpio_interrupt_is_rejected_for_desktop_target() {
    let source = "Interrupt button = interrupts.gpio(0, InterruptEdge.Falling, GpioPull.Up)\n";
    let tokens = lex(source).expect("lex GPIO source");
    let program = parse_program(&tokens).expect("parse GPIO source");
    semantic_analyze(&program).expect("semantic GPIO source");
    let error =
        v01::codegen::ensure_codegen_supported_with_options(&program, CodegenOptions::default())
            .expect_err("desktop target should reject GPIO interrupts");

    assert!(error.contains("[SC-CG-303]"), "{error}");
    assert!(error.contains("interrupts.gpio"), "{error}");
}

#[test]
fn esp_idf_static_profile_uses_static_freertos_storage() {
    let source = r#"
fn echo(Channel(Int) values) returns Int {
    return values.receive()
}

Channel(Int) values = channel(3)
Task(Int) worker = run echo(values)
values.send(7)
new Int result = wait worker
values.close()
"#;
    let tokens = lex(source).expect("lex static embedded source");
    let program = parse_program(&tokens).expect("parse static embedded source");
    semantic_analyze(&program).expect("semantic static embedded source");
    let generated = transpile_program_to_c_with_options(
        &program,
        CodegenOptions {
            target: CTarget::EspIdf,
            embedded: EmbeddedCodegenOptions {
                allocation: RuntimeAllocation::Static,
                task_stack_bytes: 6144,
                task_priority: 3,
                task_core: 0,
            },
            ..CodegenOptions::default()
        },
    )
    .c_code;

    assert!(
        generated.contains("#define SKADI_STATIC_RUNTIME 1"),
        "{generated}"
    );
    assert!(
        generated.contains("#define SKADI_TASK_STACK_BYTES 6144"),
        "{generated}"
    );
    assert!(
        generated.contains("#define SKADI_TASK_PRIORITY 3"),
        "{generated}"
    );
    assert!(
        generated.contains("#define SKADI_TASK_CORE 0"),
        "{generated}"
    );
    assert!(
        generated.contains("xTaskCreateStaticPinnedToCore("),
        "{generated}"
    );
    assert!(
        generated.contains("xSemaphoreCreateBinaryStatic("),
        "{generated}"
    );
    assert!(
        generated.contains(
            "StackType_t stack[SKADI_TASK_STACK_BYTES] __attribute__((aligned(portBYTE_ALIGNMENT)))"
        ),
        "{generated}"
    );
    assert!(generated.contains("xQueueCreateStatic("), "{generated}");
    assert!(
        generated.contains("static uint8_t values_item_storage[3 * sizeof(SkInt)]"),
        "{generated}"
    );
    assert!(
        generated.contains("static SkTask worker = {0}"),
        "{generated}"
    );
    assert!(
        generated.contains("static SkTaskContext_echo worker_context_storage = {0}"),
        "{generated}"
    );
    assert!(
        generated.contains("(void)((sk_channel_close(values) ? 0 : 1));"),
        "{generated}"
    );
}

#[test]
fn esp_idf_static_profile_rejects_runtime_channel_capacity() {
    let source = "new Int capacity = 3\nChannel(Int) values = channel(capacity)\nvalues.close()\n";
    let tokens = lex(source).expect("lex source");
    let program = parse_program(&tokens).expect("parse source");
    semantic_analyze(&program).expect("semantic source");
    let error = v01::codegen::ensure_codegen_supported_with_options(
        &program,
        CodegenOptions {
            target: CTarget::EspIdf,
            embedded: EmbeddedCodegenOptions {
                allocation: RuntimeAllocation::Static,
                ..EmbeddedCodegenOptions::default()
            },
            ..CodegenOptions::default()
        },
    )
    .expect_err("static channels require compile-time capacity");

    assert!(error.contains("[SC-CG-304]"), "{error}");
    assert!(error.contains("positive integer literal"), "{error}");
}

#[test]
fn esp_idf_codegen_rejects_host_only_io_before_the_c_toolchain() {
    let source = "new Text List values = args()\noutput(len(values))\n";
    let tokens = lex(source).expect("lex source");
    let program = parse_program(&tokens).expect("parse source");
    semantic_analyze(&program).expect("semantic source");
    let error = v01::codegen::ensure_codegen_supported_with_options(
        &program,
        CodegenOptions {
            target: CTarget::EspIdf,
            ..CodegenOptions::default()
        },
    )
    .expect_err("ESP-IDF should reject host-only args");

    assert!(error.contains("[SC-CG-303]"), "{error}");
    assert!(error.contains("does not support args"), "{error}");
}
