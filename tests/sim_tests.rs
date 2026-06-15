//! Behavioural tests for the salt simulation. These pin down the invariants
//! that keep the emergent chronicle sane, and assert that each of the six core
//! rules actually fires over a long run.

// The crate is a binary, so we include the modules directly for testing.
#[path = "../src/rng.rs"]
mod rng;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/world.rs"]
mod world;
#[path = "../src/sim.rs"]
mod sim;

use model::*;

/// Run a world forward and return it, so tests can inspect the outcome.
fn run(seed: u64, years: u32) -> World {
    let mut w = world::build(seed);
    for _ in 0..years * TICKS_PER_YEAR {
        sim::tick(&mut w);
    }
    w
}

#[test]
fn stock_and_price_stay_in_bounds() {
    let w = run(42, 80);
    for t in &w.towns {
        assert!(t.stock >= 0.0, "{} had negative stock", t.name);
        assert!(t.stock <= STOCK_CAP + 1.0, "{} exceeded the stock cap", t.name);
        let cp = t.consumer_price();
        assert!(
            cp >= PRICE_FLOOR - 0.01 && cp <= PRICE_CEIL + 0.01,
            "{} price {} out of bounds",
            t.name,
            cp
        );
        assert!(t.discontent >= 0.0, "{} had negative discontent", t.name);
    }
}

#[test]
fn determinism_same_seed_same_history() {
    let a = run(123, 40);
    let b = run(123, 40);
    assert_eq!(a.chronicle, b.chronicle, "same seed must yield same chronicle");
    assert_eq!(a.stats.deliveries, b.stats.deliveries);
    assert_eq!(a.stats.revolts, b.stats.revolts);
}

#[test]
fn different_seeds_diverge() {
    let a = run(1, 40);
    let b = run(2, 40);
    assert_ne!(a.chronicle, b.chronicle, "different seeds should tell different stories");
}

#[test]
fn the_long_roads_move_salt() {
    // Trade must happen: caravans deliver salt across the map.
    let w = run(7, 50);
    assert!(w.stats.deliveries > 5, "expected merchant deliveries, got {}", w.stats.deliveries);
}

#[test]
fn timbuktu_pays_in_gold() {
    // White Gold: the far desert market should command enormous prices.
    let w = run(7, 60);
    assert!(w.stats.gold_to_timbuktu > 0.0, "no salt ever reached Timbuktu");
    let timbuktu = w.towns.iter().find(|t| t.name == "Timbuktu").unwrap();
    let brouage = w.towns.iter().find(|t| t.name == "Brouage").unwrap();
    // Salt at the desert market is dearer than at a coastal salt-pan.
    assert!(
        timbuktu.consumer_price() > brouage.consumer_price(),
        "Timbuktu ({:.0}) should be dearer than Brouage ({:.0})",
        timbuktu.consumer_price(),
        brouage.consumer_price()
    );
}

#[test]
fn the_gabelle_fills_the_treasury() {
    // A taxed town should collect gabelle revenue over time.
    let w = run(42, 50);
    let anjou = w.towns.iter().find(|t| t.name == "Anjou").unwrap();
    assert!(anjou.treasury > 0.0, "the gabelle collected no revenue");
}

#[test]
fn faux_sauniers_and_revolts_emerge_somewhere() {
    // Across a spread of seeds, both smuggling and at least one revolt occur,
    // proving those rules are reachable rather than dead code.
    let mut total_smuggled = 0;
    let mut total_revolts = 0;
    for seed in [7u64, 42, 100, 13, 99] {
        let w = run(seed, 60);
        total_smuggled += w.stats.smuggled + w.stats.busts;
        total_revolts += w.stats.revolts;
    }
    assert!(total_smuggled > 0, "faux-sauniers never appeared");
    assert!(total_revolts > 0, "the gabelle never provoked a revolt");
}
