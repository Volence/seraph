use std::path::Path;

pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;
        let s0 = samples[idx.min(samples.len() - 1)];
        let s1 = samples[(idx + 1).min(samples.len() - 1)];
        output.push(s0 + (s1 - s0) * frac);
    }

    output
}

pub fn quantize_u8(samples: &[f32]) -> Vec<u8> {
    samples
        .iter()
        .map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            ((clamped * 127.0) + 128.0) as u8
        })
        .collect()
}

pub fn import_wav(wav_path: &Path, target_rate: u32) -> Result<Vec<u8>, String> {
    let reader =
        hound::WavReader::open(wav_path).map_err(|e| format!("failed to open WAV: {e}"))?;
    let spec = reader.spec();

    let float_samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max_val))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        }
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?,
    };

    let mono = if spec.channels > 1 {
        float_samples
            .chunks(spec.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect()
    } else {
        float_samples
    };

    let resampled = resample(&mono, spec.sample_rate, target_rate);
    Ok(quantize_u8(&resampled))
}

pub fn import_raw(raw_path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(raw_path).map_err(|e| format!("failed to read raw PCM: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_same_rate_is_identity() {
        let samples = vec![0.0, 0.5, 1.0, -1.0];
        let out = resample(&samples, 44100, 44100);
        assert_eq!(out, samples);
    }

    #[test]
    fn test_resample_halves_length() {
        let samples: Vec<f32> = (0..1000).map(|i| (i as f32) / 1000.0).collect();
        let out = resample(&samples, 44100, 22050);
        let expected_len = (1000.0 * 22050.0 / 44100.0) as usize;
        assert!((out.len() as i32 - expected_len as i32).abs() <= 1);
    }

    #[test]
    fn test_resample_doubles_length() {
        let samples: Vec<f32> = (0..100).map(|i| (i as f32) / 100.0).collect();
        let out = resample(&samples, 22050, 44100);
        let expected_len = (100.0 * 44100.0 / 22050.0) as usize;
        assert!((out.len() as i32 - expected_len as i32).abs() <= 1);
    }

    #[test]
    fn test_quantize_center_is_128() {
        let out = quantize_u8(&[0.0]);
        assert_eq!(out[0], 128);
    }

    #[test]
    fn test_quantize_max_is_255() {
        let out = quantize_u8(&[1.0]);
        assert_eq!(out[0], 255);
    }

    #[test]
    fn test_quantize_min_is_1() {
        let out = quantize_u8(&[-1.0]);
        assert_eq!(out[0], 1);
    }

    #[test]
    fn test_quantize_clamps_overflow() {
        let out = quantize_u8(&[2.0, -2.0]);
        assert_eq!(out[0], 255);
        assert_eq!(out[1], 1);
    }

    #[test]
    fn test_import_wav_file() {
        let dir = std::env::temp_dir().join(format!("megadaw_wav_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav_path = dir.join("test.wav");

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
        for i in 0..4410 {
            let t = i as f32 / 44100.0;
            let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
            writer.write_sample((sample * 32767.0) as i16).unwrap();
        }
        writer.finalize().unwrap();

        let pcm = import_wav(&wav_path, 16000).unwrap();
        let expected_len = (4410.0 * 16000.0 / 44100.0) as usize;
        assert!((pcm.len() as i32 - expected_len as i32).abs() <= 2);
        assert!(pcm.iter().any(|&s| s != 128), "should contain non-silent data");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_wav_stereo_to_mono() {
        let dir = std::env::temp_dir().join(format!("megadaw_stereo_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav_path = dir.join("stereo.wav");

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
        for _ in 0..1000 {
            writer.write_sample(16383i16).unwrap(); // left
            writer.write_sample(-16383i16).unwrap(); // right
        }
        writer.finalize().unwrap();

        let pcm = import_wav(&wav_path, 44100).unwrap();
        assert_eq!(pcm.len(), 1000);
        for &s in &pcm {
            assert!((s as i16 - 128).abs() <= 1);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
