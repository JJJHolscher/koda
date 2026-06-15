//! Core data types and the tuning constants that give the world its feel.

use crate::rng::Rng;

pub const MAP_W: usize = 72;
pub const MAP_H: usize = 23;

/// Four seasons make a year. Seasonality is a first-class driver: summer suns
/// the salt pans, autumn brings the herring that must be salted, winter slows
/// the roads.
pub const TICKS_PER_YEAR: u32 = 4;

// --- Economic tuning -------------------------------------------------------

/// A comfortable buffer is this many ticks of demand in the storehouse. Stock
/// below it pushes the price up; above it, down.
pub const TARGET_COVERAGE: f32 = 18.0;
pub const BASE_PRICE: f32 = 14.0;
pub const PRICE_FLOOR: f32 = 2.0;
pub const PRICE_CEIL: f32 = 320.0;
/// Storehouses are finite; production beyond this spoils or is left in the pan.
pub const STOCK_CAP: f32 = 900.0;

/// Travel cost per measure, per map cell, by terrain the road crosses.
pub const COST_SEA: f32 = 0.5; // ships are cheap (the Hanseatic edge)
pub const COST_LAND: f32 = 1.6;
pub const COST_DESERT: f32 = 3.4; // the long, deadly haul to Timbuktu

/// A merchant only sets out if the expected margin per measure clears this.
pub const MIN_MARGIN: f32 = 4.0;
/// How many measures a single caravan carries.
pub const CARAVAN_LOAD: f32 = 60.0;
pub const MAX_CARAVANS: usize = 14;

/// Smugglers appear once the taxed price exceeds a cheap neighbour by this gap.
pub const SMUGGLE_GAP: f32 = 30.0;

/// Discontent thresholds.
pub const REVOLT_AT: f32 = 90.0;
pub const REVOLT_COOLDOWN: u32 = 6; // ticks the gabelle stays abolished

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Works {
    None,
    Sun,
    Rock,
    Brine,
}

impl Works {
    pub fn tag(self) -> &'static str {
        match self {
            Works::None => "---",
            Works::Sun => "Sun",
            Works::Rock => "Rok",
            Works::Brine => "Bri",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Terrain {
    Sea,
    Plain,
    Mountain,
    Desert,
}

pub struct Town {
    pub name: &'static str,
    pub realm: &'static str,
    pub x: f32,
    pub y: f32,
    pub coastal: bool,
    pub works: Works,
    pub base_production: f32,
    pub base_demand: f32,
    /// Extra demand at the autumn herring catch (0 for towns with no fishery).
    pub fishery: f32,

    pub stock: f32,
    /// The gabelle: a multiplicative tax on the consumer price.
    pub tax: f32,
    /// The gabelle the crown *wants* to levy; `tax` may be 0 during a revolt.
    pub base_tax: f32,
    pub enforces_monopoly: bool,

    pub treasury: f32,
    pub discontent: f32,
    pub revolt_cooldown: u32,
    /// Set while a foreign power (Venice) has cornered this town's output.
    pub cornered: u32,
    /// Ticks remaining of a production halt (mine collapse, frozen pans).
    pub disrupted: u32,
}

impl Town {
    /// Untaxed market price, driven purely by scarcity.
    pub fn base_price(&self) -> f32 {
        let demand = (self.base_demand + 0.4 * self.fishery).max(0.1);
        let coverage = self.stock / demand;
        let p = BASE_PRICE * (TARGET_COVERAGE / (coverage + 1.0));
        p.clamp(PRICE_FLOOR, PRICE_CEIL)
    }

    /// What the people actually pay, gabelle included.
    pub fn consumer_price(&self) -> f32 {
        (self.base_price() * (1.0 + self.tax)).min(PRICE_CEIL)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CaravanKind {
    Merchant,
    Smuggler,
}

pub struct Caravan {
    pub kind: CaravanKind,
    pub from: usize,
    pub to: usize,
    pub x: f32,
    pub y: f32,
    pub cargo: f32,
    pub buy_price: f32,
    /// Cells advanced per tick.
    pub speed: f32,
    pub dead: bool,
}

/// Running tallies, used for the closing chronicle.
#[derive(Default)]
pub struct Stats {
    pub deliveries: u32,
    pub smuggled: u32,
    pub busts: u32,
    pub revolts: u32,
    pub caravans_lost: u32,
    pub gold_to_timbuktu: f32,
    pub peak_unrest: f32,
    pub peak_unrest_town: &'static str,
}

pub struct World {
    pub rng: Rng,
    pub seed: u64,
    pub tick: u32,
    pub terrain: Vec<Terrain>,
    pub towns: Vec<Town>,
    pub caravans: Vec<Caravan>,
    pub chronicle: Vec<String>,
    pub stats: Stats,

    // Active world events.
    pub drought: u32,
    pub war: Option<(&'static str, &'static str, u32)>,
    pub last_event_gap: u32,
}

impl World {
    pub fn year(&self) -> u32 {
        1 + self.tick / TICKS_PER_YEAR
    }

    pub fn season(&self) -> Season {
        match self.tick % TICKS_PER_YEAR {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Autumn,
            _ => Season::Winter,
        }
    }

    pub fn date_label(&self) -> String {
        format!("Year {:>2}, {}", self.year(), self.season().name())
    }

    /// Push a sentence into the running chronicle of the world.
    pub fn record(&mut self, line: String) {
        self.chronicle.push(format!("[{}] {}", self.date_label(), line));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn name(self) -> &'static str {
        match self {
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Autumn => "Autumn",
            Season::Winter => "Winter",
        }
    }

    /// Multiplier on Sun salt production.
    pub fn sun_factor(self) -> f32 {
        match self {
            Season::Spring => 1.0,
            Season::Summer => 1.6,
            Season::Autumn => 0.7,
            Season::Winter => 0.3,
        }
    }

    /// Multiplier on the herring/fishery demand for coastal towns.
    pub fn fishery_factor(self) -> f32 {
        match self {
            Season::Autumn => 1.0, // the great catch
            Season::Summer => 0.4,
            Season::Spring => 0.3,
            Season::Winter => 0.1,
        }
    }
}

/// Straight-line distance between two towns, in map cells.
pub fn distance(a: &Town, b: &Town) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}
