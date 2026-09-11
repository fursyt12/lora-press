# LoRaPress AI & Developer Agent Guidelines

This document contains architecture guidelines, development conventions, and reference information for AI agents and human contributors working on `lorapress`.

---

## 1. Project Overview & Mission

`lorapress` is a high-efficiency, multi-modal payload compressor and container optimized specifically for ultra-low-bandwidth LoRa and Meshtastic mesh networks (with primary focus on the `Medium-Fast` preset).

### Core Philosophy:
* **The 1-Packet Rule:** LoRa packets with size $\le 237$ bytes fit into a single unfragmented Meshtastic MTU packet. Fragmentation in mesh networks causes exponential packet loss and air congestion. Everything in `lorapress` is optimized to fit inside 1 packet (or strictly bounded 2-3 packets for high-res imagery / audio).
* **Airtime is the Bottleneck:** Compute power on modern nodes (ESP32-S3, nRF52840, smartphones) is abundant; Radio Time-on-Air (Airtime) is the critical constraint.
* **No Header Overhead Penalty:** Standard compression formats (Gzip, Zstandard) add 10–20+ bytes of headers and frequency tables, degrading compression for short packets (<100 bytes). `lorapress` uses 1-byte envelope headers and static pre-shared codebooks.

---

## 2. Workspace Structure

```text
lorapress/
├── Cargo.toml                  # Workspace definition
├── README.md                   # English documentation
├── README_RU.md                # Russian documentation
├── AGENTS.md                   # Agent & contributor guidelines
├── CLAUDE.md                   # Claude reference pointer to AGENTS.md
└── crates/
    ├── lorapress-core/         # `no_std` compatible compression & envelope engine
    │   ├── src/
    │   │   ├── lib.rs          # Core entry point and Error types
    │   │   ├── airtime.rs      # Physics-accurate Time-on-Air calculator (SF, BW, CR)
    │   │   ├── bitstream.rs    # Sub-byte BitWriter and BitReader
    │   │   ├── envelope.rs     # 1-byte packet header format and types
    │   │   ├── text.rs         # Prefix-free LPC variable-length text & slang dictionary
    │   │   ├── structured.rs   # Bit-packed micro-schema for IoT & GPS JSON
    │   │   ├── image.rs        # 4x4 Macro-block Cluster monochrome image codec
    │   │   └── detector.rs     # UniversalEngine (Algorithm race & multi-format dispatcher)
    ├── lorapress-cli/          # Universal CLI utility ("ffmpeg for LoRa")
    │   └── src/main.rs         # CLI subcommands: compress, decompress, image, inspect
    └── lorapress-bench/        # Benchmark suite comparing against Gzip and Zstd
        └── src/main.rs         # Test cases, Airtime calculations, and metrics reporting
```

---

## 3. Supported Data Formats & Codecs

| Payload Type ID | Variant | Best For | Typical Compression Ratio |
|---|---|---|---|
| `0x0` | `Raw` | Uncompressible binary fallback | 0% |
| `0x1` | `TextMicro` | Short RU/EN chat messages (< 80 bytes) | -55% .. -65% |
| `0x2` | `TextDeflate` | Long text messages (5+ sentences, > 150 bytes) | -30% .. -45% |
| `0x3` | `JsonCompact` | IoT Telemetry, GPS coordinates, battery, temperature | -60% .. -85% |
| `0x4` | `JsonDeflate` | Arbitrary multi-node complex JSONs | -25% .. -40% |
| `0x5` | `ImageCluster` | 32x32 to 64x64 1-bit monochrome maps / icons | -60% .. -80% |
| `0x6` | `VoiceCodec2` | Ultra-low-bitrate speech (450 / 700 / 1200 bps) | ~100-200 bytes / 2 sec |
| `0x7` | `MirenStream` | Miren 3D avatar & video-circle stream (ARKit 52) | -90% .. -96% |

---

## 4. Development & Coding Standards

1. **`no_std` First in `lorapress-core`:**
   * Never introduce dependencies in `lorapress-core` that break `no_std` builds unless guarded by the `std` feature.
   * Target platforms include microcontroller firmware (ESP32, nRF52840, STM32) and WebAssembly (`wasm32-unknown-unknown`).
2. **Determinism:**
   * Encoding and decoding must produce identical round-trip data with zero side effects.
3. **Testing:**
   * Every new codec or envelope type must include a unit test with round-trip verification (`compress` $\to$ `decompress` $\to$ `assert_eq`).
   * Run tests with `cargo test`.
4. **Benchmarking:**
   * Benchmark changes using `cargo run --bin lorapress-bench` to verify Airtime impact on Meshtastic Medium-Fast.

---

## 5. Common Commands

```bash
# Run all unit and integration tests
cargo test

# Run the comparative benchmark
cargo run --bin lorapress-bench

# Inspect a payload via CLI
cargo run --bin lorapress-cli -- inspect -t "База, как слышно? Прием."

# Compress an image into a 1-bit cluster payload
cargo run --bin lorapress-cli -- image -i input.png -s 32 --hex
```
