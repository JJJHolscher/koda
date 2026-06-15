//! The simulation: one `tick` advances the salt world by a single season.
//!
//! Everything emergent in the chronicle comes out of the six rules applied in
//! order here — production, demand & the gabelle, revolts, the long roads,
//! the faux-sauniers, and the lean years.

use crate::model::*;
use crate::world::route_cost_per_cell;

const VENICE: usize = 6;
const TIMBUKTU: usize = 9;

pub fn tick(w: &mut World) {
    let season = w.season();
    production(w, season);
    consumption(w, season);
    taxes_and_revolts(w);
    move_caravans(w);
    spawn_merchants(w);
    spawn_smugglers(w);
    lean_years(w);
    cool_down(w);
    w.tick += 1;
}

// --- The Salt Works --------------------------------------------------------

fn production(w: &mut World, season: Season) {
    // Compute each town's output, then deposit it — redirecting the output of
    // any "cornered" works into Venice's storehouses (The Monopoly).
    let mut deposits: Vec<(usize, f32)> = Vec::new();
    for (i, t) in w.towns.iter().enumerate() {
        if t.works == Works::None || t.disrupted > 0 {
            continue;
        }
        let mult = match t.works {
            Works::Sun => season.sun_factor() * if w.drought > 0 { 0.2 } else { 1.0 },
            Works::Rock => 1.0,
            Works::Brine => if w.drought > 0 { 0.85 } else { 1.0 },
            Works::None => 0.0,
        };
        let amount = t.base_production * mult;
        let target = if t.cornered > 0 { VENICE } else { i };
        deposits.push((target, amount));
    }
    for (idx, amount) in deposits {
        let t = &mut w.towns[idx];
        t.stock = (t.stock + amount).min(STOCK_CAP);
    }
}

// --- White Gold & The Gabelle ---------------------------------------------

fn consumption(w: &mut World, season: Season) {
    for t in w.towns.iter_mut() {
        let demand = t.base_demand + t.fishery * season.fishery_factor();
        let bought = demand.min(t.stock);
        let deficit = demand - bought;
        t.stock -= bought;

        // The crown takes its gabelle on every measure actually sold.
        let base = t.base_price();
        t.treasury += bought * base * t.tax;

        // Discontent rises from three things: the simmering resentment of the
        // gabelle itself (hated whether or not salt is plentiful), an
        // unaffordable price, and outright shortage. It cools in cool_down.
        let cp = base * (1.0 + t.tax);
        let gabelle_resentment = t.tax * 1.1;
        let price_pain = (cp - 24.0).max(0.0) * 0.012 * (0.4 + t.tax);
        let hunger_pain = deficit * 2.0;
        t.discontent = (t.discontent + gabelle_resentment + price_pain + hunger_pain).max(0.0);
    }
}

fn taxes_and_revolts(w: &mut World) {
    let mut events: Vec<String> = Vec::new();
    for t in w.towns.iter() {
        // Only taxed towns have a gabelle to revolt against.
        if t.base_tax > 0.0 && t.discontent > w.stats.peak_unrest {
            w.stats.peak_unrest = t.discontent;
            w.stats.peak_unrest_town = t.name;
        }
    }
    for t in w.towns.iter_mut() {
        if t.revolt_cooldown > 0 {
            t.revolt_cooldown -= 1;
            if t.revolt_cooldown == 0 && t.base_tax > 0.0 {
                t.tax = t.base_tax;
                events.push(format!(
                    "The crown reimposes the gabelle on {} ({:.0}%). (Gabelle)",
                    t.name,
                    t.base_tax * 100.0
                ));
            }
            continue;
        }
        if t.base_tax > 0.0 && t.discontent >= REVOLT_AT {
            // Revolt! The gabelle is thrown down and the coffers ransacked.
            t.tax = 0.0;
            t.revolt_cooldown = REVOLT_COOLDOWN;
            t.discontent *= 0.35;
            t.treasury *= 0.3;
            let rebels = rebel_name(t.realm);
            events.push(format!(
                "{} rise in {}! The gabelle is cast down and the salt-store thrown open. (Revolt)",
                rebels, t.name
            ));
            w.stats.revolts += 1;
        }
    }
    for e in events {
        w.record(e);
    }
}

fn rebel_name(realm: &str) -> &'static str {
    match realm {
        "Couronne" => "The Pitauds",
        "Serenissima" => "The guildsmen",
        "Reich" => "The miners",
        _ => "The townsfolk",
    }
}

// --- The Long Roads (caravan movement & delivery) --------------------------

fn move_caravans(w: &mut World) {
    let mut log: Vec<String> = Vec::new();
    let n = w.caravans.len();
    for ci in 0..n {
        let (kind, to, tx, ty, speed, from);
        {
            let c = &w.caravans[ci];
            kind = c.kind;
            to = c.to;
            from = c.from;
            speed = c.speed;
            tx = w.towns[to].x;
            ty = w.towns[to].y;
        }

        // Advance toward the destination.
        let (cx, cy) = { let c = &w.caravans[ci]; (c.x, c.y) };
        let (dx, dy) = (tx - cx, ty - cy);
        let dist = (dx * dx + dy * dy).sqrt();

        // Perils of the road: sandstorms on the desert haul.
        let desert_route = w.towns[from].y >= 18.5 || to_is_desert(w, to);
        if desert_route && w.rng.chance(0.04) {
            w.caravans[ci].dead = true;
            w.stats.caravans_lost += 1;
            log.push(format!(
                "A sandstorm swallows a caravan bound for {}; salt and souls lost to the dunes. (Lean Years)",
                w.towns[to].name
            ));
            continue;
        }

        // The crown hunts smugglers, fiercest where a monopoly is enforced.
        if kind == CaravanKind::Smuggler {
            let mut bust = 0.05;
            if w.towns[to].enforces_monopoly {
                bust += 0.08;
            }
            if dist < 4.0 {
                bust += 0.06; // patrols thicken near the gates
            }
            if w.rng.chance(bust) {
                let cargo = w.caravans[ci].cargo;
                w.caravans[ci].dead = true;
                w.stats.busts += 1;
                w.towns[to].treasury += cargo * 6.0; // seized and sold
                log.push(format!(
                    "A faux-saunier is taken at the gates of {} — the salt seized, the smuggler bound for the galleys. (Faux-Sauniers)",
                    w.towns[to].name
                ));
                continue;
            }
        }

        if dist <= speed.max(1.0) {
            // Arrived: unload.
            let cargo = w.caravans[ci].cargo;
            let buy_price = w.caravans[ci].buy_price;
            w.caravans[ci].dead = true;
            let dest = &mut w.towns[to];
            dest.stock = (dest.stock + cargo).min(STOCK_CAP);

            match kind {
                CaravanKind::Merchant => {
                    w.stats.deliveries += 1;
                    if to == TIMBUKTU {
                        let gold = cargo * dest.base_price();
                        w.stats.gold_to_timbuktu += gold;
                        log.push(format!(
                            "A caravan crosses to Timbuktu: {:.0} measures of salt traded for their weight in gold. (White Gold)",
                            cargo
                        ));
                    } else if buy_price < 8.0 {
                        log.push(format!(
                            "Merchants land {:.0} measures of cheap salt at {}, easing the price. (Long Roads)",
                            cargo, dest.name
                        ));
                    }
                }
                CaravanKind::Smuggler => {
                    w.stats.smuggled += 1;
                    // Smuggled salt fills bellies but does not quiet the
                    // political grievance against the tax — only a little relief.
                    dest.discontent = (dest.discontent - cargo * 0.12).max(0.0);
                    log.push(format!(
                        "Contraband salt reaches {} untaxed — the poor are fed, the crown's coffers bleed. (Faux-Sauniers)",
                        dest.name
                    ));
                }
            }
            continue;
        }

        // Step toward the target.
        let step = speed / dist;
        let c = &mut w.caravans[ci];
        c.x += dx * step;
        c.y += dy * step;
    }
    w.caravans.retain(|c| !c.dead);
    for l in log {
        w.record(l);
    }
}

fn to_is_desert(w: &World, idx: usize) -> bool {
    w.towns[idx].y >= 18.5
}

// --- Spawning merchants (The Long Roads) -----------------------------------

fn war_blocks(w: &World, a: usize, b: usize) -> bool {
    if let Some((ra, rb, _)) = w.war {
        let (ta, tb) = (w.towns[a].realm, w.towns[b].realm);
        (ta == ra && tb == rb) || (ta == rb && tb == ra)
    } else {
        false
    }
}

fn spawn_merchants(w: &mut World) {
    if w.caravans.len() >= MAX_CARAVANS {
        return;
    }
    // Greedily pick the single most profitable buy-low / sell-high route this
    // season; the act of trading nudges both prices, so the market self-limits.
    let mut best: Option<(usize, usize, f32)> = None;
    for a in 0..w.towns.len() {
        let sa = &w.towns[a];
        // Producers keep a reserve; only true surplus goes to market.
        let reserve = (sa.base_demand + 0.4 * sa.fishery) * TARGET_COVERAGE;
        if sa.stock <= reserve + 5.0 {
            continue;
        }
        if sa.cornered > 0 {
            continue;
        }
        let pa = sa.base_price();
        for b in 0..w.towns.len() {
            if a == b || war_blocks(w, a, b) {
                continue;
            }
            let sb = &w.towns[b];
            let pb = sb.base_price();
            let cost = distance(sa, sb) * route_cost_per_cell(sa, sb);
            let margin = pb - pa - cost;
            if margin > MIN_MARGIN {
                if best.map_or(true, |(_, _, m)| margin > m) {
                    best = Some((a, b, margin));
                }
            }
        }
    }

    if let Some((a, b, _)) = best {
        let reserve = (w.towns[a].base_demand + 0.4 * w.towns[a].fishery) * TARGET_COVERAGE;
        let load = CARAVAN_LOAD.min(w.towns[a].stock - reserve);
        if load < 5.0 {
            return;
        }
        let buy_price = w.towns[a].base_price();
        w.towns[a].stock -= load;
        let speed = leg_speed(&w.towns[a], &w.towns[b]);
        let (x, y) = (w.towns[a].x, w.towns[a].y);
        w.caravans.push(Caravan {
            kind: CaravanKind::Merchant,
            from: a,
            to: b,
            x,
            y,
            cargo: load,
            buy_price,
            speed,
            dead: false,
        });
    }
}

fn leg_speed(a: &Town, b: &Town) -> f32 {
    if a.coastal && b.coastal {
        3.2 // ships
    } else if a.y >= 18.5 || b.y >= 18.5 {
        1.2 // desert plod
    } else {
        2.0 // cart on the road
    }
}

// --- The Faux-Sauniers (smuggling) -----------------------------------------

fn spawn_smugglers(w: &mut World) {
    if w.caravans.len() >= MAX_CARAVANS {
        return;
    }
    // For each heavily taxed, dear town, see whether a cheap neighbour makes
    // smuggling worth the noose.
    for b in 0..w.towns.len() {
        let tb = &w.towns[b];
        if tb.tax < 0.25 {
            continue;
        }
        let taxed_price = tb.consumer_price();
        let mut best_src: Option<usize> = None;
        let mut best_gap = SMUGGLE_GAP;
        for a in 0..w.towns.len() {
            if a == b {
                continue;
            }
            let sa = &w.towns[a];
            let reserve = (sa.base_demand + 0.4 * sa.fishery) * TARGET_COVERAGE * 0.6;
            if sa.stock <= reserve {
                continue;
            }
            let cost = distance(sa, &w.towns[b]) * route_cost_per_cell(sa, &w.towns[b]);
            let gap = taxed_price - sa.base_price() - cost;
            if gap > best_gap {
                best_gap = gap;
                best_src = Some(a);
            }
        }
        if let Some(a) = best_src {
            if w.rng.chance(0.45) && w.caravans.len() < MAX_CARAVANS {
                let load = 38.0_f32.min(w.towns[a].stock * 0.3);
                if load < 5.0 {
                    continue;
                }
                let buy_price = w.towns[a].base_price();
                w.towns[a].stock -= load;
                let speed = leg_speed(&w.towns[a], &w.towns[b]);
                let (x, y) = (w.towns[a].x, w.towns[a].y);
                w.caravans.push(Caravan {
                    kind: CaravanKind::Smuggler,
                    from: a,
                    to: b,
                    x,
                    y,
                    cargo: load,
                    buy_price,
                    speed,
                    dead: false,
                });
                let msg = format!(
                    "Smugglers slip out of {} for {}, salt hidden beneath the cargo. (Faux-Sauniers)",
                    w.towns[a].name, w.towns[b].name
                );
                w.record(msg);
            }
        }
    }
}

// --- The Lean Years (world events) -----------------------------------------

fn lean_years(w: &mut World) {
    w.last_event_gap += 1;
    let p = 0.05 + w.last_event_gap as f32 * 0.012;
    if w.last_event_gap < 2 || !w.rng.chance(p) {
        return;
    }
    w.last_event_gap = 0;

    match w.rng.below(6) {
        0 => {
            // Drought
            w.drought = 3 + w.rng.below(4) as u32;
            w.record("A scorching drought withers the salt pans across the land. (Lean Years)".into());
        }
        1 => {
            // Storm wrecks a harbour
            let coastal: Vec<usize> = (0..w.towns.len()).filter(|&i| w.towns[i].coastal).collect();
            let t = coastal[w.rng.below(coastal.len())];
            w.towns[t].stock *= 0.55;
            let name = w.towns[t].name;
            w.record(format!(
                "A great storm wrecks the harbour at {}; salt and herring are lost to the waves. (Lean Years)",
                name
            ));
        }
        2 => {
            // War closes the roads between two realms
            let realms = ["Hansa", "Couronne", "Reich", "Serenissima", "Sicilia"];
            let ra = realms[w.rng.below(realms.len())];
            let mut rb = realms[w.rng.below(realms.len())];
            let mut guard = 0;
            while rb == ra && guard < 8 {
                rb = realms[w.rng.below(realms.len())];
                guard += 1;
            }
            if rb != ra {
                w.war = Some((ra, rb, 4 + w.rng.below(4) as u32));
                w.record(format!(
                    "War flares between {} and {}; the salt roads between them are closed. (Lean Years)",
                    ra, rb
                ));
            }
        }
        3 => {
            // Mine collapse
            let mines: Vec<usize> =
                (0..w.towns.len()).filter(|&i| w.towns[i].works == Works::Rock).collect();
            let t = mines[w.rng.below(mines.len())];
            w.towns[t].disrupted = 3 + w.rng.below(4) as u32;
            let name = w.towns[t].name;
            w.record(format!(
                "A gallery collapses in the salt mine of {}; the works fall silent. (Lean Years)",
                name
            ));
        }
        4 => {
            // The Monopoly: Venice corners a Mediterranean producer
            if w.towns[VENICE].stock > 20.0 {
                let target = 7; // Trapani
                w.towns[target].cornered = 4 + w.rng.below(4) as u32;
                w.record(
                    "Venice corners the salt of Trapani, buying up the pans to drive the Adriatic price. (Monopoly)"
                        .into(),
                );
            }
        }
        _ => {
            // The Great Catch: a sudden glut of herring demands salt now
            let fisheries: Vec<usize> =
                (0..w.towns.len()).filter(|&i| w.towns[i].fishery > 5.0).collect();
            if !fisheries.is_empty() {
                let t = fisheries[w.rng.below(fisheries.len())];
                let drain = w.towns[t].fishery * 4.0;
                w.towns[t].stock = (w.towns[t].stock - drain).max(0.0);
                let name = w.towns[t].name;
                w.record(format!(
                    "A vast herring shoal runs at {}; every measure of salt is wanted to cure the catch. (White Gold)",
                    name
                ));
            }
        }
    }
}

// --- Timers & cooling ------------------------------------------------------

fn cool_down(w: &mut World) {
    if w.drought > 0 {
        w.drought -= 1;
    }
    if let Some((ra, rb, t)) = w.war {
        if t <= 1 {
            w.war = None;
            w.record(format!("Peace is sworn between {} and {}; the roads reopen.", ra, rb));
        } else {
            w.war = Some((ra, rb, t - 1));
        }
    }
    for t in w.towns.iter_mut() {
        if t.cornered > 0 {
            t.cornered -= 1;
        }
        if t.disrupted > 0 {
            t.disrupted -= 1;
        }
        // Discontent slowly cools.
        t.discontent *= 0.985;
    }
}
