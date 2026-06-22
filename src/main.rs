use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use caffeine::application::CaffeineService;
use caffeine::domain::ports::{ConfigRepository, LoginItemManager, StatusRepository};
use caffeine::infrastructure::{
    battery::IokitBatteryMonitor,
    config::FileConfigRepository,
    ipc::FileStatusRepository,
    jiggle::{CoreGraphicsIdleDetector, CoreGraphicsJiggler},
    login_item::LaunchdLoginItemManager,
    power::IokitPowerManager,
    update_checker,
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "caffeine",
    version,
    about = "Keep your Mac awake — spawns a menu bar icon",
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

    /// Keep Teams/Slack status active by simulating periodic mouse activity
    #[arg(short = 'k', long)]
    keep_status_active: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Print the status of the running caffeine instance
    Status {
        /// Output status as JSON
        #[arg(long)]
        json: bool,
    },
    /// Stop the running caffeine instance
    Stop,
    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

// ── Status / stop handlers (exit before the event loop) ──────────────────────

fn cmd_status(repo: &FileStatusRepository, json: bool) {
    match repo.read() {
        None => {
            if json {
                println!("{{\"running\":false}}");
            } else {
                println!("○ Not running");
            }
        }
        Some(s) => {
            if !repo.is_alive(s.pid) {
                repo.delete();
                if json {
                    println!("{{\"running\":false}}");
                } else {
                    println!("○ Not running");
                }
                return;
            }
            let now = repo.now_secs();

            if json {
                let remaining_secs = s.expiry.and_then(|exp| exp.checked_sub(now));
                let obj = serde_json::json!({
                    "running": true,
                    "pid": s.pid,
                    "expiry_unix": s.expiry,
                    "remaining_secs": remaining_secs,
                    "prevent_display": s.prevent_display,
                    "jiggle": s.jiggle,
                });
                println!("{}", serde_json::to_string_pretty(&obj).unwrap());
                return;
            }

            let mode = if s.prevent_display {
                "display + system"
            } else {
                "system"
            };
            match s.expiry {
                None => {
                    println!("● Active — indefinite");
                    println!("  {} sleep prevented", mode);
                    if s.jiggle {
                        println!("  Keep Status Active: enabled");
                    }
                    println!("  PID {}", s.pid);
                }
                Some(expiry) if expiry > now => {
                    let remaining = Duration::from_secs(expiry - now);
                    println!(
                        "● Active — {} remaining",
                        caffeine::duration::fmt(remaining)
                    );
                    println!("  {} sleep prevented", mode);
                    if s.jiggle {
                        println!("  Keep Status Active: enabled");
                    }
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

fn cmd_stop(repo: &FileStatusRepository) {
    match repo.read() {
        None => println!("caffeine is not running"),
        Some(s) => {
            if !repo.is_alive(s.pid) {
                repo.delete();
                println!("caffeine is not running");
                return;
            }
            unsafe { libc::kill(s.pid as libc::pid_t, libc::SIGTERM) };
            println!("Stopped caffeine (PID {})", s.pid);
        }
    }
}

// ── Menu bar icon ─────────────────────────────────────────────────────────────

static ICON_ACTIVE_SVG: &[u8] = include_bytes!("../assets/icon_active.svg");
static ICON_INACTIVE_SVG: &[u8] = include_bytes!("../assets/icon_inactive.svg");

fn load_icon(bytes: &[u8]) -> Icon {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_data(bytes, &Default::default()).expect("invalid SVG");
    let w = tree.size().width().round() as u32;
    let h = tree.size().height().round() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("failed to allocate pixmap");
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Icon::from_rgba(pixmap.data().to_vec(), w, h).expect("invalid icon RGBA")
}

struct MenuItems {
    status: MenuItem,
    update: MenuItem,
    /// (label, seconds) — None seconds means indefinite.
    presets: Vec<(MenuItem, Option<u64>)>,
    inf: MenuItem,
    keep_status: CheckMenuItem,
    launch_at_login: CheckMenuItem,
    toggle: MenuItem,
    quit: MenuItem,
}

impl MenuItems {
    /// Build (or rebuild) the tray menu. Prepends the update banner when `show_update` is true.
    fn build(&self, show_update: bool) -> Menu {
        let menu = Menu::new();
        if show_update {
            menu.append(&self.update).unwrap();
            menu.append(&PredefinedMenuItem::separator()).unwrap();
        }
        menu.append(&self.status).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        for (item, _) in &self.presets {
            menu.append(item).unwrap();
        }
        menu.append(&self.inf).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&self.keep_status).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&self.launch_at_login).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&self.toggle).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&self.quit).unwrap();
        menu
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args = Args::parse();

    let repo = FileStatusRepository;

    // Load config; CLI flags override config values.
    let cfg = FileConfigRepository.load();

    match &args.command {
        Some(Command::Status { json }) => {
            cmd_status(&repo, *json);
            return;
        }
        Some(Command::Stop) => {
            cmd_stop(&repo);
            return;
        }
        Some(Command::Completions { shell }) => {
            clap_complete::generate(*shell, &mut Args::command(), "caffeine", &mut stdout());
            return;
        }
        None => {}
    }

    let initial_dur: Option<Duration> = args.duration.as_deref().and_then(|s| {
        caffeine::duration::parse(s).unwrap_or_else(|e| {
            eprintln!("caffeine: invalid duration — {e}");
            std::process::exit(1);
        })
    });

    // Merge config + CLI flags (CLI wins).
    let prevent_display = if args.no_display {
        false
    } else {
        cfg.prevent_display
    };
    let jiggle_enabled = args.keep_status_active || cfg.keep_status_active;

    // ── Detach from terminal when run interactively ───────────────────────────

    {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;

        let in_tty = unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 };
        let already_daemon = std::env::var("CAFFEINE_DAEMON").is_ok();

        if in_tty && !already_daemon {
            let exe = std::env::current_exe().expect("cannot resolve executable path");
            let child = std::process::Command::new(&exe)
                .args(std::env::args_os().skip(1))
                .env("CAFFEINE_DAEMON", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn()
                .expect("failed to spawn caffeine in background");
            let pid = child.id();
            std::mem::forget(child);
            println!("Caffeine started (PID {})", pid);
            return;
        }
    }

    // ── Spawn update checker in background ───────────────────────────────────

    let update_result: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    if cfg.check_for_updates {
        let result_ref = Arc::clone(&update_result);
        std::thread::spawn(move || {
            if let Some(tag) = update_checker::check_for_update()
                && let Ok(mut guard) = result_ref.lock()
            {
                *guard = Some(tag);
            }
        });
    }

    // ── Login item manager ────────────────────────────────────────────────────

    let login_mgr = LaunchdLoginItemManager;

    // ── Build menu items ──────────────────────────────────────────────────────

    let presets: Vec<(MenuItem, Option<u64>)> = cfg
        .menu_durations
        .iter()
        .filter_map(|s| {
            match caffeine::duration::parse(s) {
                Ok(Some(d)) => {
                    let secs = d.as_secs();
                    let label = caffeine::duration::fmt(d);
                    Some((MenuItem::new(label, true, None), Some(secs)))
                }
                Ok(None) => None, // "0" or similar — skip
                Err(e) => {
                    eprintln!("caffeine: ignoring invalid menu_duration {s:?} — {e}");
                    None
                }
            }
        })
        .collect();

    let items = MenuItems {
        status: MenuItem::new("Active · indefinite", false, None),
        update: MenuItem::new("", true, None), // text set when update found
        presets,
        inf: MenuItem::new("Indefinite", true, None),
        keep_status: CheckMenuItem::new("Keep Status Active", true, jiggle_enabled, None),
        launch_at_login: CheckMenuItem::new("Launch at Login", true, login_mgr.is_enabled(), None),
        toggle: MenuItem::new("Stop", true, None),
        quit: MenuItem::new("Quit caffeine", true, None),
    };
    let menu = items.build(false);

    // ── Wire service ──────────────────────────────────────────────────────────

    let mut service = CaffeineService::new(
        Box::new(IokitPowerManager),
        Box::new(CoreGraphicsIdleDetector),
        Box::new(CoreGraphicsJiggler),
        Box::new(FileStatusRepository),
        Box::new(IokitBatteryMonitor),
        prevent_display,
        jiggle_enabled,
        cfg.battery_threshold,
        std::process::id(),
    );
    service.activate(initial_dur);
    service.sync_status();

    // ── Pre-load both icon variants ───────────────────────────────────────────

    let icon_active = load_icon(ICON_ACTIVE_SVG);
    let icon_inactive = load_icon(ICON_INACTIVE_SVG);
    let mut last_active: Option<bool> = None;

    // ── Event loop ────────────────────────────────────────────────────────────

    let mut menu_opt: Option<Menu> = Some(menu);
    let mut tray: Option<TrayIcon> = None;
    let mut update_shown = false;

    let mut event_loop = EventLoopBuilder::<()>::new().build();

    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(500));

        if tray.is_none()
            && let Event::NewEvents(StartCause::Init) = event
            && let Some(m) = menu_opt.take()
        {
            let initial_icon = if service.is_active() {
                icon_active.clone()
            } else {
                icon_inactive.clone()
            };
            tray = Some(
                TrayIconBuilder::new()
                    .with_menu(Box::new(m))
                    .with_icon(initial_icon)
                    .with_icon_as_template(true)
                    .with_tooltip("caffeine")
                    .build()
                    .expect("failed to create tray icon"),
            );
            last_active = Some(service.is_active());
        }

        // Show update banner the first time the check completes.
        if !update_shown
            && let Ok(guard) = update_result.try_lock()
            && let Some(ref tag) = *guard
        {
            items.update.set_text(format!("⬆ Update available: {tag}"));
            update_shown = true;
            if let Some(t) = tray.as_ref() {
                t.set_menu(Some(Box::new(items.build(true))));
            }
        }

        service.tick();
        service.maybe_jiggle();

        let now_active = service.is_active();
        if last_active != Some(now_active) {
            last_active = Some(now_active);
            let icon = if now_active {
                icon_active.clone()
            } else {
                icon_inactive.clone()
            };
            if let Some(t) = tray.as_ref() {
                let _ = t.set_icon_with_as_template(Some(icon), true);
            }
        }
        items.status.set_text(service.status_text());
        items.toggle.set_text(if service.is_active() {
            "Stop"
        } else {
            "Resume"
        });

        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let id = &ev.id;

            if id == items.quit.id() {
                service.shutdown();
                *control_flow = ControlFlow::Exit;
                return;
            }

            if id == items.update.id() {
                let _ = open::that("https://github.com/juliocanizalez/caffeine/releases");
                continue;
            }

            if id == items.toggle.id() {
                if service.is_active() {
                    service.deactivate();
                    service.sync_status();
                } else {
                    service.activate(None);
                    service.sync_status();
                }
                continue;
            }

            if id == items.keep_status.id() {
                service.set_jiggle_enabled(!service.jiggle_enabled);
                items.keep_status.set_checked(service.jiggle_enabled);
                service.sync_status();
                continue;
            }

            if id == items.launch_at_login.id() {
                let currently_enabled = login_mgr.is_enabled();
                if currently_enabled {
                    if let Err(e) = login_mgr.disable() {
                        eprintln!("caffeine: failed to disable login item — {e}");
                    }
                } else if let Err(e) = login_mgr.enable() {
                    eprintln!("caffeine: failed to enable login item — {e}");
                }
                items.launch_at_login.set_checked(login_mgr.is_enabled());
                continue;
            }

            let preset = if id == items.inf.id() {
                None
            } else if let Some((_, secs)) = items.presets.iter().find(|(item, _)| id == item.id()) {
                secs.map(Duration::from_secs)
            } else {
                continue;
            };

            service.activate(preset);
            service.sync_status();
        }
    });
}
