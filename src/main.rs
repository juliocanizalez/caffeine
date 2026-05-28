use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{TrayIcon, TrayIconBuilder};

mod duration;
mod ipc;
mod power;

use power::AssertionGuard;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "caffeine",
    version,
    about = "Keep your Mac awake — spawns a live countdown in the menu bar",
    long_about = None
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// How long to stay awake: "30" (min), "2h", "1h30m", "90s", or omit for indefinite
    duration: Option<String>,

    /// Prevent only system idle sleep, not display sleep (lower power draw)
    #[arg(long)]
    no_display: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Print the status of the running caffeine instance
    Status,
    /// Stop the running caffeine instance
    Stop,
}

// ── Status / stop handlers (exit before the event loop) ──────────────────────

fn cmd_status() {
    match ipc::Status::read() {
        None => println!("○ Not running"),
        Some(s) => {
            if !s.is_alive() {
                ipc::Status::delete();
                println!("○ Not running");
                return;
            }
            let now = ipc::now_secs();
            let mode = if s.prevent_display {
                "display + system"
            } else {
                "system"
            };
            match s.expiry {
                None => {
                    println!("● Active — indefinite");
                    println!("  {} sleep prevented", mode);
                    println!("  PID {}", s.pid);
                }
                Some(expiry) if expiry > now => {
                    let remaining = Duration::from_secs(expiry - now);
                    println!("● Active — {} remaining", duration::fmt(remaining));
                    println!("  {} sleep prevented", mode);
                    println!("  PID {}", s.pid);
                }
                Some(_) => {
                    println!("● Running (timer expired — menu bar still open)");
                    println!("  PID {}", s.pid);
                }
            }
        }
    }
}

fn cmd_stop() {
    match ipc::Status::read() {
        None => println!("caffeine is not running"),
        Some(s) => {
            if !s.is_alive() {
                ipc::Status::delete();
                println!("caffeine is not running");
                return;
            }
            unsafe { libc::kill(s.pid as libc::pid_t, libc::SIGTERM) };
            println!("Stopped caffeine (PID {})", s.pid);
        }
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    guard: Option<AssertionGuard>,
    expiry: Option<Instant>,
    prevent_display: bool,
}

impl State {
    fn new(prevent_display: bool) -> Self {
        Self {
            guard: None,
            expiry: None,
            prevent_display,
        }
    }

    fn active(&self) -> bool {
        self.guard.is_some()
    }

    fn remaining(&self) -> Option<Duration> {
        self.expiry.map(|e| {
            e.checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO)
        })
    }

    fn activate(&mut self, dur: Option<Duration>) {
        self.guard = AssertionGuard::acquire(self.prevent_display)
            .map_err(|e| eprintln!("caffeine: {e}"))
            .ok();
        self.expiry = dur.map(|d| Instant::now() + d);
    }

    fn deactivate(&mut self) {
        self.guard = None;
        self.expiry = None;
    }

    fn tick(&mut self) {
        if let Some(rem) = self.remaining()
            && rem.is_zero()
        {
            self.deactivate();
            ipc::Status::delete();
        }
    }

    fn tray_title(&self) -> String {
        if !self.active() {
            return "☕ off".into();
        }
        match self.remaining() {
            None => "☕ ∞".into(),
            Some(d) if d.is_zero() => "☕ off".into(),
            Some(d) => format!("☕ {}", duration::fmt(d)),
        }
    }

    fn status_text(&self) -> String {
        if !self.active() {
            return "Inactive".into();
        }
        match self.expiry {
            None => "Active · indefinite".into(),
            Some(e) => {
                let rem = e
                    .checked_duration_since(Instant::now())
                    .unwrap_or(Duration::ZERO);
                format!("Active · {} remaining", duration::fmt(rem))
            }
        }
    }
}

// ── Sync lock file with current state ────────────────────────────────────────

fn write_lock(state: &State, pid: u32, started_at: u64) {
    if !state.active() {
        ipc::Status::delete();
        return;
    }
    let now = ipc::now_secs();
    let expiry_secs = state.expiry.map(|e| {
        let rem = e.checked_duration_since(Instant::now()).unwrap_or_default();
        now + rem.as_secs()
    });
    ipc::Status {
        pid,
        started_at,
        expiry: expiry_secs,
        prevent_display: state.prevent_display,
    }
    .write();
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args = Args::parse();

    // Handle non-interactive subcommands before any GUI startup
    match &args.command {
        Some(Command::Status) => {
            cmd_status();
            return;
        }
        Some(Command::Stop) => {
            cmd_stop();
            return;
        }
        None => {}
    }

    let initial_dur: Option<Duration> = args
        .duration
        .as_deref()
        .and_then(|s| {
            duration::parse(s).unwrap_or_else(|e| {
                eprintln!("caffeine: invalid duration — {e}");
                std::process::exit(1);
            })
        });

    // ── Build menu items ──────────────────────────────────────────────────────

    let item_status = MenuItem::new("Active · indefinite", false, None);
    let item_15m = MenuItem::new("15 minutes", true, None);
    let item_30m = MenuItem::new("30 minutes", true, None);
    let item_1h = MenuItem::new("1 hour", true, None);
    let item_2h = MenuItem::new("2 hours", true, None);
    let item_4h = MenuItem::new("4 hours", true, None);
    let item_inf = MenuItem::new("Indefinite", true, None);
    let item_toggle = MenuItem::new("Stop", true, None);
    let item_quit = MenuItem::new("Quit caffeine", true, None);

    let menu = Menu::new();
    menu.append(&item_status).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&item_15m).unwrap();
    menu.append(&item_30m).unwrap();
    menu.append(&item_1h).unwrap();
    menu.append(&item_2h).unwrap();
    menu.append(&item_4h).unwrap();
    menu.append(&item_inf).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&item_toggle).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&item_quit).unwrap();

    // ── State + lock file ─────────────────────────────────────────────────────

    let my_pid = std::process::id();
    let started_at = ipc::now_secs();

    let mut menu_opt: Option<Menu> = Some(menu);
    let mut tray: Option<TrayIcon> = None;
    let mut state = State::new(!args.no_display);
    state.activate(initial_dur);
    write_lock(&state, my_pid, started_at);

    // ── Event loop ────────────────────────────────────────────────────────────

    let mut event_loop = EventLoopBuilder::<()>::new().build();

    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(500));

        // ── Init: create the tray icon on the first event ─────────────────────
        if tray.is_none()
            && let Event::NewEvents(StartCause::Init) = event
            && let Some(m) = menu_opt.take()
        {
            tray = Some(
                TrayIconBuilder::new()
                    .with_menu(Box::new(m))
                    .with_title(state.tray_title())
                    .with_tooltip("caffeine")
                    .build()
                    .expect("failed to create tray icon"),
            );
        }

        // ── Tick: check expiry ────────────────────────────────────────────────
        state.tick();

        // ── Sync tray title + menu text ───────────────────────────────────────
        if let Some(t) = tray.as_ref() {
            t.set_title(Some(&state.tray_title()));
        }
        item_status.set_text(state.status_text());
        item_toggle.set_text(if state.active() { "Stop" } else { "Resume" });

        // ── Handle menu clicks ────────────────────────────────────────────────
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let id = &ev.id;

            if id == item_quit.id() {
                state.deactivate();
                ipc::Status::delete();
                *control_flow = ControlFlow::Exit;
                return;
            }

            if id == item_toggle.id() {
                if state.active() {
                    state.deactivate();
                    ipc::Status::delete();
                } else {
                    state.activate(None);
                    write_lock(&state, my_pid, started_at);
                }
                return;
            }

            let preset = if id == item_15m.id() {
                Some(Duration::from_secs(15 * 60))
            } else if id == item_30m.id() {
                Some(Duration::from_secs(30 * 60))
            } else if id == item_1h.id() {
                Some(Duration::from_secs(3600))
            } else if id == item_2h.id() {
                Some(Duration::from_secs(2 * 3600))
            } else if id == item_4h.id() {
                Some(Duration::from_secs(4 * 3600))
            } else if id == item_inf.id() {
                None
            } else {
                continue;
            };

            state.activate(preset);
            write_lock(&state, my_pid, started_at);
        }
    });
}
