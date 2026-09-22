use v01::codegen::{CTarget, CodegenOptions, transpile_program_to_c_with_options};
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
    assert!(generated.contains("xTaskCreate("), "{generated}");
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
        generated.contains("static void SK_INTERRUPT_ATTR sk_interrupt_handler_0"),
        "{generated}"
    );
    assert!(!generated.contains("pthread_"), "{generated}");
    assert!(!generated.contains("int main(void)"), "{generated}");
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
