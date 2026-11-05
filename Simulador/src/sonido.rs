// Efecto de perder una vida sintetizado por codigo: no necesita archivos de audio

pub const SAMPLE_RATE: u32 = 44_100;
const DURATION: f32 = 0.55;

// Golpe metalico con un tono que cae: muestras de 16 bits en mono
pub fn life_lost() -> Vec<i16> {
    let count = (DURATION * SAMPLE_RATE as f32) as usize;
    let mut samples = Vec::with_capacity(count);
    let mut noise_state: u32 = 90_731;
    let mut phase = 0f32;
    for i in 0..count {
        let t = i as f32 / SAMPLE_RATE as f32;
        // El tono baja de 420 Hz a unos 70 Hz
        let frequency = 70. + 350. * (-t * 7.).exp();
        phase += std::f32::consts::TAU * frequency / SAMPLE_RATE as f32;
        // Un seno saturado suena mas a choque que un seno limpio
        let tone = (phase.sin() * 2.2).clamp(-1., 1.);
        noise_state = noise_state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = (noise_state >> 16) as f32 / 32_768. - 1.;
        // Primero domina el ruido del impacto y despues queda el tono
        let crash = noise * (-t * 22.).exp();
        let body = tone * (-t * 6.).exp();
        // Entrada y salida cortas para evitar chasquidos
        let attack = (t / 0.002).min(1.);
        let release = ((DURATION - t) / 0.03).clamp(0., 1.);
        let value = (crash * 0.6 + body * 0.55) * attack * release;
        samples.push((value.clamp(-1., 1.) * 30_000.) as i16);
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(samples: &[i16]) -> i32 {
        samples.iter().map(|s| (*s as i32).abs()).max().unwrap_or(0)
    }

    #[test]
    fn sound_is_short_loud_at_first_and_fades_to_silence() {
        let sound = life_lost();
        assert_eq!(sound.len(), (DURATION * SAMPLE_RATE as f32) as usize);
        let tenth = sound.len() / 10;
        assert!(peak(&sound[..tenth]) > 15_000);
        assert!(peak(&sound[sound.len() - tenth..]) < 5_000);
        assert!(sound.last().unwrap().abs() < 300);
    }

    #[test]
    fn sound_is_the_same_every_time() {
        assert_eq!(life_lost(), life_lost());
    }
}
