# Licences

Every file in this repository that is not code written for it: its source, its author, its terms. The code is under the repository's licence.

## Assets

| Path | Source | Terms |
| --- | --- | --- |
| `web/art/` | drawn by `tools/art/art.py` from TikZ in this repository | this repository's licence |

## Vendored code

| Path | Project and version | Author | Licence | Checksum |
| --- | --- | --- | --- | --- |

## Code taken from other repositories

| From | What | Licence |
| --- | --- | --- |
| Mage Tower Battle (`sgilson7/mage-tower-battle`), by the same author | `fx.rs`, `rng.rs`, the clock in `ticks.rs` (all three from Builder's realtime kit), `geom.rs`; the pattern of `mesh.rs`, `render.rs`, `web/gl.js` and `data/shaders/shapes.*` | MIT, same author |
| Builder (`sgilson7/builder-setting`) | the template this project was copied from | Builder's licence |

## Methods taken from papers and notes

| Where | Method | Source |
| --- | --- | --- |
| `line.rs` | XPBD constraints | Macklin, Müller and Chentanez, 2016, MIG '16 |
| `line.rs` | long-range attachments | Kim, Chentanez and Müller, 2012, SCA '12 |
| `line.rs` | drag along a line far less than across it | Morison, Johnson and Schaaf, 1950 |
| `hull.rs` | buoyancy from probes on the waterplane | Kerner, 2015, Game Developer |
| `fish.rs` | a mouth zone about one gape | Higham, Day and Wainwright, 2006, J. Exp. Biol. |
| `water.frag` | a hash without sine | Dave Hoskins, "Hash without Sine", MIT |

A file with no licence, a likeness, a recording or any third-party picture is a worklist row in `SECOND-ORDER.md` for the person to decide, and stays out of git until they do.
