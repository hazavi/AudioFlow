pub mod decision;
use crate::{
    config::Storage,
    models::*,
    native::{Platform, Routes},
};
use decision::{decide, matching_rule, select_monitor, select_window, Decision};
use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

pub enum Mutation {
    Settings(Settings),
    Assignment(String, Option<String>),
    Rule(Rule),
    DeleteRule(String),
    Import(Config),
    Reset,
    ClearActivity,
}
pub enum Message {
    Wake,
    Mutate(Mutation, mpsc::Sender<Result<(), String>>),
    Test(String, mpsc::Sender<Result<(), String>>),
    Suspend(bool, mpsc::Sender<Result<(), String>>),
    Stop,
}
pub struct Engine {
    tx: Mutex<mpsc::Sender<Message>>,
    state: Mutex<Arc<Mutex<Snapshot>>>,
    join: Mutex<Option<thread::JoinHandle<()>>>,
}
impl Engine {
    pub fn start(
        app: AppHandle,
        storage: Storage,
        config: Config,
        load_error: Option<String>,
    ) -> Self {
        let repository = crate::updates::repository(&app);
        let state = Arc::new(Mutex::new(Snapshot {
            monitors: vec![],
            devices: vec![],
            sessions: vec![],
            applications: vec![],
            config,
            activity: vec![],
            routing_supported: false,
            compatibility: "Connecting to Windows audio services…".into(),
            successful_changes: 0,
            last_scan: None,
            errors: load_error.clone().into_iter().collect(),
            version: app.package_info().version.to_string(),
            updater_configured: repository.is_some(),
            repository,
        }));
        let (tx, rx) = mpsc::channel();
        let worker_state = state.clone();
        let worker_tx = tx.clone();
        let join = thread::spawn(move || {
            #[cfg(windows)]
            {
                let apartment = crate::native::ComApartment::new();
                let platform = apartment
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|_| crate::native::WindowsPlatform::new());
                match platform {
                    Ok(platform) => {
                        let _events =
                            crate::native::EventWatcher::start(platform.enumerator(), worker_tx);
                        Worker::new(app, storage, worker_state, load_error.is_some())
                            .run(platform, rx);
                    }
                    Err(e) => {
                        let mut s = worker_state.lock().unwrap_or_else(|e| e.into_inner());
                        s.compatibility = e.clone();
                        s.errors.push(e);
                        let snapshot = s.clone();
                        drop(s);
                        let _ = app.emit("state-changed", snapshot);
                    }
                }
            }
        });
        Self {
            tx: Mutex::new(tx),
            state: Mutex::new(state),
            join: Mutex::new(Some(join)),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let snapshot = state.lock().unwrap_or_else(|e| e.into_inner()).clone();
        snapshot
    }
    pub fn wake(&self) -> Result<(), String> {
        self.tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .send(Message::Wake)
            .map_err(|_| "Windows audio worker is unavailable. Restart AudioFlow.".into())
    }
    pub fn mutate(&self, mutation: Mutation) -> Result<(), String> {
        self.request(|reply| Message::Mutate(mutation, reply))
    }
    pub fn test(&self, id: String) -> Result<(), String> {
        self.request(|reply| Message::Test(id, reply))
    }
    pub fn suspend(&self, value: bool) -> Result<(), String> {
        self.request(|reply| Message::Suspend(value, reply))
    }
    fn request(
        &self,
        make: impl FnOnce(mpsc::Sender<Result<(), String>>) -> Message,
    ) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .send(make(tx))
            .map_err(|_| "Windows audio worker is unavailable. Restart AudioFlow.".to_string())?;
        rx.recv_timeout(Duration::from_secs(15)).map_err(|_| {
            "Windows did not respond in time. Scan again or restart AudioFlow.".to_string()
        })?
    }
    pub fn shutdown(&self) {
        let _ = self
            .tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .send(Message::Stop);
        if let Some(join) = self.join.lock().unwrap_or_else(|e| e.into_inner()).take() {
            if join.join().is_err() {
                log::error!("Audio worker panicked during shutdown");
            }
        }
    }
    pub fn restart(&self, app: AppHandle, storage: Storage) {
        self.shutdown();
        let replacement = Self::start(app, storage, self.snapshot().config, None);
        *self.tx.lock().unwrap_or_else(|e| e.into_inner()) = replacement
            .tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = replacement
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        *self.join.lock().unwrap_or_else(|e| e.into_inner()) = replacement
            .join
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
    }
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Managed {
    identity: u64,
    executable: String,
    original: Routes,
    applied: Routes,
}
struct Pending {
    target: Option<String>,
    since: Instant,
}
impl Managed {
    fn restore_if_owned<P: Platform>(&self, platform: &P, pid: u32) -> Result<bool, String> {
        if !platform
            .process(pid)
            .is_ok_and(|(exe, birth)| birth == self.identity && exe == self.executable)
        {
            return Ok(false);
        }
        if platform.get_current_route(pid)? != self.applied {
            return Ok(false);
        }
        platform.restore_routes(pid, &self.original)?;
        Ok(true)
    }
}
impl Pending {
    fn ready(&mut self, target: &Option<String>, now: Instant, delay: Duration) -> bool {
        if &self.target != target {
            self.target = target.clone();
            self.since = now;
        }
        now.duration_since(self.since) >= delay
    }
}
struct Worker {
    icons: HashMap<String, Option<String>>,
    app: AppHandle,
    storage: Storage,
    state: Arc<Mutex<Snapshot>>,
    config: Config,
    locked: bool,
    suspended: bool,
    managed: HashMap<u32, Managed>,
    pending: HashMap<u32, Pending>,
    relinquished: HashSet<(u32, u64)>,
    last_windows: HashMap<u32, isize>,
    last_monitors: HashMap<u32, String>,
    seen: HashSet<(u32, u64)>,
    deferred: HashMap<(u32, u64), Option<String>>,
    awaiting: HashMap<u32, (String, Option<String>, Instant)>,
    last_switch: HashMap<u32, String>,
    failures: HashMap<u32, (String, Instant)>,
    activity_dirty: bool,
    last_flush: Instant,
}
impl Worker {
    fn new(app: AppHandle, storage: Storage, state: Arc<Mutex<Snapshot>>, locked: bool) -> Self {
        let config = state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .config
            .clone();
        let path = storage.path.with_file_name("activity.json");
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(mut entries) = serde_json::from_slice::<Vec<Activity>>(&bytes) {
                entries.truncate(300);
                state.lock().unwrap_or_else(|e| e.into_inner()).activity = entries;
            }
        }
        let managed = std::fs::read(storage.path.with_file_name("managed-routes.json"))
            .ok()
            .filter(|bytes| bytes.len() < 2_000_000)
            .and_then(|bytes| serde_json::from_slice::<HashMap<u32, Managed>>(&bytes).ok())
            .filter(|routes| routes.len() <= 512)
            .unwrap_or_default();
        Self {
            icons: HashMap::new(),
            app,
            storage,
            state,
            config,
            locked,
            suspended: false,
            managed,
            pending: HashMap::new(),
            relinquished: HashSet::new(),
            last_windows: HashMap::new(),
            last_monitors: HashMap::new(),
            seen: HashSet::new(),
            deferred: HashMap::new(),
            awaiting: HashMap::new(),
            last_switch: HashMap::new(),
            failures: HashMap::new(),
            activity_dirty: false,
            last_flush: Instant::now(),
        }
    }
    fn event(
        &mut self,
        kind: &str,
        application: Option<String>,
        previous: Option<String>,
        next: Option<String>,
        outcome: &str,
        message: String,
    ) {
        if outcome == "failed"
            && self.config.settings.notifications
            && self
                .app
                .get_webview_window("main")
                .is_some_and(|w| !w.is_visible().unwrap_or(true))
        {
            use tauri_plugin_notification::NotificationExt;
            let _ = self
                .app
                .notification()
                .builder()
                .title("AudioFlow routing error")
                .body(&message)
                .show();
        }
        let entry = Activity {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: kind.into(),
            application,
            previous_device: previous,
            new_device: next,
            outcome: outcome.into(),
            message,
        };
        log::info!("{}: {}", entry.event_type, entry.message);
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.activity.insert(0, entry);
        s.activity.truncate(300);
        self.activity_dirty = true;
    }
    fn publish(&self) {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let _ = self.app.emit("state-changed", s);
    }
    fn save_managed(&self) {
        let path = self.storage.path.with_file_name("managed-routes.json");
        let temp = path.with_extension("json.tmp");
        let result = (|| -> Result<(), String> {
            use std::io::Write;
            let bytes = serde_json::to_vec(&self.managed).map_err(|e| e.to_string())?;
            let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            std::fs::rename(&temp, &path).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Managed route recovery journal could not be saved: {e}");
        }
    }
    fn flush(&mut self) {
        if self.activity_dirty {
            let entries = self
                .state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .activity
                .clone();
            let path = self.storage.path.with_file_name("activity.json");
            match serde_json::to_vec(&entries)
                .map_err(|e| e.to_string())
                .and_then(|bytes| std::fs::write(path, bytes).map_err(|e| e.to_string()))
            {
                Ok(()) => self.activity_dirty = false,
                Err(e) => log::warn!("Could not write activity history: {e}"),
            }
        }
        self.last_flush = Instant::now();
    }
    fn mutate(&mut self, mutation: Mutation) -> Result<(), String> {
        if matches!(&mutation, Mutation::ClearActivity) {
            self.state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .activity
                .clear();
            self.activity_dirty = true;
            self.flush();
            return Ok(());
        }
        if self.locked && !matches!(&mutation, Mutation::Import(_) | Mutation::Reset) {
            return Err("The saved configuration is invalid and has been preserved. Export it for recovery, then import a valid file or reset settings.".into());
        }
        let mut next = self.config.clone();
        let explicit_recovery = matches!(&mutation, Mutation::Import(_) | Mutation::Reset);
        match mutation {
            Mutation::Settings(settings) => next.settings = settings,
            Mutation::Assignment(id, endpoint) => {
                if let Some(endpoint) = endpoint {
                    next.assignments.insert(id, endpoint);
                } else {
                    next.assignments.remove(&id);
                }
            }
            Mutation::Rule(mut rule) => {
                let now = chrono::Utc::now().to_rfc3339();
                rule.updated_at = now.clone();
                if let Some(old) = next.rules.iter_mut().find(|r| r.id == rule.id) {
                    rule.created_at = old.created_at.clone();
                    *old = rule;
                } else {
                    rule.created_at = now;
                    next.rules.push(rule);
                }
                // An explicit rule grants control again after an external manual change.
                self.relinquished.clear();
            }
            Mutation::DeleteRule(id) => next.rules.retain(|r| r.id != id),
            Mutation::Import(config) => next = config,
            Mutation::Reset => next = Config::default(),
            Mutation::ClearActivity => {}
        }
        next.validate()?;
        use tauri_plugin_autostart::ManagerExt;
        let autostart = self.app.autolaunch();
        if next.settings.launch_on_startup != self.config.settings.launch_on_startup {
            if next.settings.launch_on_startup {
                autostart.enable()
            } else {
                autostart.disable()
            }
            .map_err(|e| format!("Windows startup setting could not be changed: {e}"))?;
        }
        if explicit_recovery && self.locked {
            std::fs::copy(
                &self.storage.path,
                self.storage
                    .path
                    .with_extension(format!("invalid-{}.json", chrono::Utc::now().timestamp())),
            )
            .map_err(|e| e.to_string())?;
        }
        if let Err(e) = self.storage.save(&next) {
            if self.config.settings.launch_on_startup {
                let _ = autostart.enable();
            } else {
                let _ = autostart.disable();
            }
            return Err(e);
        }
        log::set_max_level(if next.settings.debug_logging {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        });
        let enabled_changed = self.config.settings.routing_enabled != next.settings.routing_enabled;
        self.config = next.clone();
        self.locked = false;
        self.pending.clear();
        self.failures.clear();
        self.state.lock().unwrap_or_else(|e| e.into_inner()).config = next;
        if enabled_changed {
            self.event(
                "routing",
                None,
                None,
                None,
                "saved",
                if self.config.settings.routing_enabled {
                    "Automatic routing enabled."
                } else {
                    "Automatic routing paused. Current outputs are preserved."
                }
                .into(),
            );
        }
        Ok(())
    }
    fn restore<P: Platform>(&mut self, platform: &P) -> Result<(), String> {
        let mut errors = vec![];
        for (pid, owned) in &self.managed {
            if let Err(e) = owned.restore_if_owned(platform, *pid) {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            self.managed.retain(|pid, owned| {
                !platform
                    .process(*pid)
                    .is_ok_and(|(exe, birth)| exe == owned.executable && birth == owned.identity)
            });
            self.save_managed();
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
    fn run<P: Platform>(&mut self, platform: P, rx: mpsc::Receiver<Message>) {
        {
            let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            s.routing_supported = platform.supports_process_routing();
            s.compatibility=platform.get_last_error().unwrap_or_else(||"Windows policy interface available. Audio movement is verified separately from preference changes.".into());
        }
        self.scan(&platform);
        self.publish();
        let mut next_scan = Instant::now() + Duration::from_secs(3);
        loop {
            let timeout = next_scan.saturating_duration_since(Instant::now());
            match rx.recv_timeout(timeout) {
                Ok(Message::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(Message::Wake) => {
                    next_scan = next_scan.min(Instant::now() + Duration::from_millis(150));
                }
                Ok(Message::Mutate(mutation, reply)) => {
                    let result = self.mutate(mutation);
                    let _ = reply.send(result);
                    self.publish();
                    next_scan = Instant::now();
                }
                Ok(Message::Test(endpoint, reply)) => {
                    let result = platform.test_speaker(&endpoint);
                    let _ = reply.send(result);
                }
                Ok(Message::Suspend(value, reply)) => {
                    self.suspended = value;
                    let result = if value && self.config.settings.restore_on_exit {
                        self.restore(&platform)
                    } else {
                        Ok(())
                    };
                    let _ = reply.send(result);
                    next_scan = Instant::now();
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !self.suspended {
                        self.scan(&platform);
                        self.publish();
                    }
                    next_scan = Instant::now()
                        + if self.pending.is_empty() && self.awaiting.is_empty() {
                            Duration::from_secs(3)
                        } else {
                            Duration::from_millis(150)
                        };
                }
            }
            if self.last_flush.elapsed() > Duration::from_secs(30) {
                self.flush();
            }
        }
        if self.config.settings.restore_on_exit {
            if let Err(e) = self.restore(&platform) {
                log::error!("Route restoration: {e}");
            }
        }
        self.flush();
    }
    fn scan<P: Platform>(&mut self, platform: &P) {
        let previous_managed = self.managed.clone();
        log::debug!("Reconciling Windows displays, windows, and audio sessions");
        let mut errors = vec![];
        let monitors = match platform.monitors() {
            Ok(v) => v,
            Err(e) => {
                errors.push(e);
                vec![]
            }
        };
        let devices = match platform.list_output_devices() {
            Ok(v) => v,
            Err(e) => {
                errors.push(e);
                vec![]
            }
        };
        let (sessions, sessions_ok) = match platform.list_audio_sessions(&devices) {
            Ok(v) => (v, true),
            Err(e) => {
                errors.push(e);
                (vec![], false)
            }
        };
        let windows = match platform.windows() {
            Ok(v) => v,
            Err(e) => {
                errors.push(e);
                vec![]
            }
        };
        let parents = platform.parents();
        let topology_changed = {
            let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            s.monitors != monitors || s.devices != devices
        };
        if topology_changed {
            self.event(
                "devices",
                None,
                None,
                None,
                "detected",
                format!(
                    "Detected {} displays and {} active playback devices.",
                    monitors.len(),
                    devices.len()
                ),
            );
        }
        let pids: HashSet<_> = sessions.iter().map(|s| s.pid).collect();
        let mut applications = vec![];
        for pid in &pids {
            let (executable, identity) = match platform.process(*pid) {
                Ok(value) => value,
                Err(e) => {
                    let own: Vec<_> = sessions.iter().filter(|s| s.pid == *pid).collect();
                    applications.push(Application {
                        icon_data: None,
                        pid: *pid,
                        name: format!("Process {pid}"),
                        executable: String::new(),
                        monitor_id: None,
                        endpoint_ids: own.iter().map(|s| s.endpoint_id.clone()).collect(),
                        active: own.iter().any(|s| s.active),
                        peak: None,
                        mode: RoutingMode::Ignore,
                        status: "Windows restricted access to this process".into(),
                        last_switch: None,
                        last_error: Some(e),
                        window_count: 0,
                    });
                    continue;
                }
            };
            let name = executable
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or(&executable)
                .trim_end_matches(".exe")
                .to_string();
            if self
                .managed
                .get(pid)
                .is_some_and(|m| m.identity != identity || m.executable != executable)
            {
                self.managed.remove(pid);
                self.pending.remove(pid);
                self.awaiting.remove(pid);
                self.last_monitors.remove(pid);
                self.last_windows.remove(pid);
            }
            let mut owner = *pid;
            let mut candidates: Vec<AppWindow> =
                windows.iter().filter(|w| w.pid == owner).cloned().collect();
            for _ in 0..8 {
                if !candidates.is_empty() {
                    break;
                }
                let Some(parent) = parents.get(&owner) else {
                    break;
                };
                if *parent == owner || *parent == 0 {
                    break;
                }
                owner = *parent;
                // Only climb within the same executable, so a launcher cannot own a child app's sound.
                if !platform
                    .process(owner)
                    .is_ok_and(|(path, _)| path.eq_ignore_ascii_case(&executable))
                {
                    break;
                }
                candidates = windows.iter().filter(|w| w.pid == owner).cloned().collect();
            }
            let selected = select_window(&candidates, self.last_windows.get(pid).copied());
            if let Some(w) = selected {
                self.last_windows.insert(*pid, w.handle);
            }
            let monitor = selected
                .and_then(|w| {
                    select_monitor(
                        w.bounds,
                        &monitors,
                        self.last_monitors.get(pid).map(String::as_str),
                    )
                })
                .map(|m| m.id.clone());
            if let Some(m) = &monitor {
                self.last_monitors.insert(*pid, m.clone());
            }
            let app_sessions: Vec<_> = sessions.iter().filter(|s| s.pid == *pid).collect();
            let active = app_sessions.iter().any(|s| s.active);
            let endpoints: Vec<_> = app_sessions
                .iter()
                .map(|s| s.endpoint_id.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            let peak = app_sessions.iter().filter_map(|s| s.peak).reduce(f32::max);
            let rule = matching_rule(&self.config, &executable).cloned();
            let mode = rule
                .as_ref()
                .map(|r| r.mode.clone())
                .unwrap_or(RoutingMode::Auto);
            let current = platform.get_current_route(*pid);
            // Recover ownership after AudioFlow or the audio application restarts.
            // Only reclaim an exact executable whose preferences still match our last write.
            if !self.managed.contains_key(pid) {
                let previous = self
                    .managed
                    .iter()
                    .find(|(old_pid, owned)| {
                        owned.executable == executable
                            && current.as_ref().is_ok_and(|r| r == &owned.applied)
                            && !platform
                                .process(**old_pid)
                                .is_ok_and(|(_, birth)| birth == owned.identity)
                    })
                    .map(|(id, owned)| (*id, owned.clone()));
                if let Some((old_pid, mut owned)) = previous {
                    self.managed.remove(&old_pid);
                    owned.identity = identity;
                    self.managed.insert(*pid, owned);
                }
            }
            if let (Some(owned), Ok(routes)) = (self.managed.get(pid), &current) {
                if routes != &owned.applied && self.config.settings.respect_manual {
                    self.managed.remove(pid);
                    self.pending.remove(pid);
                    self.awaiting.remove(pid);
                    self.relinquished.insert((*pid, identity));
                    self.event("manual",Some(name.clone()),None,None,"preserved","Windows output was changed externally. AudioFlow released control until an explicit rule is saved or the process restarts.".into());
                }
            }
            let manual = current
                .as_ref()
                .is_ok_and(|r| r.iter().any(Option::is_some))
                && !self.managed.contains_key(pid);
            let decision = decide(
                &self.config,
                &executable,
                monitor.as_deref(),
                &devices,
                manual,
            );
            let first_seen = self.seen.insert((*pid, identity));
            if first_seen && !self.config.settings.route_new_apps && rule.is_none() {
                self.deferred.insert((*pid, identity), monitor.clone());
            }
            if self
                .deferred
                .get(&(*pid, identity))
                .is_some_and(|original| original != &monitor)
                || rule.is_some()
                || self.config.settings.route_new_apps
            {
                self.deferred.remove(&(*pid, identity));
            }
            let mut status = match &decision {
                Decision::Leave(reason) => (*reason).to_string(),
                Decision::Route(_) => "Following monitor".into(),
            };
            let mut last_error = None;
            if let Some((target, previous, since)) = self.awaiting.get(pid).cloned() {
                if app_sessions
                    .iter()
                    .any(|s| s.active && s.endpoint_id == target)
                    && !app_sessions
                        .iter()
                        .any(|s| s.active && s.endpoint_id != target)
                {
                    self.awaiting.remove(pid);
                    let changed = previous
                        .as_ref()
                        .is_some_and(|endpoint| endpoint != &target);
                    if changed {
                        let timestamp = chrono::Utc::now().to_rfc3339();
                        self.last_switch.insert(*pid, timestamp);
                        self.state
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .successful_changes += 1;
                    }
                    self.event(
                        if changed { "verified" } else { "confirmed" },
                        Some(name.clone()),
                        previous,
                        Some(target),
                        if changed { "verified" } else { "observed" },
                        "An active Windows audio session was observed on the requested output."
                            .into(),
                    );
                } else {
                    status = if since.elapsed() > Duration::from_secs(8) {
                        "Preference saved; restart playback to apply"
                    } else {
                        "Preference saved; waiting for audio session"
                    }
                    .into();
                }
            }
            if self.relinquished.contains(&(*pid, identity)) {
                status = "External Windows assignment preserved".into();
            } else if !platform.supports_process_routing() {
                status = "Per-process routing unavailable".into();
            } else if self.deferred.contains_key(&(*pid, identity)) {
                status = "New app routing disabled; move its window to take control".into();
            } else if let Decision::Route(target) = decision {
                if let Err(e) = &current {
                    status = "Could not read Windows route".into();
                    last_error = Some(e.clone());
                } else {
                    let expected = [target.clone(), target.clone(), target.clone()];
                    let already = current.as_ref().is_ok_and(|r| r == &expected);
                    if already {
                        self.pending.remove(pid);
                        let observed = target.as_deref().or_else(|| {
                            devices.iter().find(|d| d.is_default).map(|d| d.id.as_str())
                        });
                        if !app_sessions
                            .iter()
                            .any(|s| Some(s.endpoint_id.as_str()) == observed)
                        {
                            status = "Preference saved; restart playback to apply".into();
                        }
                    } else if self
                        .failures
                        .get(pid)
                        .is_some_and(|(_, when)| when.elapsed() < Duration::from_secs(10))
                    {
                        status = "Routing failed; will retry".into();
                        last_error = self.failures.get(pid).map(|(e, _)| e.clone());
                    } else {
                        let pending = self.pending.entry(*pid).or_insert_with(|| Pending {
                            target: target.clone(),
                            since: Instant::now(),
                        });
                        if !pending.ready(
                            &target,
                            Instant::now(),
                            Duration::from_millis(self.config.settings.switch_delay_ms),
                        ) {
                            status = "Waiting for switching delay".into();
                        } else {
                            let original = current.as_ref().unwrap().clone();
                            match if target.is_none() {
                                platform.clear_process_route(*pid)
                            } else {
                                platform.set_process_output(*pid, target.as_deref())
                            } {
                                Ok(()) => {
                                    self.managed
                                        .entry(*pid)
                                        .and_modify(|m| m.applied = expected.clone())
                                        .or_insert(Managed {
                                            identity,
                                            executable: executable.clone(),
                                            original,
                                            applied: expected,
                                        });
                                    let actual_target = target.clone().or_else(|| {
                                        devices.iter().find(|d| d.is_default).map(|d| d.id.clone())
                                    });
                                    if let Some(endpoint) = actual_target {
                                        let previous = app_sessions
                                            .iter()
                                            .find(|s| s.active && s.endpoint_id != endpoint)
                                            .map(|s| s.endpoint_id.clone());
                                        self.awaiting
                                            .insert(*pid, (endpoint, previous, Instant::now()));
                                    }
                                    self.pending.remove(pid);
                                    self.failures.remove(pid);
                                    status = "Preference saved; waiting for audio session".into();
                                    self.event("preference",Some(name.clone()),endpoints.first().cloned(),target,"pending","Windows accepted the app output preference. Live stream movement is not yet confirmed.".into());
                                }
                                Err(e) => {
                                    status = "Routing failed".into();
                                    last_error = Some(e.clone());
                                    let repeated =
                                        self.failures.get(pid).is_some_and(|(old, _)| old == &e);
                                    self.failures.insert(*pid, (e.clone(), Instant::now()));
                                    if !repeated {
                                        self.event(
                                            "error",
                                            Some(name.clone()),
                                            None,
                                            target,
                                            "failed",
                                            e,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                self.pending.remove(pid);
            }
            let icon_data = self
                .icons
                .entry(executable.clone())
                .or_insert_with(|| platform.icon(&executable))
                .clone();
            applications.push(Application {
                icon_data,
                pid: *pid,
                name,
                executable,
                monitor_id: monitor,
                endpoint_ids: endpoints,
                active,
                peak,
                mode,
                status,
                last_switch: self.last_switch.get(pid).cloned(),
                last_error,
                window_count: candidates.len(),
            });
        }
        // Restore still-running owned processes with no sessions before dropping their ownership.
        let inactive: Vec<_> = self
            .managed
            .keys()
            .filter(|pid| sessions_ok && !pids.contains(pid))
            .copied()
            .collect();
        for pid in inactive {
            if let Some(owned) = self.managed.remove(&pid) {
                if platform.process(pid).is_err() {
                    self.managed.insert(pid, owned);
                    continue;
                }
                if let Err(e) = owned.restore_if_owned(platform, pid) {
                    errors.push(e);
                    self.managed.insert(pid, owned);
                }
            }
        }
        self.pending.retain(|pid, _| pids.contains(pid));
        self.awaiting.retain(|pid, _| pids.contains(pid));
        self.failures.retain(|pid, _| pids.contains(pid));
        self.last_windows.retain(|pid, _| pids.contains(pid));
        self.last_monitors.retain(|pid, _| pids.contains(pid));
        self.last_switch.retain(|pid, _| pids.contains(pid));
        self.seen.retain(|(pid, birth)| {
            pids.contains(pid)
                || platform
                    .process(*pid)
                    .is_ok_and(|(_, identity)| identity == *birth)
        });
        self.deferred
            .retain(|identity, _| self.seen.contains(identity));
        self.relinquished
            .retain(|identity| self.seen.contains(identity));
        if self.icons.len() > 256 {
            self.icons
                .retain(|path, _| applications.iter().any(|a| &a.executable == path));
        }
        applications.sort_by_key(|a| (!a.active, a.name.clone(), a.pid));
        if self.managed.len() > 512 {
            self.managed.retain(|pid, _| pids.contains(pid));
        }
        if previous_managed != self.managed {
            self.save_managed();
        }
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.monitors = monitors;
        s.devices = devices;
        s.sessions = sessions;
        s.applications = applications;
        s.last_scan = Some(chrono::Utc::now().to_rfc3339());
        if self.locked {
            errors.extend(s.errors.clone());
        }
        errors.sort();
        errors.dedup();
        s.errors = errors;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::AudioRoutingProvider;
    use std::cell::RefCell;
    struct FakeWindows {
        identity: u64,
        current: RefCell<Routes>,
        writes: RefCell<u32>,
    }
    impl AudioRoutingProvider for FakeWindows {
        fn list_output_devices(&self) -> Result<Vec<Device>, String> {
            Ok(vec![])
        }
        fn list_audio_sessions(&self, _: &[Device]) -> Result<Vec<Session>, String> {
            Ok(vec![])
        }
        fn get_current_route(&self, _: u32) -> Result<Routes, String> {
            Ok(self.current.borrow().clone())
        }
        fn set_process_output(&self, _: u32, endpoint: Option<&str>) -> Result<(), String> {
            *self.current.borrow_mut() = [
                endpoint.map(str::to_owned),
                endpoint.map(str::to_owned),
                endpoint.map(str::to_owned),
            ];
            *self.writes.borrow_mut() += 1;
            Ok(())
        }
        fn restore_routes(&self, _: u32, routes: &Routes) -> Result<(), String> {
            *self.current.borrow_mut() = routes.clone();
            *self.writes.borrow_mut() += 1;
            Ok(())
        }
        fn supports_process_routing(&self) -> bool {
            true
        }
        fn get_last_error(&self) -> Option<String> {
            None
        }
    }
    impl Platform for FakeWindows {
        fn monitors(&self) -> Result<Vec<Monitor>, String> {
            Ok(vec![])
        }
        fn windows(&self) -> Result<Vec<AppWindow>, String> {
            Ok(vec![])
        }
        fn process(&self, _: u32) -> Result<(String, u64), String> {
            Ok(("app.exe".into(), self.identity))
        }
        fn parents(&self) -> HashMap<u32, u32> {
            HashMap::new()
        }
        fn test_speaker(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
    }
    fn owned() -> Managed {
        Managed {
            identity: 10,
            executable: "app.exe".into(),
            original: [
                None,
                Some("original-media".into()),
                Some("original-chat".into()),
            ],
            applied: [
                Some("managed".into()),
                Some("managed".into()),
                Some("managed".into()),
            ],
        }
    }
    #[test]
    fn restores_each_prior_role_only_when_owned() {
        let owned = owned();
        let fake = FakeWindows {
            identity: 10,
            current: RefCell::new(owned.applied.clone()),
            writes: RefCell::new(0),
        };
        assert!(owned.restore_if_owned(&fake, 1).unwrap());
        assert_eq!(*fake.current.borrow(), owned.original);
        assert_eq!(*fake.writes.borrow(), 1);
    }
    #[test]
    fn preserves_external_manual_changes_and_reused_pids() {
        let owned = owned();
        let fake = FakeWindows {
            identity: 10,
            current: RefCell::new([Some("user-choice".into()), None, None]),
            writes: RefCell::new(0),
        };
        assert!(!owned.restore_if_owned(&fake, 1).unwrap());
        assert_eq!(*fake.writes.borrow(), 0);
        let fake = FakeWindows {
            identity: 11,
            current: RefCell::new(owned.applied.clone()),
            writes: RefCell::new(0),
        };
        assert!(!owned.restore_if_owned(&fake, 1).unwrap());
        assert_eq!(*fake.writes.borrow(), 0);
    }
    #[test]
    fn rapid_crossings_wait_for_a_stable_target() {
        let now = Instant::now();
        let delay = Duration::from_millis(350);
        let a = Some("speaker-a".into());
        let b = Some("speaker-b".into());
        let mut pending = Pending {
            target: a.clone(),
            since: now,
        };
        assert!(!pending.ready(&a, now + Duration::from_millis(200), delay));
        assert!(!pending.ready(&b, now + Duration::from_millis(250), delay));
        assert!(!pending.ready(&a, now + Duration::from_millis(400), delay));
        assert!(!pending.ready(&a, now + Duration::from_millis(700), delay));
        assert!(pending.ready(&a, now + Duration::from_millis(750), delay));
    }
}
