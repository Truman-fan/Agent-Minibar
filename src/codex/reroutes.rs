//! Observe only the official notification stream of an already running
//! shared Codex App Server. No prompts, replay, or preference changes.

use std::{
    collections::{HashMap, HashSet},
    io::{self, Cursor, Read, Write},
    path::Path,
    process::{Child, ChildStdin, Stdio},
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use tungstenite::{HandshakeError, Message, WebSocket, protocol::WebSocketConfig};

use crate::instances::ProviderId;

const REFRESH: Duration = Duration::from_secs(2);
const TIMEOUT: Duration = Duration::from_secs(3);
const MAX_THREADS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Running,
    Ended,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Event {
    thread_id: String,
    turn_id: String,
    from_model: String,
    to_model: String,
    reason: String,
}

#[derive(Clone)]
pub(crate) struct Notice {
    event: Event,
    pub(crate) title: String,
    pub(crate) phase: Phase,
    updated_at: Instant,
}

impl Notice {
    pub(crate) fn thread_id(&self) -> &str {
        &self.event.thread_id
    }
    pub(crate) fn models(&self) -> (&str, &str) {
        (&self.event.from_model, &self.event.to_model)
    }
    pub(crate) fn reason(&self) -> &str {
        &self.event.reason
    }
}

#[derive(Default)]
struct Feed {
    // One row per conversation. Seen events stay scoped to their turn so a
    // reconnect or retransmission does not turn the red light back on.
    notices: HashMap<String, (Notice, bool)>,
    attached: HashMap<String, String>,
}

impl Feed {
    fn handle(&mut self, message: &Value) -> bool {
        let Some(method) = message["method"].as_str() else {
            return false;
        };
        let params = &message["params"];
        let Some(thread_id) = params["threadId"].as_str() else {
            return false;
        };
        match method {
            "model/rerouted" => {
                let Some(title) = self.attached.get(thread_id) else {
                    return false;
                };
                let Ok(event) = Event::deserialize(params) else {
                    return false;
                };
                if event.turn_id.is_empty()
                    || event.from_model.is_empty()
                    || event.to_model.is_empty()
                    || event.from_model == event.to_model
                    || self
                        .notices
                        .get(thread_id)
                        .is_some_and(|(old, _)| old.event == event)
                {
                    return false;
                }
                if self.notices.len() >= MAX_THREADS
                    && !self.notices.contains_key(thread_id)
                    && let Some(oldest) = self
                        .notices
                        .values()
                        .min_by_key(|(notice, _)| notice.updated_at)
                        .map(|(notice, _)| notice.event.thread_id.clone())
                {
                    self.notices.remove(&oldest);
                }
                self.notices.insert(
                    thread_id.into(),
                    (
                        Notice {
                            event,
                            title: title.clone(),
                            phase: Phase::Running,
                            updated_at: Instant::now(),
                        },
                        false,
                    ),
                );
                true
            }
            "turn/completed" => {
                if let Some((notice, _)) = self.notices.get_mut(thread_id)
                    && params["turn"]["id"].as_str() == Some(&notice.event.turn_id)
                {
                    notice.phase = Phase::Ended;
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn disconnected(&mut self) -> bool {
        let mut changed = !self.attached.is_empty();
        self.attached.clear();
        for (notice, _) in self.notices.values_mut() {
            if notice.phase == Phase::Running {
                notice.phase = Phase::Unknown;
                changed = true;
            }
        }
        changed
    }
}

type SharedFeed = Arc<Mutex<Feed>>;
static FEEDS: LazyLock<Mutex<HashMap<ProviderId, SharedFeed>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn update_ui() {
    crate::taskbar::publish_reroute_update();
    crate::popup_window::send_command(crate::popup_window::PopupCommand::ReroutesChanged);
}

fn feed_for(provider: ProviderId) -> Option<SharedFeed> {
    FEEDS.lock().ok()?.get(&provider).cloned()
}

pub(crate) fn pending(provider: ProviderId) -> Vec<Notice> {
    let feed = feed_for(provider);
    let Some(feed) = feed else { return Vec::new() };
    let Ok(feed) = feed.lock() else {
        return Vec::new();
    };
    let mut notices = feed
        .notices
        .values()
        .filter(|(_, seen)| !seen)
        .map(|(notice, _)| notice.clone())
        .collect::<Vec<_>>();
    notices.sort_unstable_by_key(|notice| {
        (
            notice.phase != Phase::Running,
            std::cmp::Reverse(notice.updated_at),
        )
    });
    notices
}

pub(crate) fn pending_count(provider: ProviderId) -> usize {
    let feed = feed_for(provider);
    feed.as_ref()
        .and_then(|feed| feed.lock().ok())
        .map_or(0, |feed| {
            feed.notices.values().filter(|(_, seen)| !seen).count()
        })
}

pub(crate) fn acknowledge(provider: ProviderId, notice: &Notice) {
    if let Some(feed) = feed_for(provider)
        && let Ok(mut feed) = feed.lock()
        && let Some((current, seen)) = feed.notices.get_mut(notice.thread_id())
        && current.updated_at == notice.updated_at
    {
        *seen = true;
        update_ui();
    }
}

pub(crate) fn connected(provider: ProviderId) -> bool {
    feed_for(provider).is_some_and(|feed| feed.lock().is_ok_and(|feed| !feed.attached.is_empty()))
}

pub(super) struct Monitor {
    provider: ProviderId,
    stop: Arc<AtomicBool>,
    feed: SharedFeed,
}

impl Monitor {
    pub(super) fn start(provider: ProviderId, executable: &Path, home: &Path) -> Option<Self> {
        let socket_dir = home.join("app-server-control");
        if !socket_dir.is_dir() {
            return None;
        }
        let socket = socket_dir.join("app-server-control.sock");
        let feed = SharedFeed::default();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_feed = Arc::clone(&feed);
        let worker_stop = Arc::clone(&stop);
        let executable = executable.to_owned();
        let home = home.to_owned();
        FEEDS.lock().ok()?.insert(provider, Arc::clone(&feed));
        let monitor = Self {
            provider,
            stop,
            feed,
        };
        thread::Builder::new()
            .name("codex-reroute-monitor".into())
            .spawn(move || {
                while !worker_stop.load(Ordering::Acquire) {
                    let _ = run(&executable, &socket, &home, &worker_feed, &worker_stop);
                    if worker_feed.lock().is_ok_and(|mut feed| feed.disconnected()) {
                        update_ui();
                    }
                    for _ in 0..20 {
                        if worker_stop.load(Ordering::Acquire) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(250));
                    }
                }
            })
            .ok()?;
        Some(monitor)
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(mut feeds) = FEEDS.lock()
            && feeds
                .get(&self.provider)
                .is_some_and(|feed| Arc::ptr_eq(feed, &self.feed))
        {
            feeds.remove(&self.provider);
        }
        update_ui();
    }
}

fn run(
    executable: &Path,
    socket: &Path,
    home: &Path,
    feed: &SharedFeed,
    stop: &AtomicBool,
) -> Result<()> {
    let mut pipe = Proxy::open(executable, socket, home)?;
    let mut handshake = tungstenite::client::client_with_config(
        "ws://localhost/",
        pipe.take_io(),
        Some(WebSocketConfig::default().max_message_size(Some(2 * 1024 * 1024))),
    );
    let deadline = Instant::now() + TIMEOUT;
    let socket = loop {
        match handshake {
            Ok((socket, _)) => break socket,
            Err(HandshakeError::Interrupted(interrupted))
                if Instant::now() < deadline && !stop.load(Ordering::Acquire) =>
            {
                handshake = interrupted.handshake()
            }
            _ => bail!("Codex control socket handshake failed"),
        }
    };
    let mut rpc = Rpc {
        socket,
        feed,
        stop,
        id: 0,
        refresh_needed: false,
    };
    rpc.request(
        "initialize",
        json!({
            "clientInfo":{"name":"agent_minibar","version":env!("CARGO_PKG_VERSION")},
            "capabilities":{"optOutNotificationMethods":[
                "item/started", "item/completed", "item/agentMessage/delta",
                "item/reasoning/textDelta", "item/reasoning/summaryTextDelta",
                "item/commandExecution/outputDelta", "item/fileChange/outputDelta",
                "turn/diff/updated", "turn/plan/updated", "thread/tokenUsage/updated"
            ]}
        }),
    )?
    .context("Codex rejected the monitoring connection")?;
    rpc.send(json!({"method":"initialized"}))?;
    let mut next = Instant::now();
    while !stop.load(Ordering::Acquire) {
        if rpc.refresh_needed || Instant::now() >= next {
            rpc.refresh_needed = false;
            rpc.refresh_threads()?;
            next = Instant::now() + REFRESH;
        } else {
            rpc.receive()?;
        }
    }
    Ok(())
}

struct Rpc<'a> {
    socket: WebSocket<ProxyIo>,
    feed: &'a SharedFeed,
    stop: &'a AtomicBool,
    id: u64,
    refresh_needed: bool,
}

impl Rpc<'_> {
    fn send(&mut self, value: Value) -> Result<()> {
        Ok(self.socket.send(Message::text(value.to_string()))?)
    }

    fn receive(&mut self) -> Result<Option<Value>> {
        match self.socket.read() {
            Ok(Message::Text(text)) => {
                let message: Value = serde_json::from_str(&text)?;
                if message["method"] == "account/updated" {
                    if let Ok(mut feed) = self.feed.lock() {
                        *feed = Feed::default();
                    }
                    update_ui();
                    bail!("Codex login changed");
                }
                if message["method"] == "thread/status/changed"
                    && message["params"]["status"]["type"] == "active"
                {
                    self.refresh_needed = true;
                }
                if self.feed.lock().is_ok_and(|mut feed| feed.handle(&message)) {
                    update_ui();
                }
                Ok(Some(message))
            }
            Ok(Message::Close(_)) => bail!("Codex control socket closed"),
            Ok(_) => Ok(None),
            Err(tungstenite::Error::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Option<Value>> {
        self.id += 1;
        self.send(json!({"id": self.id, "method": method, "params": params}))?;
        let deadline = Instant::now() + TIMEOUT;
        while !self.stop.load(Ordering::Acquire) && Instant::now() < deadline {
            if let Some(message) = self.receive()?
                && message["id"].as_u64() == Some(self.id)
            {
                return Ok(message.get("result").cloned());
            }
        }
        bail!("Codex monitoring request timed out or stopped")
    }

    fn refresh_threads(&mut self) -> Result<()> {
        let list = self
            .request("thread/loaded/list", json!({}))?
            .context("Codex did not report loaded threads")?;
        let ids = list["data"]
            .as_array()
            .context("invalid loaded thread list")?;
        let loaded = ids
            .iter()
            .filter_map(Value::as_str)
            .filter(|&id| safe_thread_id(id))
            .take(MAX_THREADS)
            .collect::<HashSet<_>>();
        let attached = self
            .feed
            .lock()
            .map(|feed| feed.attached.keys().cloned().collect::<HashSet<_>>())
            .unwrap_or_default();
        if let Ok(mut feed) = self.feed.lock() {
            feed.attached.retain(|id, _| loaded.contains(id.as_str()));
        }
        for id in loaded {
            if attached.contains(id) {
                continue;
            }
            let Some(read) =
                self.request("thread/read", json!({"threadId":id,"includeTurns":false}))?
            else {
                continue;
            };
            let metadata = &read["thread"];
            if metadata["status"]["type"] != "active"
                || !metadata["source"].is_string()
                || metadata["ephemeral"] == true
            {
                continue;
            }
            let Some(response) =
                self.request("thread/resume", json!({"threadId":id,"excludeTurns":true}))?
            else {
                continue;
            };
            if response["thread"]["id"].as_str() == Some(id)
                && let Ok(mut feed) = self.feed.lock()
            {
                feed.attached
                    .insert(id.into(), metadata["name"].as_str().unwrap_or(id).into());
                update_ui();
            }
        }
        Ok(())
    }
}

pub(crate) fn safe_thread_id(id: &str) -> bool {
    !id.is_empty()
        && id != "new"
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
}

struct Proxy {
    child: Child,
    io: Option<ProxyIo>,
}

impl Proxy {
    fn open(executable: &Path, socket: &Path, home: &Path) -> Result<Self> {
        let prepared = crate::discovery::prepare(executable)?;
        let mut command = super::command_for_codex(
            &prepared.path,
            &["app-server", "proxy", "--sock"],
            Some(home),
        );
        let child = command
            .arg(socket)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut proxy = Self { child, io: None };
        let mut stdout = proxy
            .child
            .stdout
            .take()
            .context("missing Codex proxy output")?;
        let stdin = proxy
            .child
            .stdin
            .take()
            .context("missing Codex proxy input")?;
        let (tx, rx) = mpsc::sync_channel(8);
        thread::Builder::new()
            .name("codex-control-socket-read".into())
            .spawn(move || {
                let mut bytes = [0; 8192];
                while let Ok(size @ 1..) = stdout.read(&mut bytes) {
                    if tx.send(bytes[..size].to_vec()).is_err() {
                        break;
                    }
                }
            })?;
        proxy.io = Some(ProxyIo {
            stdin,
            rx,
            buffer: Cursor::new(Vec::new()),
        });
        Ok(proxy)
    }

    fn take_io(&mut self) -> ProxyIo {
        self.io.take().expect("one WebSocket session per proxy")
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        super::terminate(&mut self.child);
    }
}

struct ProxyIo {
    stdin: ChildStdin,
    rx: Receiver<Vec<u8>>,
    buffer: Cursor<Vec<u8>>,
}

impl Read for ProxyIo {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let available = self.buffer.read(buffer)?;
        if available != 0 || buffer.is_empty() {
            return Ok(available);
        }
        match self.rx.recv_timeout(Duration::from_millis(150)) {
            Ok(bytes) => {
                self.buffer = Cursor::new(bytes);
                self.buffer.read(buffer)
            }
            Err(RecvTimeoutError::Timeout) => Err(io::ErrorKind::WouldBlock.into()),
            Err(RecvTimeoutError::Disconnected) => Ok(0),
        }
    }
}

impl Write for ProxyIo {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.stdin.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdin.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reroute(thread_id: &str, turn_id: &str) -> Value {
        json!({"method":"model/rerouted","params":{
            "threadId":thread_id, "turnId":turn_id, "fromModel":"requested",
            "toModel":"routed", "reason":"highRiskCyberActivity",
        }})
    }

    #[test]
    fn official_routes_are_counted_per_attached_conversation_and_survive_turn_end() {
        let mut feed = Feed::default();
        feed.attached.insert("root-a".into(), "A".into());
        feed.attached.insert("root-b".into(), "B".into());
        assert!(!feed.handle(&reroute("unattached", "turn-a")));
        assert!(
            !feed.handle(&json!({"method":"thread/settings/updated","params":{
                "threadId":"root-a", "model":"different",
            }}))
        );
        assert!(feed.handle(&reroute("root-a", "turn-a")));
        assert!(!feed.handle(&reroute("root-a", "turn-a")));
        assert!(feed.handle(&reroute("root-b", "turn-b")));
        assert_eq!(feed.notices.len(), 2);
        assert!(!feed.handle(&json!({"method":"turn/completed","params":{
            "threadId":"root-a", "turn":{"id":"older-turn"},
        }})));
        assert!(feed.handle(&json!({"method":"turn/completed","params":{
            "threadId":"root-a", "turn":{"id":"turn-a"},
        }})));
        assert_eq!(feed.notices["root-a"].0.phase, Phase::Ended);
        assert!(!feed.notices["root-a"].1);
        feed.disconnected();
        assert_eq!(feed.notices["root-a"].0.phase, Phase::Ended);
        assert_eq!(feed.notices["root-b"].0.phase, Phase::Unknown);
    }

    #[test]
    fn unchanged_models_and_invalid_links_do_not_create_alerts() {
        let mut feed = Feed::default();
        feed.attached.insert("root-a".into(), "A".into());
        let mut event = reroute("root-a", "turn-a");
        event["params"]["toModel"] = json!("requested");
        assert!(!feed.handle(&event));
        assert!(safe_thread_id("01a00000-1234-abcd-abcd-000000000000"));
        assert!(!safe_thread_id("new?prompt=start"));
    }
}
