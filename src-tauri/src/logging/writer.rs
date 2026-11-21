use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use super::{LogLevel, format_log_line_at};

pub const LOG_CHANNEL_CAPACITY: usize = 256;
pub const LOG_BUF_CAPACITY: usize = 8 * 1024;
pub const LOG_MAX_FILE_BYTES: u64 = 1024 * 1024;
pub const LOG_IDLE_FLUSH_MS: u64 = 500;
pub const LOG_FILE_NAME: &str = "schumazgg.log";
pub const LOG_BACKUP_FILE_NAME: &str = "schumazgg.log.1";

#[cfg(windows)]
const WIN_FILE_SHARE_ALL: u32 = 0x0000_0001 | 0x0000_0002 | 0x0000_0004;

pub(crate) enum LogCommand {
    Record { level: LogLevel, line: String },
    Flush(SyncSender<()>),
    Shutdown(SyncSender<()>),
}

/// Non-blocking handle communicating with the dedicated `"schumaz-logger"` actor thread.
pub struct LogWriterHandle {
    tx: SyncSender<LogCommand>,
    log_path: PathBuf,
    pending_dropped: Arc<AtomicU64>,
    dropped_total: Arc<AtomicU64>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl LogWriterHandle {
    pub fn start(logs_dir: PathBuf) -> Result<Self, String> {
        Self::start_with_config(logs_dir, LOG_MAX_FILE_BYTES, LOG_CHANNEL_CAPACITY)
    }

    pub fn start_with_config(
        logs_dir: PathBuf,
        max_file_bytes: u64,
        channel_capacity: usize,
    ) -> Result<Self, String> {
        fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to create logs dir {}: {e}", logs_dir.display()))?;

        let log_path = logs_dir.join(LOG_FILE_NAME);
        let backup_path = logs_dir.join(LOG_BACKUP_FILE_NAME);
        let (tx, rx) = mpsc::sync_channel::<LogCommand>(channel_capacity.max(1));
        let pending_dropped = Arc::new(AtomicU64::new(0));
        let dropped_total = Arc::new(AtomicU64::new(0));

        let actor_log_path = log_path.clone();
        let actor_pending_dropped = Arc::clone(&pending_dropped);
        let join = thread::Builder::new()
            .name("schumaz-logger".to_string())
            .spawn(move || {
                run_logger_actor(
                    rx,
                    actor_log_path,
                    backup_path,
                    max_file_bytes.max(128),
                    actor_pending_dropped,
                );
            })
            .map_err(|e| format!("Failed to spawn schumaz-logger thread: {e}"))?;

        Ok(Self {
            tx,
            log_path,
            pending_dropped,
            dropped_total,
            join_handle: Mutex::new(Some(join)),
        })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn dropped_total(&self) -> u64 {
        self.dropped_total.load(Ordering::Relaxed)
    }

    /// Enqueues a log line without ever blocking the caller thread.
    /// Returns `true` when enqueued, or `false` when dropped due to channel saturation.
    pub fn try_send(&self, level: LogLevel, line: String) -> bool {
        match self.tx.try_send(LogCommand::Record { level, line }) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.pending_dropped.fetch_add(1, Ordering::Relaxed);
                self.dropped_total.fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    /// Requests a synchronous flush of all currently buffered log records (bounded timeout).
    pub fn flush(&self) {
        let (ack_tx, ack_rx) = mpsc::sync_channel::<()>(1);
        if self.tx.send(LogCommand::Flush(ack_tx)).is_ok() {
            let _ = ack_rx.recv_timeout(Duration::from_secs(2));
        }
    }

    /// Flushes remaining records and stops the background logger thread cleanly.
    pub fn shutdown(&self) {
        let (ack_tx, ack_rx) = mpsc::sync_channel::<()>(1);
        if self.tx.send(LogCommand::Shutdown(ack_tx)).is_ok() {
            let _ = ack_rx.recv_timeout(Duration::from_secs(2));
        }
        if let Ok(mut guard) = self.join_handle.lock()
            && let Some(handle) = guard.take()
        {
            let _ = handle.join();
        }
    }
}

/// Opens a log file with read/write/delete sharing on Windows so users can inspect or copy
/// `schumazgg.log` while the application is running.
pub(crate) fn open_shared_log_file(path: &Path, truncate: bool) -> std::io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.create(true)
        .write(true)
        .truncate(truncate)
        .append(!truncate);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(WIN_FILE_SHARE_ALL);
    }
    opts.open(path)
}

/// Synchronously appends a single sanitized crash line directly to disk before `panic = "abort"`.
pub fn write_panic_sync(log_path: &Path, line: &str) {
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = open_shared_log_file(log_path, false) {
        let _ = file.write_all(line.as_bytes());
        let _ = file.write_all(b"\n");
        let _ = file.flush();
    }
}

struct RotatingSink {
    log_path: PathBuf,
    backup_path: PathBuf,
    max_file_bytes: u64,
    writer: Option<BufWriter<File>>,
    current_bytes: u64,
    dirty: bool,
}

impl RotatingSink {
    fn open(log_path: PathBuf, backup_path: PathBuf, max_file_bytes: u64) -> Self {
        let (writer, current_bytes) = match open_shared_log_file(&log_path, false) {
            Ok(file) => {
                let len = file.metadata().map(|m| m.len()).unwrap_or(0);
                (Some(BufWriter::with_capacity(LOG_BUF_CAPACITY, file)), len)
            }
            Err(_) => (None, 0),
        };
        Self {
            log_path,
            backup_path,
            max_file_bytes,
            writer,
            current_bytes,
            dirty: false,
        }
    }

    fn ensure_open(&mut self) {
        if self.writer.is_some() {
            return;
        }
        if let Some(parent) = self.log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(file) = open_shared_log_file(&self.log_path, false) {
            self.current_bytes = file.metadata().map(|m| m.len()).unwrap_or(0);
            self.writer = Some(BufWriter::with_capacity(LOG_BUF_CAPACITY, file));
        }
    }

    fn rotate(&mut self) {
        if let Some(mut w) = self.writer.take() {
            let _ = w.flush();
        }
        self.dirty = false;

        let _ = fs::remove_file(&self.backup_path);
        if fs::rename(&self.log_path, &self.backup_path).is_err() {
            // Windows fallback if an external viewer briefly holds a non-delete-shared handle.
            let _ = fs::copy(&self.log_path, &self.backup_path);
        }

        if let Ok(file) = open_shared_log_file(&self.log_path, true) {
            self.writer = Some(BufWriter::with_capacity(LOG_BUF_CAPACITY, file));
            self.current_bytes = 0;
        }
    }

    fn write_line(&mut self, line: &str) {
        self.ensure_open();
        let line_bytes = line.len() as u64 + 1;
        if self.current_bytes > 0 && self.current_bytes + line_bytes > self.max_file_bytes {
            self.rotate();
        }
        if let Some(writer) = self.writer.as_mut()
            && writer.write_all(line.as_bytes()).is_ok()
            && writer.write_all(b"\n").is_ok()
        {
            self.current_bytes = self.current_bytes.saturating_add(line_bytes);
            self.dirty = true;
        }
    }

    fn flush(&mut self) {
        if let Some(writer) = self.writer.as_mut() {
            let _ = writer.flush();
        }
        self.dirty = false;
    }
}

fn emit_consolidated_drop_notice(sink: &mut RotatingSink, pending_dropped: &AtomicU64) -> bool {
    let dropped = pending_dropped.swap(0, Ordering::Relaxed);
    if dropped == 0 {
        return false;
    }
    let notice = format_log_line_at(
        SystemTime::now(),
        LogLevel::Warn,
        "logging",
        &format!("event=dropped_events count={dropped}"),
    );
    sink.write_line(&notice);
    true
}

#[derive(Default)]
struct BatchState {
    immediate_flush: bool,
    should_exit: bool,
    flush_acks: Vec<SyncSender<()>>,
    shutdown_ack: Option<SyncSender<()>>,
}

fn process_command(
    cmd: LogCommand,
    sink: &mut RotatingSink,
    pending_dropped: &AtomicU64,
    batch: &mut BatchState,
) {
    match cmd {
        LogCommand::Record { level, line } => {
            if emit_consolidated_drop_notice(sink, pending_dropped) {
                batch.immediate_flush = true;
            }
            sink.write_line(&line);
            if matches!(level, LogLevel::Warn | LogLevel::Error) {
                batch.immediate_flush = true;
            }
        }
        LogCommand::Flush(ack) => {
            let _ = emit_consolidated_drop_notice(sink, pending_dropped);
            batch.immediate_flush = true;
            batch.flush_acks.push(ack);
        }
        LogCommand::Shutdown(ack) => {
            let _ = emit_consolidated_drop_notice(sink, pending_dropped);
            batch.immediate_flush = true;
            batch.should_exit = true;
            batch.shutdown_ack = Some(ack);
        }
    }
}

fn run_logger_actor(
    rx: mpsc::Receiver<LogCommand>,
    log_path: PathBuf,
    backup_path: PathBuf,
    max_file_bytes: u64,
    pending_dropped: Arc<AtomicU64>,
) {
    let mut sink = RotatingSink::open(log_path, backup_path, max_file_bytes);
    let idle_timeout = Duration::from_millis(LOG_IDLE_FLUSH_MS);

    loop {
        let first_cmd = if sink.dirty {
            match rx.recv_timeout(idle_timeout) {
                Ok(cmd) => cmd,
                Err(RecvTimeoutError::Timeout) => {
                    let _ = emit_consolidated_drop_notice(&mut sink, &pending_dropped);
                    sink.flush();
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(cmd) => cmd,
                Err(_) => break,
            }
        };

        let mut batch = BatchState::default();
        process_command(first_cmd, &mut sink, &pending_dropped, &mut batch);
        while !batch.should_exit
            && let Ok(next_cmd) = rx.try_recv()
        {
            process_command(next_cmd, &mut sink, &pending_dropped, &mut batch);
        }

        if batch.immediate_flush {
            sink.flush();
            for ack in batch.flush_acks.drain(..) {
                let _ = ack.send(());
            }
        }

        if batch.should_exit {
            if let Some(ack) = batch.shutdown_ack {
                let _ = ack.send(());
            }
            break;
        }
    }

    let _ = emit_consolidated_drop_notice(&mut sink, &pending_dropped);
    sink.flush();
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod tests;
