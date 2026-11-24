# Spelunkery for Horizon

Rough gems and raw nuggets from ores, cave mushrooms, geodes, rock salt, glowsticks, rope ladders and mining tools.

A port of [Spelunkery](https://github.com/AstralOrdana/Spelunkery) 1.20.1-0.3.16 (Fabric 1.20.1) by Ordana and Keybounce to [Horizon](https://github.com/horizon-rust/horizon) 26.3, under the original's licence (LGPL-3.0-only, in `LICENSE`, with the GNU GPL v3 text it builds on in `COPYING`; see `NOTICE`, which also credits the two remixed sounds).

This folder is the package source: one native mod on Horizon's mod API only (no game module, no mixins). Release [`spelunkery-v0.2.1`](https://github.com/lavoide/horizon-ports/releases/tag/spelunkery-v0.2.1) holds `spelunkery.hmod`, built from this folder with Horizon's exact-build SDK `dd67b76ff23e28c6d8e21859df600d31572f81a0` (Horizon Mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`). Earlier versions, if any, are in this folder's history.

Since 0.2.1 the package holds no Minecraft data: Spelunkery's changes to vanilla loot tables (rough gems and raw nuggets from vanilla ores, sculk to shears, no crying obsidian from bartering) are made in place on the player's own tables as they load (`src/loot.rs`), and the copies of vanilla's ore features that put the stone ore variants into vanilla's veins are gone, so those variants don't generate.
