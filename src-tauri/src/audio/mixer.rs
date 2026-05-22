use super::types::{Frame, Sample};

pub struct MixInput<'a> {
    pub mic: &'a [Sample],
    pub music: &'a [Sample],
    pub carrier: &'a [Sample],
}

#[derive(Debug, Clone, Copy)]
pub struct MixGains {
    pub mic: f32,
    pub music: f32,
    pub carrier: f32,
    /// If mic RMS < this threshold, carrier is enabled. Set to 0 to always-off.
    pub carrier_gate_rms: f32,
}

impl Default for MixGains {
    fn default() -> Self {
        MixGains {
            mic: 1.0,
            music: 0.8,
            carrier: 0.05,
            carrier_gate_rms: 0.01,
        }
    }
}

/// Mix mic + music + (gated) carrier with a soft-knee limiter at +/-1.0.
/// All input slices MUST be the same length; output is a fresh Vec of that length.
pub fn mix(input: MixInput, gains: &MixGains) -> Frame {
    let n = input.mic.len();
    assert_eq!(input.music.len(), n, "music length mismatch");
    assert_eq!(input.carrier.len(), n, "carrier length mismatch");

    let mic_rms = rms(input.mic);
    let carrier_active = mic_rms < gains.carrier_gate_rms;
    let carrier_g = if carrier_active { gains.carrier } else { 0.0 };

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let s = input.mic[i] * gains.mic
            + input.music[i] * gains.music
            + input.carrier[i] * carrier_g;
        out.push(soft_limit(s));
    }
    Frame(out)
}

fn rms(s: &[f32]) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = s.iter().map(|x| x * x).sum();
    (sum_sq / s.len() as f32).sqrt()
}

fn soft_limit(s: f32) -> f32 {
    s.tanh()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input_yields_silent_output() {
        let mic = [0.0; 480];
        let music = [0.0; 480];
        let carrier = [0.5; 480];
        let gains = MixGains {
            carrier_gate_rms: 0.0,
            ..MixGains::default()
        };
        let out = mix(
            MixInput {
                mic: &mic,
                music: &music,
                carrier: &carrier,
            },
            &gains,
        );
        assert!(out.0.iter().all(|s| s.abs() < 1e-6));
    }

    #[test]
    fn mic_only_passes_through_with_gain() {
        let mic = [0.5; 480];
        let music = [0.0; 480];
        let carrier = [0.0; 480];
        let gains = MixGains {
            mic: 1.0,
            music: 0.0,
            carrier: 0.0,
            carrier_gate_rms: 0.0,
        };
        let out = mix(
            MixInput {
                mic: &mic,
                music: &music,
                carrier: &carrier,
            },
            &gains,
        );
        approx::assert_abs_diff_eq!(out.0[0], 0.5_f32.tanh(), epsilon = 1e-5);
    }

    #[test]
    fn carrier_engages_when_mic_below_threshold() {
        let mic = [0.001; 480];
        let music = [0.0; 480];
        let carrier = [0.4; 480];
        let gains = MixGains {
            mic: 0.0,
            music: 0.0,
            carrier: 1.0,
            carrier_gate_rms: 0.01,
        };
        let out = mix(
            MixInput {
                mic: &mic,
                music: &music,
                carrier: &carrier,
            },
            &gains,
        );
        assert!(
            out.0[0] > 0.3,
            "carrier should be audible: got {}",
            out.0[0]
        );
    }

    #[test]
    fn carrier_disengages_when_mic_above_threshold() {
        let mic = [0.5; 480];
        let music = [0.0; 480];
        let carrier = [0.4; 480];
        let gains = MixGains {
            mic: 0.0,
            music: 0.0,
            carrier: 1.0,
            carrier_gate_rms: 0.01,
        };
        let out = mix(
            MixInput {
                mic: &mic,
                music: &music,
                carrier: &carrier,
            },
            &gains,
        );
        assert!(out.0[0].abs() < 1e-6, "carrier should be gated off: got {}", out.0[0]);
    }

    #[test]
    fn limiter_prevents_clip() {
        let mic = [10.0; 480];
        let music = [10.0; 480];
        let carrier = [0.0; 480];
        let out = mix(
            MixInput {
                mic: &mic,
                music: &music,
                carrier: &carrier,
            },
            &MixGains::default(),
        );
        assert!(out.0.iter().all(|s| s.abs() < 1.0));
    }
}
