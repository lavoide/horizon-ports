//! Spelunkery's changes to vanilla loot tables (ModLootOverrides, with the live server's config: rough gems and raw
//! nuggets on, sculk shearing on, no crying obsidian from bartering), made in place on the table the player's own game
//! loaded. The jar replaced each whole table; here only the part Spelunkery changes is touched, found by its shape (an
//! ore's silk-touch alternative and the drop beside it, the sculk entry, the crying obsidian entry), never by position.
//! A table without that shape is left as it is and `apply` says why, so `loot_table_load` can log it once.
//!
//! Only Spelunkery's own items, counts and conditions are written here (from the jar's override tables).
use super::{json, Value};

const SP: &str = "spelunkery:";

/// A vanilla ore whose drop becomes Spelunkery's: (block, vanilla drop, Spelunkery's drop, its count, shard pool).
/// The drop keeps its fortune (`ore_drops`) and explosion decay; a count replaces vanilla's.
type Swap = (&'static str, &'static str, &'static str, Option<(f64, f64)>, Option<(&'static str, f64, f64)>);
const SWAPS: &[Swap] = &[
    ("diamond_ore", "diamond", "rough_diamond", None, None),
    ("deepslate_diamond_ore", "diamond", "rough_diamond", None, Some(("rough_diamond_shard", 0.0, 2.0))),
    ("emerald_ore", "emerald", "rough_emerald", None, None),
    ("deepslate_emerald_ore", "emerald", "rough_emerald", None, Some(("rough_emerald_shard", 0.0, 2.0))),
    ("lapis_ore", "lapis_lazuli", "rough_lazurite", Some((1.0, 3.0)), None),
    ("deepslate_lapis_ore", "lapis_lazuli", "rough_lazurite", Some((1.0, 2.0)), Some(("rough_lazurite_shard", 2.0, 7.0))),
    ("redstone_ore", "redstone", "rough_cinnabar", Some((1.0, 2.0)), None),
    ("deepslate_redstone_ore", "redstone", "rough_cinnabar", Some((1.0, 2.0)), Some(("rough_cinnabar_shard", 3.0, 6.0))),
    ("nether_gold_ore", "gold_nugget", "raw_gold_nugget", Some((2.0, 6.0)), None),
];

/// A deepslate metal ore that keeps its vanilla drop and gains a raw nugget pool (not with silk touch):
/// (block, vanilla drop, nugget, count). Deepslate iron ore rolls a raw magnetite nugget (1 in 8, 1-2) first.
const NUGGETS: &[(&str, &str, &str, (f64, f64))] = &[
    ("deepslate_coal_ore", "coal", "coal_lump", (3.0, 6.0)),
    ("deepslate_copper_ore", "raw_copper", "raw_copper_nugget", (3.0, 6.0)),
    ("deepslate_gold_ore", "raw_gold", "raw_gold_nugget", (1.0, 4.0)),
    ("deepslate_iron_ore", "raw_iron", "raw_iron_nugget", (1.0, 4.0)),
];

/// What `apply` did to a table.
#[derive(Debug, PartialEq)]
pub(crate) enum Patched {
    /// Not a table Spelunkery changes.
    NotOurs,
    Changed,
    /// Already holds Spelunkery's change (a data pack's, or a second call).
    Already,
    /// The table hasn't the shape Spelunkery's change needs; left as it is.
    Unexpected(&'static str),
}

pub(crate) fn apply(id: &str, table: &mut Value) -> Patched {
    let Some(name) = id.strip_prefix("minecraft:") else { return Patched::NotOurs };
    let block = name.strip_prefix("blocks/");
    if let Some(s) = SWAPS.iter().find(|s| Some(s.0) == block) {
        return guarded(table, &format!("{SP}{}", s.2), |t| swap(t, s));
    }
    if let Some(n) = NUGGETS.iter().find(|n| Some(n.0) == block) {
        return guarded(table, &format!("{SP}{}", n.2), |t| nuggets(t, n));
    }
    match name {
        "blocks/sculk" => guarded(table, "minecraft:shears", sculk),
        "blocks/sculk_vein" => guarded(table, "minecraft:shears", sculk_vein),
        "gameplay/piglin_bartering" => bartering(table),
        _ => Patched::NotOurs,
    }
}

/// Runs `f` unless the table already names `marker` (Spelunkery's item, or the shears it adds).
fn guarded(table: &mut Value, marker: &str, f: impl FnOnce(&mut Value) -> Patched) -> Patched {
    if names(table, marker) {
        return Patched::Already;
    }
    f(table)
}

fn names(v: &Value, id: &str) -> bool {
    match v {
        Value::String(s) => same_id(s, id),
        Value::Array(a) => a.iter().any(|x| names(x, id)),
        Value::Object(o) => o.values().any(|x| names(x, id)),
        _ => false,
    }
}

fn same_id(a: &str, b: &str) -> bool {
    let full = |s: &str| if s.contains(':') { s.to_string() } else { format!("minecraft:{s}") };
    full(a) == full(b)
}

fn is_type(v: &Value, ty: &str) -> bool {
    v.get("type").and_then(Value::as_str).is_some_and(|t| same_id(t, ty))
}

fn is_item(v: &Value, id: &str) -> bool {
    is_type(v, "item") && v.get("name").and_then(Value::as_str).is_some_and(|n| same_id(n, id))
}

/// A silk touch condition in any form the game reads: 26.3's named `minecraft:tool/can_silk_touch`, or a
/// `match_tool` naming the silk touch enchantment.
fn is_silk(c: &Value) -> bool {
    c.to_string().contains("silk_touch")
}

fn pools(table: &mut Value) -> Option<&mut Vec<Value>> {
    table.get_mut("pools").and_then(Value::as_array_mut)
}

/// The ore's `alternatives`: a silk touch child and an unconditioned `drop` child. Returns the place of the drop
/// child (pool, entry, child) and the silk touch condition, to reuse it as the game wrote it.
fn ore_drop(table: &mut Value, drop: &str) -> Option<((usize, usize, usize), Value)> {
    for (p, pool) in pools(table)?.iter().enumerate() {
        let Some(entries) = pool.get("entries").and_then(Value::as_array) else { continue };
        for (e, entry) in entries.iter().enumerate() {
            if !is_type(entry, "alternatives") {
                continue;
            }
            let Some(children) = entry.get("children").and_then(Value::as_array) else { continue };
            let silk = children.iter().filter_map(|c| c.get("condition")).find(|c| is_silk(c));
            let found = children.iter().position(|c| is_item(c, drop) && c.get("condition").is_none());
            if let (Some(silk), Some(c)) = (silk, found) {
                return Some(((p, e, c), silk.clone()));
            }
        }
    }
    None
}

fn uniform(min: f64, max: f64) -> Value {
    json!({"type": "minecraft:set_count", "add": false, "count": {"type": "minecraft:uniform", "min": min, "max": max}})
}

/// Spelunkery's drop: its count (if any), then fortune (`ore_drops`) and explosion decay.
fn ore_item(id: &str, count: Option<(f64, f64)>, extra: Option<Value>) -> Value {
    let mut modifier: Vec<Value> = count.map(|(a, b)| uniform(a, b)).into_iter().collect();
    modifier.push(json!({"type": "minecraft:apply_bonus", "enchantment": "minecraft:fortune", "formula": "minecraft:ore_drops"}));
    modifier.push(json!({"type": "minecraft:explosion_decay"}));
    let mut v = json!({"type": "minecraft:item", "name": format!("{SP}{id}"), "modifier": modifier});
    if let (Some(extra), Some(o)) = (extra, v.as_object_mut()) {
        o.insert("condition".into(), extra);
    }
    v
}

/// One roll: nothing with silk touch, else `child`.
fn no_silk_pool(silk: Value, child: Value) -> Value {
    json!({"rolls": 1.0, "bonus_rolls": 0.0, "entries": [{"type": "minecraft:alternatives", "children": [
        {"type": "minecraft:empty", "condition": silk}, child]}]})
}

fn swap(table: &mut Value, s: &Swap) -> Patched {
    let (_, from, to, count, shard) = *s;
    let Some(((p, e, c), silk)) = ore_drop(table, &format!("minecraft:{from}")) else {
        return Patched::Unexpected("no silk touch alternative beside the vanilla drop");
    };
    let Some(pools) = pools(table) else { return Patched::Unexpected("no pools") };
    let Some(child) = pools[p]["entries"][e]["children"].get_mut(c) else {
        return Patched::Unexpected("drop moved");
    };
    *child = ore_item(to, count, None);
    if let Some((id, a, b)) = shard {
        pools.push(no_silk_pool(silk, ore_item(id, Some((a, b)), None)));
    }
    Patched::Changed
}

fn nuggets(table: &mut Value, n: &(&str, &str, &str, (f64, f64))) -> Patched {
    let (block, from, nugget, count) = *n;
    let Some((_, silk)) = ore_drop(table, &format!("minecraft:{from}")) else {
        return Patched::Unexpected("no silk touch alternative beside the vanilla drop");
    };
    let mut child = ore_item(nugget, Some(count), None);
    if block == "deepslate_iron_ore" {
        let magnetite = ore_item(
            "raw_magnetite_nugget",
            Some((1.0, 2.0)),
            Some(json!({"type": "minecraft:random_chance", "chance": 0.125})),
        );
        child = json!({"type": "minecraft:alternatives", "children": [magnetite, child]});
    }
    let Some(pools) = pools(table) else { return Patched::Unexpected("no pools") };
    pools.insert(0, no_silk_pool(silk, child));
    Patched::Changed
}

/// Shears as the tool. `items` as one id, as vanilla's own tables write it: dd67b76's block drops read only that form
/// of the holder set (a list matches nothing).
fn shears() -> Value {
    json!({"type": "minecraft:match_tool", "predicate": {"items": "minecraft:shears"}})
}

/// The silk touch condition gating `entry` of `pool`, on the entry or on the pool (26.3 puts sculk's on the pool),
/// taken off where it was. None if neither holds one, or the pool's condition is something else.
fn take_silk(pool: &mut Value, e: usize) -> Option<Value> {
    let on_pool = pool.get("condition").cloned();
    let entry = &mut pool["entries"][e];
    match (on_pool, entry.get("condition").cloned()) {
        (None, Some(c)) if is_silk(&c) => entry.as_object_mut()?.remove("condition"),
        (Some(c), None) if is_silk(&c) => pool.as_object_mut()?.remove("condition"),
        _ => None,
    }
}

fn find_entry(table: &mut Value, id: &str) -> Option<(usize, usize)> {
    pools(table)?.iter().enumerate().find_map(|(p, pool)| {
        let e = pool.get("entries")?.as_array()?.iter().position(|x| is_item(x, id))?;
        Some((p, e))
    })
}

/// Sculk: silk touch gives the block, shears 0-2 sculk veins (SculkShearing).
fn sculk(table: &mut Value) -> Patched {
    let Some((p, e)) = find_entry(table, "minecraft:sculk") else { return Patched::Unexpected("no sculk entry") };
    let Some(pools) = pools(table) else { return Patched::Unexpected("no pools") };
    let pool = &mut pools[p];
    let Some(silk) = take_silk(pool, e) else { return Patched::Unexpected("sculk entry not gated by silk touch") };
    let mut block = pool["entries"][e].take();
    if let Some(o) = block.as_object_mut() {
        o.insert("condition".into(), silk);
    }
    let vein = json!({"type": "minecraft:item", "name": "minecraft:sculk_vein", "condition": shears(),
        "modifier": [{"type": "minecraft:set_count", "add": false, "count": {"type": "minecraft:uniform", "min": 0.0, "max": 2.0}}]});
    pool["entries"][e] = json!({"type": "minecraft:alternatives", "children": [block, vein]});
    Patched::Changed
}

/// Sculk vein: shears take it as silk touch does (its count by faces stays the game's).
fn sculk_vein(table: &mut Value) -> Patched {
    let Some((p, e)) = find_entry(table, "minecraft:sculk_vein") else {
        return Patched::Unexpected("no sculk vein entry");
    };
    let Some(pools) = pools(table) else { return Patched::Unexpected("no pools") };
    let pool = &mut pools[p];
    let Some(silk) = take_silk(pool, e) else {
        return Patched::Unexpected("sculk vein entry not gated by silk touch");
    };
    if let Some(o) = pool["entries"][e].as_object_mut() {
        o.insert("condition".into(), json!({"type": "minecraft:any_of", "terms": [silk, shears()]}));
    }
    Patched::Changed
}

/// Piglins no longer barter crying obsidian (the jar's override table leaves it out).
fn bartering(table: &mut Value) -> Patched {
    let Some(pools) = pools(table) else { return Patched::Unexpected("no pools") };
    let mut removed = 0;
    for pool in pools.iter_mut() {
        if let Some(entries) = pool.get_mut("entries").and_then(Value::as_array_mut) {
            let before = entries.len();
            entries.retain(|x| !is_item(x, "minecraft:crying_obsidian"));
            removed += before - entries.len();
        }
    }
    if removed == 0 { Patched::Already } else { Patched::Changed }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bare shapes only (no game values): an ore's alternatives, sculk's pool-gated entry, a barter pool.
    fn ore(block: &str, drop: &str) -> Value {
        json!({"type": "minecraft:block", "pools": [{"rolls": 1, "entries": [{"type": "minecraft:alternatives", "children": [
            {"type": "minecraft:item", "condition": "minecraft:tool/can_silk_touch", "name": format!("minecraft:{block}")},
            {"type": "minecraft:item", "name": format!("minecraft:{drop}"), "modifier": []}]}]}]})
    }

    #[test]
    fn gem_swap_and_shard() {
        let mut t = ore("deepslate_diamond_ore", "diamond");
        assert_eq!(apply("minecraft:blocks/deepslate_diamond_ore", &mut t), Patched::Changed);
        let p = t["pools"].as_array().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0]["entries"][0]["children"][1]["name"], "spelunkery:rough_diamond");
        assert_eq!(p[0]["entries"][0]["children"][0]["name"], "minecraft:deepslate_diamond_ore");
        assert_eq!(p[1]["entries"][0]["children"][0]["condition"], "minecraft:tool/can_silk_touch");
        assert_eq!(p[1]["entries"][0]["children"][1]["name"], "spelunkery:rough_diamond_shard");
        assert_eq!(apply("minecraft:blocks/deepslate_diamond_ore", &mut t), Patched::Already);
    }

    #[test]
    fn metal_nuggets_first() {
        let mut t = ore("deepslate_iron_ore", "raw_iron");
        assert_eq!(apply("minecraft:blocks/deepslate_iron_ore", &mut t), Patched::Changed);
        let p = t["pools"].as_array().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[1]["entries"][0]["children"][1]["name"], "minecraft:raw_iron");
        let alt = &p[0]["entries"][0]["children"][1];
        assert_eq!(alt["children"][0]["name"], "spelunkery:raw_magnetite_nugget");
        assert_eq!(alt["children"][1]["name"], "spelunkery:raw_iron_nugget");
    }

    #[test]
    fn unexpected_left_alone() {
        let mut t = ore("diamond_ore", "coal");
        let before = t.clone();
        assert!(matches!(apply("minecraft:blocks/diamond_ore", &mut t), Patched::Unexpected(_)));
        assert_eq!(t, before);
        assert_eq!(apply("minecraft:blocks/stone", &mut t), Patched::NotOurs);
    }

    #[test]
    fn sculk_shapes() {
        let mut t = json!({"pools": [{"rolls": 1, "condition": "minecraft:tool/can_silk_touch",
            "entries": [{"type": "minecraft:item", "name": "minecraft:sculk"}]}]});
        assert_eq!(apply("minecraft:blocks/sculk", &mut t), Patched::Changed);
        let pool = &t["pools"][0];
        assert!(pool.get("condition").is_none());
        assert_eq!(pool["entries"][0]["children"][0]["condition"], "minecraft:tool/can_silk_touch");
        assert_eq!(pool["entries"][0]["children"][1]["name"], "minecraft:sculk_vein");
        let mut v = json!({"pools": [{"rolls": 1, "entries": [{"type": "minecraft:item",
            "condition": "minecraft:tool/can_silk_touch", "name": "minecraft:sculk_vein", "modifier": []}]}]});
        assert_eq!(apply("minecraft:blocks/sculk_vein", &mut v), Patched::Changed);
        assert_eq!(v["pools"][0]["entries"][0]["condition"]["type"], "minecraft:any_of");
    }

    #[test]
    fn barter_without_crying_obsidian() {
        let mut t = json!({"pools": [{"rolls": 1, "entries": [
            {"type": "minecraft:item", "name": "minecraft:obsidian"},
            {"type": "minecraft:item", "name": "minecraft:crying_obsidian"}]}]});
        assert_eq!(apply("minecraft:gameplay/piglin_bartering", &mut t), Patched::Changed);
        assert_eq!(t["pools"][0]["entries"].as_array().unwrap().len(), 1);
        assert_eq!(apply("minecraft:gameplay/piglin_bartering", &mut t), Patched::Already);
    }
}
