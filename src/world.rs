//! World generation: the map terrain and the ten salt towns that anchor the
//! simulation. Each town is a compressed piece of real salt history.

use crate::model::*;
use crate::rng::Rng;

/// Cheap deterministic per-cell hash, used to ragged the coastlines.
fn cell_noise(seed: u64, x: usize, y: usize) -> f32 {
    let mut h = seed ^ 0xD1B5_4A32_D192_ED03;
    h ^= (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h = h.rotate_left(27);
    h ^= (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h = h.wrapping_mul(0x1656_67B1_9E37_79F9);
    ((h >> 40) as f32) / ((1u64 << 24) as f32)
}

fn gen_terrain(seed: u64) -> Vec<Terrain> {
    let mut t = vec![Terrain::Plain; MAP_W * MAP_H];
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let n = cell_noise(seed, x, y);
            let cell = classify(x, y, n);
            t[y * MAP_W + x] = cell;
        }
    }
    t
}

fn classify(x: usize, y: usize, n: f32) -> Terrain {
    let (xf, yf) = (x as f32, y as f32);

    // Northern Sea (Baltic / North Sea) across the top, with a ragged shore.
    if yf <= 2.0 + n * 1.5 {
        return Terrain::Sea;
    }
    // Atlantic down the upper-left flank.
    if xf <= 4.0 + n * 2.0 && yf <= 14.0 {
        return Terrain::Sea;
    }
    // Adriatic pocket (Venice's lagoon) on the centre-right.
    let adx = xf - 57.0;
    let ady = yf - 14.0;
    if adx * adx * 0.6 + ady * ady * 1.1 <= 9.0 + n * 4.0 {
        return Terrain::Sea;
    }
    // The Mediterranean: a band separating Europe from Africa.
    if (16.5..=18.2).contains(&(yf - n * 1.2)) {
        return Terrain::Sea;
    }
    // The Sahara along the bottom.
    if yf >= 18.5 + n * 1.0 {
        return Terrain::Desert;
    }
    // The Alps: a ragged massif through the centre.
    let mdx = xf - 47.0;
    let mdy = yf - 11.5;
    if mdx * mdx * 0.10 + mdy * mdy * 0.55 <= 5.0 + n * 3.0 {
        return Terrain::Mountain;
    }
    Terrain::Plain
}

fn town(
    name: &'static str,
    realm: &'static str,
    x: f32,
    y: f32,
    coastal: bool,
    works: Works,
    base_production: f32,
    base_demand: f32,
    fishery: f32,
    base_tax: f32,
    enforces_monopoly: bool,
) -> Town {
    let start_demand = (base_demand + 0.4 * fishery).max(0.1);
    Town {
        name,
        realm,
        x,
        y,
        coastal,
        works,
        base_production,
        base_demand,
        fishery,
        stock: start_demand * TARGET_COVERAGE,
        tax: base_tax,
        base_tax,
        enforces_monopoly,
        treasury: 0.0,
        discontent: 0.0,
        revolt_cooldown: 0,
        cornered: 0,
        disrupted: 0,
    }
}

pub fn build(seed: u64) -> World {
    let terrain = gen_terrain(seed);

    // Ten towns, each a thumbnail of real salt history.
    let towns = vec![
        // Hanseatic north: Lübeck salts the Baltic herring; Lüneburg's brine
        // springs are the source that fed it down the Old Salt Route.
        town("Lubeck", "Hansa", 40.0, 3.0, true, Works::None, 0.0, 6.0, 16.0, 0.05, false),
        town("Luneburg", "Hansa", 37.0, 7.0, false, Works::Brine, 19.0, 4.0, 0.0, 0.05, false),
        // Atlantic France: Brouage's solar marshes are bountiful and lightly
        // taxed; Anjou inland groans under the gabelle.
        town("Brouage", "Couronne", 6.0, 11.0, true, Works::Sun, 24.0, 4.0, 3.0, 0.10, false),
        town("Anjou", "Couronne", 15.0, 9.0, false, Works::None, 0.0, 9.0, 0.0, 1.40, false),
        // The Alps: rock-salt mines on the Salzstrasse.
        town("Salzburg", "Reich", 45.0, 11.0, false, Works::Rock, 17.0, 5.0, 0.0, 0.20, false),
        town("Hallstatt", "Reich", 50.0, 13.0, false, Works::Rock, 12.0, 3.0, 0.0, 0.15, false),
        // Venice: solar pans plus a jealously guarded monopoly.
        town("Venezia", "Serenissima", 55.0, 15.0, true, Works::Sun, 21.0, 7.0, 4.0, 0.35, true),
        // Sicilian (Trapani) sun-salt, a great Mediterranean exporter.
        town("Trapani", "Sicilia", 48.0, 17.0, true, Works::Sun, 22.0, 3.0, 2.0, 0.10, false),
        // The Sahara: Taghaza's slabs of mined rock salt...
        town("Taghaza", "Sahel", 33.0, 21.0, false, Works::Rock, 15.0, 1.0, 0.0, 0.0, false),
        // ...hauled by caravan to Timbuktu, where salt meets its weight in gold.
        town("Timbuktu", "Mali", 24.0, 22.0, false, Works::None, 0.0, 7.0, 0.0, 0.0, false),
    ];

    let mut terrain = terrain;
    // Make sure no town sits in open water; give it land that fits its setting.
    for tn in &towns {
        let idx = tn.y as usize * MAP_W + tn.x as usize;
        let land = if tn.y >= 18.5 {
            Terrain::Desert
        } else if tn.works == Works::Rock {
            Terrain::Mountain
        } else {
            Terrain::Plain
        };
        terrain[idx] = land;
    }

    World {
        rng: Rng::new(seed),
        seed,
        tick: 0,
        terrain,
        towns,
        caravans: Vec::new(),
        chronicle: Vec::new(),
        stats: Stats::default(),
        drought: 0,
        war: None,
        last_event_gap: 0,
    }
}

/// The dominant terrain a route crosses, used to price the journey.
pub fn route_cost_per_cell(a: &Town, b: &Town) -> f32 {
    if a.coastal && b.coastal {
        COST_SEA
    } else if a.y >= 18.5 || b.y >= 18.5 {
        COST_DESERT
    } else {
        COST_LAND
    }
}
