//! LoRa Airtime calculator for Meshtastic presets (specifically Medium-Fast)

/// LoRa Modem Configuration
#[derive(Debug, Clone, Copy)]
pub struct LoraModemConfig {
    pub sf: u8,          // Spreading Factor (e.g. 9 for Medium-Fast)
    pub bw_khz: f64,     // Bandwidth in kHz (e.g. 250.0 for Medium-Fast)
    pub cr: u8,          // Coding Rate denominator (e.g. 5 for 4/5)
    pub preamble_len: u8,// Preamble symbols (Meshtastic default is 16)
    pub header_explicit: bool, // Meshtastic uses explicit header
}

impl LoraModemConfig {
    /// Meshtastic Medium-Fast preset (SF9, BW 250kHz, CR 4/5, Preamble 16)
    pub const fn medium_fast() -> Self {
        Self {
            sf: 9,
            bw_khz: 250.0,
            cr: 5,
            preamble_len: 16,
            header_explicit: true,
        }
    }

    /// Meshtastic Long-Fast preset (SF11, BW 250kHz, CR 4/5, Preamble 16)
    pub const fn long_fast() -> Self {
        Self {
            sf: 11,
            bw_khz: 250.0,
            cr: 5,
            preamble_len: 16,
            header_explicit: true,
        }
    }

    /// Meshtastic Short-Fast preset (SF7, BW 250kHz, CR 4/5, Preamble 16)
    pub const fn short_fast() -> Self {
        Self {
            sf: 7,
            bw_khz: 250.0,
            cr: 5,
            preamble_len: 16,
            header_explicit: true,
        }
    }

    /// Calculate Time-on-Air in milliseconds for a payload of `payload_bytes`
    pub fn airtime_ms(&self, payload_bytes: usize) -> f64 {
        let sf = self.sf as f64;
        let bw = self.bw_khz * 1000.0;
        let t_sym = (2.0_f64.powf(sf)) / bw * 1000.0; // Symbol time in ms

        // Preamble duration
        let t_preamble = (self.preamble_len as f64 + 4.25) * t_sym;

        // Low Data Rate Optimization (mandatory when Tsym >= 16ms or SF11/SF12 on 125kHz)
        let de = if sf >= 11.0 && self.bw_khz <= 125.0 { 1.0 } else { 0.0 };
        let h = if self.header_explicit { 0.0 } else { 1.0 }; // 0 for explicit header
        let cr_term = self.cr as f64 - 4.0; // 1 for 4/5, 2 for 4/6, etc.

        let pl = payload_bytes as f64;
        let num = 8.0 * pl - 4.0 * sf + 28.0 + 16.0 - 20.0 * h;
        let den = 4.0 * (sf - 2.0 * de);

        let n_payload = if num <= 0.0 {
            8.0
        } else {
            let terms = (num / den).ceil();
            8.0 + (terms * (cr_term + 4.0)).max(0.0)
        };

        let t_payload = n_payload * t_sym;
        t_preamble + t_payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_airtime_medium_fast() {
        let mf = LoraModemConfig::medium_fast();
        let airtime_10 = mf.airtime_ms(10);
        let airtime_100 = mf.airtime_ms(100);
        let airtime_237 = mf.airtime_ms(237);

        assert!(airtime_10 < airtime_100);
        assert!(airtime_100 < airtime_237);
        // Typical Medium-Fast airtime for ~100 bytes is ~200-300ms
        assert!(airtime_100 > 150.0 && airtime_100 < 400.0);
    }
}
