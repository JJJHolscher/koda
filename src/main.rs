//! WHITE GOLD — a generative chronicle of the historic salt roads.
//!
//! Run with `cargo run --release`. Watch a stylised medieval world — the
//! Hanseatic Baltic, the gabelle country of France, the Alpine mines, Venice,
//! and the Sahara — generate its own salt-trade history, season by season.

mod model;
mod render;
mod rng;
mod sim;
mod world;

use std::io::{self, Read, Write};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Args {
    seed: u64,
    years: u32,
    speed_ms: u64,
    wait: bool,
    headless: bool,
}

fn parse_args() -> Args {
    let mut a = Args {
        seed: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5A17),
        years: 50,
        speed_ms: 220,
        wait: true,
        headless: false,
    };
    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--seed" => {
                if let Some(v) = argv.get(i + 1).and_then(|s| s.parse().ok()) {
                    a.seed = v;
                    i += 1;
                }
            }
            "--years" => {
                if let Some(v) = argv.get(i + 1).and_then(|s| s.parse().ok()) {
                    a.years = v;
                    i += 1;
                }
            }
            "--speed" => {
                if let Some(v) = argv.get(i + 1).and_then(|s| s.parse().ok()) {
                    a.speed_ms = v;
                    i += 1;
                }
            }
            "--no-wait" => a.wait = false,
            "--headless" => {
                a.headless = true;
                a.wait = false;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    a
}

fn print_help() {
    println!(
        "WHITE GOLD — a generative chronicle of the salt roads\n\n\
         USAGE: white-gold [options]\n\
           --seed N     world seed (same seed = same history)\n\
           --years N    seasons to simulate, in years (default 50)\n\
           --speed MS   milliseconds per season (default 220)\n\
           --no-wait    skip the rules screen and begin at once\n\
           --headless   run with no live display; print the full chronicle\n\
           --help       show this message"
    );
}

fn bold(s: &str) -> String {
    format!("\x1b[1;93m{}\x1b[0m", s)
}

fn boot_screen() {
    let b = bold;
    println!("\x1b[2J\x1b[H");
    println!("\x1b[1;97m        W H I T E   G O L D\x1b[0m");
    println!("\x1b[2;37m        a generative chronicle of the historic salt roads\x1b[0m\n");
    println!("  For most of history salt was wealth itself — the only way to keep");
    println!("  meat and fish through winter. Whole empires were built on who made");
    println!("  it, who moved it, and who taxed it. You don't play a merchant; you");
    println!("  watch a little world play out its own salt history. Six rules drive");
    println!("  everything that happens. Learn their names — the Chronicle uses them.\n");

    println!("  {}  Everyone needs salt. When a town's store runs low its price", b("WHITE GOLD"));
    println!("              soars — and where salt is dearest (the deep Sahara) it is");
    println!("              quite literally traded for its weight in gold.\n");

    println!("  {}  Salt is won three ways: {} (coastal pans, richest in", b("THE SALT WORKS"), b("Sun"));
    println!("              summer), {} (mountain & desert mines, steady), and {}", b("Rock"), b("Brine"));
    println!("              (boiled springs). Each region lives by one of them.\n");

    println!("  {}  Merchants ({}) buy where salt is cheap and haul it where", b("THE LONG ROADS"), bold("+"));
    println!("              it is dear, minus the cost of the road. Trade slowly evens");
    println!("              out prices — unless the road is too long or cut by war.\n");

    println!("  {}      A heavy salt tax. The crown's coffers swell, but a dear,", b("THE GABELLE"));
    println!("              hungry town grows resentful — and will one day {}.\n", b("REVOLT"));

    println!("  {} Where the taxed price towers over a cheap neighbour,", b("THE FAUX-SAUNIERS"));
    println!("              smugglers ({}) run salt past the gabelle. They feed the poor", bold("!"));
    println!("              and starve the crown — unless they are caught and hanged.\n");

    println!("  {}   Drought, storm, war, mine-collapse, monopoly. Scarcity", b("THE LEAN YEARS"));
    println!("              ripples outward — and from it, history is made.\n");

    println!("\x1b[2;37m  Map: ~ sea   . plains   ^ mountains   : desert.  Town letters glow");
    println!("  green (cheap) -> yellow -> red -> magenta (worth its weight in gold).\x1b[0m\n");
}

fn epilogue(w: &model::World) {
    let s = &w.stats;
    println!("\n\x1b[1;97m  ═══ The Chronicle closes after {} years ═══\x1b[0m", w.year().saturating_sub(1));
    println!("  Seed {} remembered this history:", w.seed);
    println!("    {:>5} caravans delivered along the long roads", s.deliveries);
    println!("    {:>5} loads run past the gabelle by faux-sauniers", s.smuggled);
    println!("    {:>5} smugglers caught and condemned", s.busts);
    println!("    {:>5} caravans lost to the desert", s.caravans_lost);
    println!("    {:>5} revolts thrown up against the salt tax", s.revolts);
    println!("    {:>8.0} in gold paid for salt at Timbuktu", s.gold_to_timbuktu);
    if !s.peak_unrest_town.is_empty() {
        println!(
            "    highest unrest: {:.0}/100 at {}",
            s.peak_unrest, s.peak_unrest_town
        );
    }
    println!("\n\x1b[2;37m  Re-run with --seed {} to relive it, or pick a new seed for a new world.\x1b[0m", w.seed);
}

fn main() {
    let args = parse_args();

    if args.wait {
        boot_screen();
        print!("  \x1b[1mPress Enter to let the world begin...\x1b[0m");
        let _ = io::stdout().flush();
        let mut buf = [0u8; 1];
        let _ = io::stdin().read(&mut buf);
    }

    let mut w = world::build(args.seed);
    let total = args.years * model::TICKS_PER_YEAR;

    if args.headless {
        for _ in 0..total {
            sim::tick(&mut w);
        }
        println!("WHITE GOLD — headless run, seed {}, {} years\n", w.seed, args.years);
        for line in &w.chronicle {
            // Strip colour for clean logs.
            println!("{}", line);
        }
        epilogue(&w);
        return;
    }

    let mut stdout = io::stdout();
    for _ in 0..total {
        sim::tick(&mut w);
        let _ = write!(stdout, "{}{}", render::clear(), render::frame(&w));
        let _ = stdout.flush();
        sleep(Duration::from_millis(args.speed_ms));
    }
    epilogue(&w);
}
