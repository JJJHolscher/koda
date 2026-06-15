# WHITE GOLD — a generative chronicle of the historic salt roads

A tiny, dependency-free Rust program that simulates a stylised medieval world
and lets a **salt-trade history write itself**. You don't play a merchant; you
watch a little world play out its own salt history, season by season, narrated
in a live chronicle. Find a seed you like and the whole history is reproducible.

```
cargo run --release
```

Press **Enter** at the rules screen and watch. `Ctrl-C` to leave.

```
cargo run --release -- --seed 7        # a specific world
cargo run --release -- --speed 120     # faster (ms per season)
cargo run --release -- --years 80      # a longer history
cargo run --release -- --headless      # no live view; print the full chronicle
cargo run --release -- --help          # all options
```

There are **no external dependencies** — the whole thing is `std`-only and
renders to the terminal with ANSI escapes, so it builds and runs anywhere
`cargo` does, with no GPU or windowing libraries.

---

## Why salt?

For most of recorded history salt was not a seasoning but a survival
technology: the only way to keep meat and fish edible through winter. That made
it strategic, and whoever controlled its making, moving, and *taxing* grew
rich. The research behind this game drew out a handful of recurring patterns
across very different times and places:

- **Three ways to make it.** Solar evaporation in coastal pans (oldest method,
  best in hot dry summers), rock-salt mining in mountains and the desert, and
  boiling **brine** from salt springs. Lüneburg's brine, Salzburg's mines on the
  *Salzstraße*, and the Mediterranean salt pans each lived by one method.
- **The salted catch.** The Hanseatic League's whole northern economy ran on
  salting the autumn Baltic herring with Lüneburg salt shipped to Lübeck —
  salt's demand was tied to the food it preserved.
- **The gabelle.** France's salt tax was so heavy and so regionally uneven that
  it became one of history's most hated levies, a grievance that fed into the
  French Revolution and sparked earlier risings like the **Revolt of the
  Pitauds**.
- **The faux-sauniers.** Wherever a taxed region bordered a cheap one, salt
  smuggling ("faux-saunage") became endemic — buy low across the border, sell
  high past the tax, risk the galleys.
- **The monopoly.** Venice built an empire by cornering salt production across
  the Adriatic and Mediterranean and ruthlessly suppressing competition and
  smuggling.
- **Worth its weight in gold.** Across the Sahara, Tuareg caravans hauled slabs
  of rock salt from mines like Taghaza to Timbuktu, where in some places salt
  was quite literally traded for its weight in gold.

That history is boiled down into six core rules.

---

## The six rules (also shown at boot-up)

Each rule has a **bolded shorthand** — the chronicle tags every event with one,
so you can read the emerging story in terms of them.

1. **White Gold** — Everyone needs salt. When a town's store runs low its price
   soars; where salt is dearest (the deep Sahara) it is traded for its weight in
   gold.
2. **The Salt Works** — Salt is won three ways: **Sun** (coastal pans, richest
   in summer), **Rock** (mountain & desert mines, steady), and **Brine** (boiled
   springs). Each region lives by one.
3. **The Long Roads** — Merchants (`+`) buy where salt is cheap and haul it
   where it is dear, minus the cost of the journey. Trade slowly evens out
   prices — unless the road is too long (the desert to Timbuktu) or cut by war.
4. **The Gabelle** — A heavy salt tax. The crown's coffers swell, but a dear,
   hungry town grows resentful — and will one day **Revolt** (the Pitauds rise,
   the gabelle is cast down, until the crown reimposes it).
5. **The Faux-Sauniers** — Where the taxed price towers over a cheap neighbour,
   smugglers (`!`) run salt past the gabelle. They feed the poor and starve the
   crown — unless caught and hanged (fiercest under Venice's **Monopoly**).
6. **The Lean Years** — Drought dries the pans, storms wreck the catch, war
   closes the roads, mines collapse. Scarcity ripples outward, and from it,
   history is made.

---

## The world

Ten towns, each a thumbnail of real salt history, laid across a map that runs
from the Baltic down to the Sahara:

| Town | Region | Works | Role |
|------|--------|-------|------|
| Lübeck | Hansa | — | Salts the Baltic herring; depends on imports |
| Lüneburg | Hansa | Brine | The brine springs that fed the north |
| Brouage | Couronne | Sun | Bountiful Atlantic salt marshes |
| Anjou | Couronne | — | Groans under the gabelle; home of the Pitauds |
| Salzburg | Reich | Rock | Alpine mine on the Salzstraße |
| Hallstatt | Reich | Rock | Ancient salt mine |
| Venezia | Serenissima | Sun | Solar pans + a jealous monopoly |
| Trapani | Sicilia | Sun | Great Mediterranean exporter |
| Taghaza | Sahel | Rock | Desert slabs of mined salt |
| Timbuktu | Mali | — | The gold market at the end of the caravan road |

On the map: `~` sea, `.` plains, `^` mountains, `:` desert. Town letters glow
**green** (cheap) → **yellow** → **red** → **magenta** (worth its weight in
gold). `+` are merchant caravans, `!` are smugglers.

---

## How it works

`src/` is small and split by concern:

- `rng.rs` — a seedable SplitMix64 PRNG (so seeds are reproducible).
- `model.rs` — the data types and all the economic tuning constants.
- `world.rs` — map generation and the ten historically-themed towns.
- `sim.rs` — the heart: one `tick` applies the six rules in order.
- `render.rs` — ANSI terminal rendering.
- `main.rs` — argument parsing, the rules screen, the loop, the epilogue.

`tests/sim_tests.rs` checks the invariants (prices and stocks stay in bounds,
same seed = same history) and that each headline rule actually fires.

```
cargo test
```
