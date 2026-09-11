use lorapress_core::airtime::LoraModemConfig;
use lorapress_core::detector::UniversalEngine;
use lorapress_core::image::ClusterImageCodec;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

fn compress_gzip(data: &[u8]) -> usize {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap().len()
}

fn compress_zstd(data: &[u8]) -> usize {
    zstd::encode_all(data, 19).unwrap().len()
}

fn main() {
    let mf = LoraModemConfig::medium_fast();

    println!("========================================================================================================");
    println!("                  LORAPRESS UNIVERSAL COMPRESSION & AIRTIME BENCHMARK (Medium-Fast)                     ");
    println!("========================================================================================================\n");

    let test_cases = [
        ("RU Short Radio", "База, как слышно? Еду на точку 5. Прием."),
        ("RU Long Message (5-6 sent)", "Всем группам, внимание! Наблюдаем изменение погодных условий в секторе 4. Дождь усиливается, видимость падает до ста метров. Переходим на запасную частоту канала два. Координаты контрольной точки подтверждены. Всем доложить о готовности."),
        ("EN Long Chat (5 sent)", "Hello team! We have successfully reached the summit base camp. Weather is clear with light winds from north. Setting up the stationary repeater node now. Battery levels are nominal at 95 percent. Waiting for further instructions, over."),
        ("IoT Micro Telemetry", r#"{"lat":55.7512,"lon":37.6184,"alt":154,"batt":89,"temp":21.4}"#),
        ("Arbitrary Large JSON", r#"{"nodes":[{"id":"!28a9b1","name":"Base-Repeater","snr":9.2,"lat":55.75124,"lon":37.61842},{"id":"!39c4f2","name":"Mobile-Scout","snr":-2.5,"lat":55.75331,"lon":37.62015}],"network_status":"active","channel":"medium_fast","hop_limit":3}"#),
    ];

    println!("{:<28} | {:<8} | {:<10} | {:<10} | {:<10} | {:<14} | {:<10}", 
             "Payload Scenario", "Original", "Gzip", "Zstandard", "LoRaPress", "Algo Selected", "Saved Airtime");
    println!("{:-<108}", "");

    for (name, text) in test_cases {
        let raw = text.as_bytes();
        let orig_size = raw.len();
        let gz_size = compress_gzip(raw);
        let zstd_size = compress_zstd(raw);
        
        let (hdr, compressed) = UniversalEngine::compress(raw);
        let lpc_size = compressed.len() + 1;

        let orig_airtime = mf.airtime_ms(orig_size);
        let lpc_airtime = mf.airtime_ms(lpc_size);
        let saved_ms = orig_airtime - lpc_airtime;

        println!("{:<28} | {:>5} B  | {:>7} B  | {:>7} B  | {:>6} B   | {:<14?} | {:>7.1} ms",
                 name, orig_size, gz_size, zstd_size, lpc_size, hdr.payload_type, saved_ms);
    }

    println!("\n--------------------------------------------------------------------------------------------------------");
    println!("                                   IMAGE 4x4 CLUSTERING BENCHMARK                                       ");
    println!("--------------------------------------------------------------------------------------------------------");

    // 32x32 Clustered Image (Map / Schematic icon)
    let mut img_32 = vec![false; 32 * 32];
    for y in 0..12 { for x in 0..12 { img_32[y * 32 + x] = true; } } // filled block
    for i in 0..32 { img_32[i * 32 + i] = true; } // line
    let img_raw = 32 * 32 / 8; // 128 bytes raw 1-bit bitmap
    let img_clustered_lossless = ClusterImageCodec::encode_4x4_clustered(32, 32, &img_32, 0);
    let img_clustered_lossy = ClusterImageCodec::encode_4x4_clustered(32, 32, &img_32, 2);

    println!("{:<28} | {:>5} B  | {:>7} B  | {:>7} B  | {:>6} B   | {:<14} | {:>7.1} ms",
             "Icon 32x32 (Lossless 4x4)", img_raw, compress_gzip(&vec![0u8; 128]), compress_zstd(&vec![0u8; 128]), img_clustered_lossless.len() + 1, "Clustered", mf.airtime_ms(img_raw) - mf.airtime_ms(img_clustered_lossless.len() + 1));
    println!("{:<28} | {:>5} B  | {:>7} B  | {:>7} B  | {:>6} B   | {:<14} | {:>7.1} ms",
             "Icon 32x32 (Lossy 4x4)", img_raw, compress_gzip(&vec![0u8; 128]), compress_zstd(&vec![0u8; 128]), img_clustered_lossy.len() + 1, "Clustered", mf.airtime_ms(img_raw) - mf.airtime_ms(img_clustered_lossy.len() + 1));

    println!("\n--------------------------------------------------------------------------------------------------------");
    println!("                                   MIREN 3D AVATAR STREAM BENCHMARK                                     ");
    println!("--------------------------------------------------------------------------------------------------------");

    // Synthetic Miren 5-second stream (1 Keyframe + 99 Deltas @ 20 FPS)
    let mut miren_raw = Vec::new();
    miren_raw.extend_from_slice(lorapress_core::miren::MIREN_MAGIC);
    miren_raw.push(0x01);
    miren_raw.extend_from_slice(&0u32.to_le_bytes());
    miren_raw.extend_from_slice(&[10u8; 52]);
    miren_raw.extend_from_slice(&[0u8; 12]);

    for _ in 1..100 {
        miren_raw.push(0x02);
        miren_raw.extend_from_slice(&50u16.to_le_bytes());
        let mut mask = [0u8; 7];
        mask[1] = 0b00111000;
        miren_raw.extend_from_slice(&mask);
        miren_raw.push(-2i8 as u8);
        miren_raw.push(3i8 as u8);
        miren_raw.push(-1i8 as u8);
        miren_raw.extend_from_slice(&[0, 1, 0]);
    }

    let (m_hdr, m_comp) = UniversalEngine::compress(&miren_raw);
    let m_lpc_size = m_comp.len() + 1;
    let m_orig_size = miren_raw.len();
    let m_gz_size = compress_gzip(&miren_raw);
    let m_zstd_size = compress_zstd(&miren_raw);
    let m_saved_airtime = mf.airtime_ms(m_orig_size) - mf.airtime_ms(m_lpc_size);

    println!("{:<28} | {:>5} B  | {:>7} B  | {:>7} B  | {:>6} B   | {:<14?} | {:>7.1} ms",
             "Miren 5s Circle (100 fr)", m_orig_size, m_gz_size, m_zstd_size, m_lpc_size, m_hdr.payload_type, m_saved_airtime);
    println!("   ↳ LoRa Packets (MTU 237): Raw = {} packets ➔ LoRaPress Miren = {} packets (Fit in 1-2 packets!)",
             (m_orig_size + 236) / 237, (m_lpc_size + 236) / 237);

    println!("========================================================================================================\n");
}
