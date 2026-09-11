# LoRaPress

> **Universal high-efficiency multi-modal payload compressor & container for LoRa and Meshtastic networks ("FFmpeg for LoRa").**

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![no_std](https://img.shields.io/badge/no__std-compatible-success.svg)](#features)

---

## 📡 The Problem with LoRa & Meshtastic

In LoRa mesh networks (especially popular presets like **Medium-Fast** and **Long-Fast**):
* **MTU is strictly limited:** ~**237 bytes** per unfragmented frame.
* **Airtime is precious:** Multi-packet fragmentation causes severe packet collisions, duty-cycle violations, and network congestion.
* **Standard compression fails on short data:** General-purpose compressors like Gzip or Zstandard add 10–25 bytes of headers and frequency tables, often making payloads under 100 bytes **larger** than raw data.

**`lorapress`** is built specifically for LoRa. It features a 1-byte envelope header, static pre-shared radio dictionaries, bit-packed micro-schemas for JSON, and 4x4 macro-block clustering for images.

---

## 🚀 Performance Benchmarks (Meshtastic Medium-Fast)

Tested on real-world radio scenarios (SF9, BW 250 kHz, CR 4/5):

| Scenario | Original Size | Gzip (Best) | Zstd (Lvl 19) | **LoRaPress** | Selected Codec | Airtime Saved |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| **RU Short Radio Chat** *(«База, как слышно? Еду на точку 5.»)* | 68 B | 91 B *(+33%)* | 70 B *(+3%)* | **30 B** | `TextMicro` | **-92.2 ms (-41.6%)** |
| **RU Long Message (5-6 sentences)** | 435 B | 265 B *(2 packets)* | 266 B *(2 packets)* | **225 B** *(1 packet!)* | `TextMicro` | **-471.0 ms (-52.3%)** |
| **EN Long Chat (5 sentences)** | 235 B | 180 B | 170 B | **163 B** | `TextDeflate` | **-163.8 ms (-31.2%)** |
| **IoT GPS & Telemetry JSON** | 61 B | 84 B *(+37%)* | 64 B *(+5%)* | **22 B** | `JsonCompact` | **-81.9 ms (-40.7%)** |
| **Arbitrary Multi-Node JSON** | 235 B | 185 B | 171 B | **168 B** | `JsonDeflate` | **-153.6 ms (-30.0%)** |
| **32x32 Map Icon / Schematic** | 128 B | 33 B | 17 B | **28 B** | `ImageCluster` | **-225.3 ms (-64.0%)** |
| **Miren 5s 3D Avatar Circle (100 fr)** | 1657 B | 65 B | 47 B | **58 B** *(1 packet!)* | `MirenStream` | **-3635.2 ms (-96.5%)** |

---

## 🛠 Features

* **Multi-Modal Algorithm Race:** Automatically benchmarks and selects the best compression strategy per payload.
* **`no_std` Ready Core:** `lorapress-core` is lightweight and designed to run directly on microcontrollers (ESP32, nRF52, STM32) and WASM.
* **Prefix-Free `TextMicro` Codec:** Cyrillic and Latin frequency tables with built-in radio slang codebook.
* **`JsonCompact` Micro-Schema:** Fixed-point coordinate packing (1.1 cm GPS precision in 4 bytes) + sub-byte key mapping.
* **Macro-Block Image Clustering:** 4x4 cluster compression for transmitting maps, sketches, and icons in a single 30–80 byte packet.
* **Miren 3D Avatar & Video-Circle Channel Separation:** Dedicated codec for `miren` video messages (ARKit 52 blendshapes + Audio), squeezing a 5-second animated avatar into a single 58-byte packet!
* **Physics-Accurate Airtime Engine:** Integrated Time-on-Air calculation for Meshtastic presets.

---

## 📦 Workspace Crates

* **`lorapress-core`**: Core `no_std` library with all codecs, bitstreams, and envelope packing.
* **`lorapress-cli`**: Command-line utility for compression, decompression, image conversion, and airtime inspection.
* **`lorapress-bench`**: Comparative benchmarking tool vs Gzip and Zstandard.

---

## 💻 CLI Usage

```bash
# 1. Compress text to HEX for LoRa transmission
lorapress compress -t "База, как слышно? Еду на точку 5. Прием." --hex

# 2. Decompress from HEX
lorapress decompress -x 22055047814b7e84a29a

# 3. Convert an image (PNG/JPG) to a 1-bit LoRa cluster packet
lorapress image -i map_icon.png -s 32 --hex

# 4. Inspect payload metrics and Airtime in Meshtastic Medium-Fast
lorapress inspect -t "{"lat":55.7512,"lon":37.6184,"alt":154,"batt":89,"temp":21.4}"
```

---

## 🧪 Testing & Benchmarks

```bash
# Run all unit tests
cargo test

# Run the benchmark suite
cargo run --bin lorapress-bench
```

---

## 📄 License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
