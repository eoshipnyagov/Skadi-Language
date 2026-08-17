use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::actions::{ActionError, DebugPrepared, FailureSource};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugCommand {
    Continue,
    Step,
    Quit,
}

impl DebugCommand {
    fn wire_value(self) -> &'static str {
        match self {
            Self::Continue => "CONTINUE\n",
            Self::Step => "STEP\n",
            Self::Quit => "QUIT\n",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugLocal {
    pub name: String,
    pub type_name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugFrame {
    pub function: String,
    pub statement_id: String,
    pub locals: Vec<DebugLocal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugStop {
    pub thread_id: u64,
    pub statement_id: String,
    pub frames: Vec<DebugFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugEvent {
    Started,
    Stopped(DebugStop),
    Stdout(String),
    Stderr(String),
    Exited { status: String, success: bool },
    Error(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugIoMode {
    Terminal,
    Captured,
}

pub struct DebugSession {
    events: Receiver<DebugEvent>,
    commands: Sender<DebugCommand>,
    child: Arc<Mutex<Child>>,
}

impl DebugSession {
    pub fn recv(&self) -> Result<DebugEvent, mpsc::RecvError> {
        self.events.recv()
    }

    pub fn try_recv(&self) -> Result<DebugEvent, TryRecvError> {
        self.events.try_recv()
    }

    pub fn command(&self, command: DebugCommand) -> Result<(), String> {
        self.commands
            .send(command)
            .map_err(|_| "debug session is no longer running".to_string())
    }

    pub fn terminate(&self) -> Result<(), String> {
        self.child
            .lock()
            .map_err(|_| "debug process handle is unavailable".to_string())?
            .kill()
            .map_err(|error| format!("failed to terminate debug process: {error}"))
    }
}

impl Drop for DebugSession {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock()
            && child.try_wait().ok().flatten().is_none()
        {
            let _ = child.kill();
        }
    }
}

pub fn start_debug_session(
    prepared: &DebugPrepared,
    io_mode: DebugIoMode,
) -> Result<DebugSession, ActionError> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|error| {
        ActionError::new(
            FailureSource::Io,
            format!("debug transport error: cannot bind loopback socket: {error}"),
        )
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        ActionError::new(
            FailureSource::Io,
            format!("debug transport error: cannot configure loopback socket: {error}"),
        )
    })?;
    let port = listener
        .local_addr()
        .map_err(|error| {
            ActionError::new(
                FailureSource::Io,
                format!("debug transport error: cannot inspect loopback socket: {error}"),
            )
        })?
        .port();
    let breakpoint_ids = prepared
        .breakpoints
        .iter()
        .map(|breakpoint| breakpoint.statement_id.as_str())
        .collect::<Vec<_>>()
        .join(",");

    let mut command = Command::new(&prepared.build.exe_path);
    command
        .current_dir(&prepared.build.project.cwd)
        .args(&prepared.program_args)
        .env("SKADI_DEBUG_PORT", port.to_string())
        .env("SKADI_DEBUG_BREAKPOINTS", breakpoint_ids)
        .env(
            "SKADI_DEBUG_STEP",
            if prepared.starts_paused { "1" } else { "0" },
        );
    match io_mode {
        DebugIoMode::Terminal => {
            command
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
        }
        DebugIoMode::Captured => {
            command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        }
    }
    let mut child = command.spawn().map_err(|error| {
        ActionError::new(
            FailureSource::Runtime,
            format!(
                "debug execution error: failed to run {}: {error}",
                prepared.build.exe_path.display()
            ),
        )
    })?;

    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    if io_mode == DebugIoMode::Captured {
        forward_output(child.stdout.take(), event_tx.clone(), false);
        forward_output(child.stderr.take(), event_tx.clone(), true);
    }
    let child = Arc::new(Mutex::new(child));
    let _ = event_tx.send(DebugEvent::Started);
    let controller_child = Arc::clone(&child);
    thread::spawn(move || run_session(listener, controller_child, event_tx, command_rx));

    Ok(DebugSession {
        events: event_rx,
        commands: command_tx,
        child,
    })
}

fn forward_output<R>(reader: Option<R>, events: Sender<DebugEvent>, stderr: bool)
where
    R: std::io::Read + Send + 'static,
{
    let Some(reader) = reader else {
        return;
    };
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else {
                break;
            };
            let event = if stderr {
                DebugEvent::Stderr(line)
            } else {
                DebugEvent::Stdout(line)
            };
            if events.send(event).is_err() {
                break;
            }
        }
    });
}

fn run_session(
    listener: TcpListener,
    child: Arc<Mutex<Child>>,
    events: Sender<DebugEvent>,
    commands: Receiver<DebugCommand>,
) {
    let stream = loop {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(error) = stream.set_nonblocking(false) {
                    let _ = events.send(DebugEvent::Error(format!(
                        "debug transport error: cannot configure runtime connection: {error}"
                    )));
                    break None;
                }
                break Some(stream);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => {
                let _ = events.send(DebugEvent::Error(format!(
                    "debug transport error: accepting runtime connection failed: {error}"
                )));
                break None;
            }
        }
        let child_status = match child.lock() {
            Ok(mut child) => child.try_wait(),
            Err(_) => {
                let _ = events.send(DebugEvent::Error(
                    "debug process handle is unavailable".to_string(),
                ));
                return;
            }
        };
        match child_status {
            Ok(Some(status)) => {
                let _ = events.send(DebugEvent::Exited {
                    status: status.to_string(),
                    success: status.success(),
                });
                return;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let _ = events.send(DebugEvent::Error(format!(
                    "debug execution error: failed to inspect process: {error}"
                )));
                return;
            }
        }
    };

    if let Some(stream) = stream
        && let Err(error) = process_protocol(stream, &events, &commands)
    {
        let _ = events.send(DebugEvent::Error(error));
        if let Ok(mut child) = child.lock() {
            let _ = child.kill();
        }
    }
    loop {
        let child_status = match child.lock() {
            Ok(mut child) => child.try_wait(),
            Err(_) => {
                let _ = events.send(DebugEvent::Error(
                    "debug process handle is unavailable".to_string(),
                ));
                return;
            }
        };
        match child_status {
            Ok(Some(status)) => {
                let _ = events.send(DebugEvent::Exited {
                    status: status.to_string(),
                    success: status.success(),
                });
                return;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let _ = events.send(DebugEvent::Error(format!(
                    "debug execution error: failed to inspect process: {error}"
                )));
                return;
            }
        }
    }
}

fn process_protocol(
    stream: TcpStream,
    events: &Sender<DebugEvent>,
    commands: &Receiver<DebugCommand>,
) -> Result<(), String> {
    let mut writer = stream
        .try_clone()
        .map_err(|error| format!("debug transport error: socket clone failed: {error}"))?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        let read = match reader.read_line(&mut line) {
            Ok(read) => read,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::BrokenPipe
                ) =>
            {
                return Ok(());
            }
            Err(error) => {
                return Err(format!("debug transport error: read failed: {error}"));
            }
        };
        if read == 0 {
            return Ok(());
        }
        if !line.starts_with("STOP\t") {
            continue;
        }
        let stop = read_stop_packet(line.trim_end_matches(['\r', '\n']), &mut reader)?;
        events
            .send(DebugEvent::Stopped(stop))
            .map_err(|_| "debug client disconnected".to_string())?;
        let command = commands
            .recv()
            .map_err(|_| "debug client disconnected while program was stopped".to_string())?;
        writer
            .write_all(command.wire_value().as_bytes())
            .and_then(|_| writer.flush())
            .map_err(|error| format!("debug transport error: command write failed: {error}"))?;
    }
}

fn read_stop_packet(stop_line: &str, reader: &mut impl BufRead) -> Result<DebugStop, String> {
    let mut stop_parts = stop_line.split('\t');
    let _ = stop_parts.next();
    let thread_id = stop_parts
        .next()
        .ok_or_else(|| "debug protocol error: STOP has no thread id".to_string())?
        .parse::<u64>()
        .map_err(|_| "debug protocol error: invalid thread id".to_string())?;
    let statement_id = decode_hex_field(
        stop_parts
            .next()
            .ok_or_else(|| "debug protocol error: STOP has no statement id".to_string())?,
    )?;
    let mut frames = Vec::<DebugFrame>::new();
    let mut line = String::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|error| format!("debug transport error: packet read failed: {error}"))?
            == 0
        {
            return Err(
                "debug protocol error: runtime disconnected inside stop packet".to_string(),
            );
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line == "END" {
            break;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.first().copied() {
            Some("FRAME") if fields.len() == 4 => {
                let offset = parse_frame_offset(fields[1])?;
                while frames.len() <= offset {
                    frames.push(DebugFrame {
                        function: String::new(),
                        statement_id: String::new(),
                        locals: Vec::new(),
                    });
                }
                frames[offset].function = decode_hex_field(fields[2])?;
                frames[offset].statement_id = decode_hex_field(fields[3])?;
            }
            Some("LOCAL") if fields.len() == 5 => {
                let offset = parse_frame_offset(fields[1])?;
                while frames.len() <= offset {
                    frames.push(DebugFrame {
                        function: String::new(),
                        statement_id: String::new(),
                        locals: Vec::new(),
                    });
                }
                frames[offset].locals.push(DebugLocal {
                    name: decode_hex_field(fields[2])?,
                    type_name: decode_hex_field(fields[3])?,
                    value: decode_hex_field(fields[4])?,
                });
            }
            _ => {
                return Err(format!(
                    "debug protocol error: malformed packet line '{line}'"
                ));
            }
        }
    }
    Ok(DebugStop {
        thread_id,
        statement_id,
        frames,
    })
}

fn parse_frame_offset(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("debug protocol error: invalid frame offset '{value}'"))
}

fn decode_hex_field(value: &str) -> Result<String, String> {
    if !value.len().is_multiple_of(2) {
        return Err("debug protocol error: odd-length text field".to_string());
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| "debug protocol error: invalid text field".to_string())?;
            u8::from_str_radix(text, 16)
                .map_err(|_| "debug protocol error: invalid hexadecimal text field".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes)
        .map_err(|_| "debug protocol error: text field is not valid UTF-8".to_string())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{DebugCommand, DebugEvent, DebugIoMode, read_stop_packet, start_debug_session};
    use crate::actions::{BuildOptions, DebugOptions, prepare_debug_at};
    use crate::project::init_project;
    use crate::targets::detect_compiler;

    #[test]
    fn parses_stop_packet_with_frames_and_locals() {
        let mut packet = Cursor::new(
            b"FRAME\t0\t776f726b6572\t534b2d53544d5440313a312331\n\
LOCAL\t0\t76616c7565\t496e74\t3432\n\
FRAME\t1\t3c6d61696e3e\t\n\
END\n",
        );
        let stop = read_stop_packet("STOP\t7\t534b2d53544d5440313a312331", &mut packet)
            .expect("packet should parse");
        assert_eq!(stop.thread_id, 7);
        assert_eq!(stop.frames[0].function, "worker");
        assert_eq!(stop.frames[0].locals[0].name, "value");
        assert_eq!(stop.frames[0].locals[0].value, "42");
        assert_eq!(stop.frames[1].function, "<main>");
    }

    #[test]
    fn captured_session_stops_continues_and_collects_program_output() {
        if !["gcc", "clang", "cc", "cl"]
            .iter()
            .any(|compiler| detect_compiler(compiler))
        {
            return;
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("skadi_debug_session_{stamp}"));
        init_project(&root).expect("project should initialize");
        let prepared = prepare_debug_at(
            &root,
            &DebugOptions {
                build: BuildOptions {
                    target: "host".to_string(),
                    cc: None,
                },
                breakpoints: Vec::new(),
                program_args: Vec::new(),
            },
        )
        .expect("debug build should prepare");
        let session = start_debug_session(&prepared, DebugIoMode::Captured)
            .expect("captured session should start");
        let mut saw_stop = false;
        let mut stdout = Vec::new();
        let mut exited = false;
        for _ in 0..20 {
            let event = session
                .events
                .recv_timeout(Duration::from_secs(2))
                .expect("debug event should arrive");
            match event {
                DebugEvent::Started => {}
                DebugEvent::Stopped(stop) => {
                    saw_stop = true;
                    assert_eq!(stop.frames[0].function, "<main>");
                    session
                        .command(DebugCommand::Continue)
                        .expect("continue should be accepted");
                }
                DebugEvent::Stdout(line) => stdout.push(line),
                DebugEvent::Stderr(line) => panic!("unexpected stderr: {line}"),
                DebugEvent::Exited { success, .. } => {
                    assert!(success);
                    exited = true;
                    break;
                }
                DebugEvent::Error(message) => panic!("debug session failed: {message}"),
            }
        }
        assert!(saw_stop);
        assert!(exited);
        assert!(stdout.iter().any(|line| line == "Hello from Skadi"));
        let _ = std::fs::remove_dir_all(root);
    }
}
