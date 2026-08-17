use v01::analysis::{AnalysisFactKind, AnalysisFactLevel, collect_analysis_facts};
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn analyze(source: &str) -> Vec<v01::analysis::AnalysisFact> {
    let tokens = lex(source).expect("lex analysis source");
    let program = parse_program(&tokens).expect("parse analysis source");
    semantic_analyze(&program).expect("semantic analysis source");
    collect_analysis_facts(&program)
}

#[test]
fn blocking_channel_facts_preserve_task_context_and_handler_state() {
    let facts = analyze(
        r#"
fn worker(Channel(Int) input, Channel(Int) output) {
    new Int value = 0
    value = input.receive() on error {
        if stopping {
            pass
        }
        pass
    }
    output.send(value)
}

Channel(Int) input = channel(1)
Channel(Int) output = channel(1)
Task worker_task = run worker(input, output)
input.send(7)
new Int result = output.receive()
wait worker_task
"#,
    );

    let receive = facts
        .iter()
        .find(|fact| fact.kind == AnalysisFactKind::BlockingChannelReceive)
        .expect("receive fact");
    assert_eq!(receive.level, AnalysisFactLevel::Info);
    assert_eq!(receive.code, "SC-AN-102");
    assert_eq!(receive.context, "task entry 'worker'");
    assert!(receive.explanation.contains("on error"));

    let task_send = facts
        .iter()
        .find(|fact| {
            fact.kind == AnalysisFactKind::BlockingChannelSend
                && fact.context == "task entry 'worker'"
        })
        .expect("task send fact");
    assert_eq!(task_send.level, AnalysisFactLevel::Attention);
    assert!(task_send.action.contains("attach on error"));
}

#[test]
fn incomplete_when_produces_explainable_fact() {
    let facts = analyze(
        r#"
new Int value = 1
when value {
    is 1 {
        output("one")
    }
}
"#,
    );

    let fact = facts
        .iter()
        .find(|fact| fact.kind == AnalysisFactKind::IncompleteWhen)
        .expect("incomplete when fact");
    assert_eq!(fact.code, "SC-AN-201");
    assert_eq!(fact.level, AnalysisFactLevel::Attention);
    assert!(fact.action.contains("else"));
}

#[test]
fn timed_channels_and_owned_resources_form_subject_lifecycle_chains() {
    let facts = analyze(
        r#"
fn worker() {
    pass
}

Channel(Int) jobs = channel(1)
Task worker_task = run worker()
jobs.send_for(7, 10ms) on error {
    if timed_out {
        pass
    }
}
jobs.close() on error {
    pass
}
stop worker_task
wait worker_task
"#,
    );

    let timed_send = facts
        .iter()
        .find(|fact| fact.kind == AnalysisFactKind::TimedChannelSend)
        .expect("timed send fact");
    assert_eq!(timed_send.code, "SC-AN-103");
    assert_eq!(timed_send.subject.as_deref(), Some("jobs"));
    assert!(timed_send.explanation.contains("timed_out"));

    let job_chain = facts
        .iter()
        .filter(|fact| fact.subject.as_deref() == Some("jobs"))
        .map(|fact| fact.kind)
        .collect::<Vec<_>>();
    assert!(job_chain.contains(&AnalysisFactKind::ResourceCreated));
    assert!(job_chain.contains(&AnalysisFactKind::ResourceClosed));

    let task_chain = facts
        .iter()
        .filter(|fact| fact.subject.as_deref() == Some("worker_task"))
        .map(|fact| fact.code)
        .collect::<Vec<_>>();
    assert!(task_chain.contains(&"SC-AN-311"));
    assert!(task_chain.contains(&"SC-AN-312"));
    assert!(task_chain.contains(&"SC-AN-313"));
}

#[test]
fn timed_task_wait_explains_conditional_handle_consumption() {
    let facts = analyze(
        r#"
fn worker() {
    sleep(20ms)
}

Task worker_task = run worker()
wait worker_task for 1ms on error {
    stop worker_task
    wait worker_task
}
"#,
    );

    let timed_wait = facts
        .iter()
        .find(|fact| fact.kind == AnalysisFactKind::TimedTaskWait)
        .expect("timed Task wait fact");
    assert_eq!(timed_wait.code, "SC-AN-314");
    assert_eq!(timed_wait.subject.as_deref(), Some("worker_task"));
    assert!(timed_wait.explanation.contains("handle remains live"));
}
