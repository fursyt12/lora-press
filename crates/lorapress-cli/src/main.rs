use clap::{Parser, Subcommand};
use lorapress_core::airtime::LoraModemConfig;
use lorapress_core::detector::UniversalEngine;
use lorapress_core::envelope::{EnvelopeHeader, PayloadType};
use lorapress_core::image::ClusterImageCodec;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lorapress")]
#[command(about = "Universal multi-format compression & airtime tool for LoRa/Meshtastic (like ffmpeg for LoRa)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compress input file, text, image or stdin
    Compress {
        #[arg(short = 'i', long)]
        input: Option<PathBuf>,
        #[arg(short = 't', long)]
        text: Option<String>,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        #[arg(long)]
        hex: bool,
    },
    /// Decompress a lorapress payload back to original format
    Decompress {
        #[arg(short = 'i', long)]
        input: Option<PathBuf>,
        #[arg(short = 'x', long)]
        hex: Option<String>,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },
    /// Compress image (PNG, JPG, BMP) to LoRa 1-bit clustered packet
    Image {
        #[arg(short = 'i', long)]
        input: PathBuf,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        #[arg(short = 's', long, default_value_t = 32)]
        size: u32,
        #[arg(short = 't', long, default_value_t = 0)]
        threshold: u8,
        #[arg(long)]
        hex: bool,
    },
    /// Inspect payload airtime, fragmentation and stats on Meshtastic Medium-Fast
    Inspect {
        #[arg(short = 'i', long)]
        input: Option<PathBuf>,
        #[arg(short = 't', long)]
        text: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mf = LoraModemConfig::medium_fast();

    match cli.command {
        Commands::Compress { input, text, output, hex } => {
            let data = if let Some(t) = text {
                t.into_bytes()
            } else if let Some(path) = input {
                fs::read(path)?
            } else {
                let mut buf = Vec::new();
                io::stdin().read_to_end(&mut buf)?;
                buf
            };

            let (header, compressed) = UniversalEngine::compress(&data);
            let mut result = Vec::with_capacity(compressed.len() + 1);
            result.push(header.serialize());
            result.extend_from_slice(&compressed);

            if let Some(out_path) = output {
                fs::write(out_path, &result)?;
            } else if hex {
                println!("{}", hex::encode(&result));
            } else {
                io::stdout().write_all(&result)?;
            }
        }
        Commands::Decompress { input, hex, output } => {
            let data = if let Some(h) = hex {
                hex::decode(h)?
            } else if let Some(path) = input {
                fs::read(path)?
            } else {
                let mut buf = Vec::new();
                io::stdin().read_to_end(&mut buf)?;
                buf
            };

            let decompressed = UniversalEngine::decompress(&data)?;

            if let Some(out_path) = output {
                fs::write(out_path, &decompressed)?;
            } else {
                io::stdout().write_all(&decompressed)?;
            }
        }
        Commands::Image { input, output, size, threshold, hex } => {
            let img = image::open(&input)?.to_luma8();
            let resized = image::imageops::resize(&img, size, size, image::imageops::FilterType::Triangle);
            
            // Dither / threshold to 1-bit monochrome
            let mut pixels = Vec::with_capacity((size * size) as usize);
            for p in resized.pixels() {
                pixels.push(p.0[0] > 128);
            }

            let comp_img = ClusterImageCodec::encode_4x4_clustered(size, size, &pixels, threshold);
            let header = EnvelopeHeader::new(PayloadType::ImageCluster, 0);
            
            let mut result = Vec::with_capacity(comp_img.len() + 1);
            result.push(header.serialize());
            result.extend_from_slice(&comp_img);

            let orig_raw_size = (size * size) as usize;
            let final_size = result.len();
            let airtime = mf.airtime_ms(final_size);

            eprintln!("Image {}x{} -> Compressed: {} bytes (Airtime: {:.1} ms, Packets: {})",
                     size, size, final_size, airtime, (final_size + 236) / 237);

            if let Some(out_path) = output {
                fs::write(out_path, &result)?;
            } else if hex {
                println!("{}", hex::encode(&result));
            } else {
                io::stdout().write_all(&result)?;
            }
        }
        Commands::Inspect { input, text } => {
            let data = if let Some(t) = text {
                t.into_bytes()
            } else if let Some(path) = input {
                fs::read(path)?
            } else {
                "База, как слышно? Еду на точку 5. Прием.".as_bytes().to_vec()
            };

            let (header, compressed) = UniversalEngine::compress(&data);
            let orig_len = data.len();
            let comp_len = compressed.len() + 1;
            let ratio = (1.0 - (comp_len as f64 / orig_len as f64)) * 100.0;

            let airtime_orig = mf.airtime_ms(orig_len);
            let airtime_comp = mf.airtime_ms(comp_len);
            let airtime_saved = airtime_orig - airtime_comp;
            let packets = (comp_len + 236) / 237;

            println!("=== LoRaPress Inspection (Meshtastic Medium-Fast) ===");
            println!("Original size:      {} bytes -> Airtime: {:.1} ms", orig_len, airtime_orig);
            println!("Compressed size:    {} bytes -> Airtime: {:.1} ms", comp_len, airtime_comp);
            println!("Algorithm chosen:   {:?}", header.payload_type);
            println!("Compression ratio:  {:.1}% saved", ratio);
            println!("Airtime reduction:  {:.1} ms (-{:.1}%)", airtime_saved, (airtime_saved / airtime_orig) * 100.0);
            println!("LoRa Packets (MTU 237): {} packet(s)", packets);
        }
    }

    Ok(())
}
