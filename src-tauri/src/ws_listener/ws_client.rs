use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tungstenite::client::IntoClientRequest;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Connector, Message, WebSocket};

use crate::lcu_connector::{
    get_cached_or_discover_lcu_credentials, invalidate_lcu_credentials, is_lcu_connected,
    lcu_tls_client_config,
};
use crate::logging;

const WS_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const WS_HANDSHAKE_IO_TIMEOUT: Duration = Duration::from_secs(3);
const WS_READ_TIMEOUT: Duration = Duration::from_millis(250);
const WS_WRITE_TIMEOUT: Duration = Duration::from_secs(2);
const WS_RECONNECT_BACKOFF: Duration = Duration::from_millis(1500);
const WS_SLEEP_SLICE: Duration = Duration::from_millis(150);

type LcuWsStream = WebSocket<MaybeTlsStream<TcpStream>>;

pub(super) use super::ws_protocol::{
    LcuWsEvent, build_subscribe_frame, endpoint_to_json_api_topic, parse_lcu_ws_event,
};

pub(super) type WsEventCallback = Arc<dyn Fn(&LcuWsEvent) + Send + Sync>;

/// Handle to the dedicated LCU WebSocket listener thread.
pub(super) struct LcuWsHandle {
    stop_flag: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl LcuWsHandle {
    pub(super) fn spawn(subscriptions: Vec<(String, WsEventCallback)>) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop_flag);
        let handle = std::thread::spawn(move || {
            run_ws_loop(&thread_stop, subscriptions);
        });
        Self { stop_flag, handle }
    }

    pub(super) fn abort(self) {
        self.stop_flag.store(true, Ordering::Release);
    }

    pub(super) fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

fn sleep_interruptible(stop_flag: &AtomicBool, total: Duration) {
    let start = Instant::now();
    while !stop_flag.load(Ordering::Acquire) && is_lcu_connected() {
        let elapsed = start.elapsed();
        if elapsed >= total {
            break;
        }
        std::thread::sleep((total - elapsed).min(WS_SLEEP_SLICE));
    }
}

fn configure_socket_timeouts(
    stream: &LcuWsStream,
    read_timeout: Duration,
    write_timeout: Duration,
) -> Result<(), String> {
    if let MaybeTlsStream::Rustls(tls_stream) = stream.get_ref() {
        tls_stream
            .sock
            .set_read_timeout(Some(read_timeout))
            .map_err(|err| format!("Failed to set WS read timeout: {err}"))?;
        tls_stream
            .sock
            .set_write_timeout(Some(write_timeout))
            .map_err(|err| format!("Failed to set WS write timeout: {err}"))?;
    }
    Ok(())
}

fn connect_and_subscribe(
    tls_config: &Arc<rustls::ClientConfig>,
    topics: &[String],
) -> Result<LcuWsStream, String> {
    let creds = get_cached_or_discover_lcu_credentials()?;
    let ws_url = format!("wss://{}", creds.addr);
    let mut request = ws_url
        .into_client_request()
        .map_err(|err| format!("Invalid LCU WebSocket request: {err}"))?;
    request
        .headers_mut()
        .insert(http::header::AUTHORIZATION, creds.auth_header);

    let tcp = TcpStream::connect_timeout(&SocketAddr::V4(creds.addr), WS_CONNECT_TIMEOUT)
        .map_err(|err| format!("LCU WebSocket TCP connect failed: {err}"))?;
    let _ = tcp.set_read_timeout(Some(WS_HANDSHAKE_IO_TIMEOUT));
    let _ = tcp.set_write_timeout(Some(WS_HANDSHAKE_IO_TIMEOUT));

    let (mut stream, _) = tungstenite::client_tls_with_config(
        request,
        tcp,
        None,
        Some(Connector::Rustls(Arc::clone(tls_config))),
    )
    .map_err(|err| format!("LCU WebSocket TLS handshake failed: {err}"))?;

    configure_socket_timeouts(&stream, WS_READ_TIMEOUT, WS_WRITE_TIMEOUT)?;

    for topic in topics {
        let frame = build_subscribe_frame(topic);
        stream
            .send(Message::Text(frame.into()))
            .map_err(|err| format!("LCU WebSocket subscribe failed for {topic}: {err}"))?;
    }

    Ok(stream)
}

fn dispatch_text_frame(raw: &str, callbacks_by_topic: &HashMap<String, Vec<WsEventCallback>>) {
    let Some(event) = parse_lcu_ws_event(raw) else {
        return;
    };
    if let Some(callbacks) = callbacks_by_topic.get(&event.topic) {
        for cb in callbacks {
            cb(&event);
        }
    }
}

fn drive_connected_stream(
    stream: &mut LcuWsStream,
    stop_flag: &AtomicBool,
    callbacks_by_topic: &HashMap<String, Vec<WsEventCallback>>,
) -> Result<(), String> {
    loop {
        if stop_flag.load(Ordering::Acquire) || !is_lcu_connected() {
            let _ = stream.close(None);
            let _ = stream.flush();
            return Ok(());
        }

        match stream.read() {
            Ok(Message::Text(text)) => dispatch_text_frame(text.as_str(), callbacks_by_topic),
            Ok(Message::Binary(bytes)) => {
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    dispatch_text_frame(text, callbacks_by_topic);
                }
            }
            Ok(Message::Ping(_)) => {
                // Tungstenite automatically queues the Pong frame; flush eagerly without duplicating.
                let _ = stream.flush();
            }
            Ok(Message::Pong(_) | Message::Frame(_)) => {}
            Ok(Message::Close(_)) => return Err("LCU WebSocket closed by peer".to_string()),
            Err(tungstenite::Error::Io(ref io_err))
                if matches!(io_err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(err) => return Err(format!("LCU WebSocket read error: {err}")),
        }
    }
}

fn run_ws_loop(stop_flag: &AtomicBool, subscriptions: Vec<(String, WsEventCallback)>) {
    let tls_config = match lcu_tls_client_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            logging::log_warn(
                "ws.listener",
                format!("event=ws_tls_init_failed err={err}"),
            );
            return;
        }
    };

    let mut topics = Vec::new();
    let mut callbacks_by_topic: HashMap<String, Vec<WsEventCallback>> = HashMap::new();
    for (topic, cb) in subscriptions {
        let entry = callbacks_by_topic.entry(topic.clone()).or_default();
        if entry.is_empty() {
            topics.push(topic);
        }
        entry.push(cb);
    }

    while !stop_flag.load(Ordering::Acquire) {
        if !is_lcu_connected() {
            logging::log_info("ws.listener", "event=ws_stopped reason=lcu_disconnected");
            break;
        }

        match connect_and_subscribe(&tls_config, &topics) {
            Ok(mut stream) => {
                logging::log_info("ws.listener", "event=ws_connected");
                if let Err(err) = drive_connected_stream(&mut stream, stop_flag, &callbacks_by_topic)
                {
                    if stop_flag.load(Ordering::Acquire) || !is_lcu_connected() {
                        logging::log_info("ws.listener", "event=ws_stopped reason=lcu_disconnected");
                        break;
                    }
                    invalidate_lcu_credentials();
                    logging::log_warn(
                        "ws.listener",
                        format!("event=ws_reconnect_scheduled err={err}"),
                    );
                    sleep_interruptible(stop_flag, WS_RECONNECT_BACKOFF);
                }
            }
            Err(err) => {
                if stop_flag.load(Ordering::Acquire) || !is_lcu_connected() {
                    logging::log_info("ws.listener", "event=ws_stopped reason=lcu_disconnected");
                    break;
                }
                invalidate_lcu_credentials();
                logging::log_warn(
                    "ws.listener",
                    format!("event=ws_reconnect_scheduled err={err}"),
                );
                sleep_interruptible(stop_flag, WS_RECONNECT_BACKOFF);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sleep_interruptible_exits_immediately_when_stopped() {
        let stop_flag = AtomicBool::new(true);
        let started = Instant::now();
        sleep_interruptible(&stop_flag, Duration::from_secs(5));
        assert!(started.elapsed() < Duration::from_millis(100));
    }
}
