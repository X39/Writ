# Writ Benchmark Results -- 2026-03-20

## Methodology

Platform: `x86_64` | Runs: 3 | Warmup: 2

All benchmarks run inside a Docker container for reproducibility.

Writ timings include separate compile and run phases; other interpreted languages run source directly; Rust is pre-compiled native code.

Ratio to Lua is the primary comparison — Lua is the closest competitor in the game scripting space. Ratio to Rust is shown for absolute reference.

## Startup

### stub

**What this measures:** Process startup overhead (hello world)

**Why it matters for Writ:** Measures CLI + compiler load + VM initialization cost. Game engines call into scripts frequently; low startup overhead matters.

| Language | Benchmark | Median (ms) | Compile (ms) | Memory (MB) | Ratio to Rust | Ratio to Lua |
|----------|-----------|-------------|--------------|-------------|---------------|--------------|
| Writ | stub | 1.7 | 1.0 | 0.0 | x3.4x | x2.8x |
| Rust | stub | 0.5 | - | 0.0 | x1.0x | x0.8x |
| Lua | stub | 0.6 | - | 0.0 | x1.2x | - |
| Squirrel | stub | 1.0 | - | 0.2 | x2.1x | x1.7x |
| Python | stub | 7.8 | - | 2.6 | x15.7x | x12.8x |
| Node.js | stub | 16.9 | - | 8.2 | x34.1x | x27.9x |

*Memory values of 0.0 MB indicate the process exited before RSS polling could sample.*

