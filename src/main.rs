use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use caffeine::application::CaffeineService;
use caffeine::domain::ports::StatusRepository;
use caffeine::infrastructure::{
    ipc::FileStatusRepository,
    jiggle::{CoreGraphicsIdleDetector, CoreGraphicsJiggler},
    power::IokitPowerManager,
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
    Status,
    /// Stop the running caffeine instance
    Stop,
}

// ── Status / stop handlers (exit before the event loop) ──────────────────────

fn cmd_status(repo: &FileStatusRepository) {
    match repo.read() {
        None => println!("○ Not running"),
        Some(s) => {
            if !repo.is_alive(s.pid) {
                repo.delete();
                println!("○ Not running");
                return;
            }
            let now = repo.now_secs();
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

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args = Args::parse();

    let repo = FileStatusRepository;

    match &args.command {
        Some(Command::Status) => {
            cmd_status(&repo);
            return;
        }
        Some(Command::Stop) => {
            cmd_stop(&repo);
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

    // ── Build menu items ──────────────────────────────────────────────────────

    let item_status = MenuItem::new("Active · indefinite", false, None);
    let item_15m = MenuItem::new("15 minutes", true, None);
    let item_30m = MenuItem::new("30 minutes", true, None);
    let item_1h = MenuItem::new("1 hour", true, None);
    let item_2h = MenuItem::new("2 hours", true, None);
    let item_4h = MenuItem::new("4 hours", true, None);
    let item_inf = MenuItem::new("Indefinite", true, None);
    let item_keep_status =
        CheckMenuItem::new("Keep Status Active", true, args.keep_status_active, None);
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
    menu.append(&item_keep_status).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&item_toggle).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&item_quit).unwrap();

    // ── Wire service ──────────────────────────────────────────────────────────

    let mut service = CaffeineService::new(
        Box::new(IokitPowerManager),
        Box::new(CoreGraphicsIdleDetector),
        Box::new(CoreGraphicsJiggler),
        Box::new(FileStatusRepository),
        !args.no_display,
        args.keep_status_active,
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
        item_status.set_text(service.status_text());
        item_toggle.set_text(if service.is_active() {
            "Stop"
        } else {
            "Resume"
        });

        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let id = &ev.id;

            if id == item_quit.id() {
                service.shutdown();
                *control_flow = ControlFlow::Exit;
                return;
            }

            if id == item_toggle.id() {
                if service.is_active() {
                    service.deactivate();
                    service.sync_status();
                } else {
                    service.activate(None);
                    service.sync_status();
                }
                return;
            }

            if id == item_keep_status.id() {
                service.set_jiggle_enabled(!service.jiggle_enabled);
                item_keep_status.set_checked(service.jiggle_enabled);
                service.sync_status();
                continue;
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

            service.activate(preset);
            service.sync_status();
        }
    });
}
