use super::*;
use std::time::{Duration, Instant};
pub fn play(enumerator: &IMMDeviceEnumerator, endpoint: &str) -> Result<(), String> {
    unsafe {
        let device = if endpoint == "default" {
            enumerator.GetDefaultAudioEndpoint(eRender, eConsole)
        } else {
            let id = wide(endpoint);
            enumerator.GetDevice(PCWSTR(id.as_ptr()))
        }
        .map_err(err)?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(err)?;
        let raw = client.GetMixFormat().map_err(err)?;
        struct Format(*mut WAVEFORMATEX);
        impl Drop for Format {
            fn drop(&mut self) {
                unsafe { CoTaskMemFree(Some(self.0.cast())) }
            }
        }
        let format = Format(raw);
        let f = &*format.0;
        let (channels, bits, rate, block) = (
            f.nChannels as usize,
            f.wBitsPerSample as usize,
            f.nSamplesPerSec as usize,
            f.nBlockAlign as usize,
        );
        let is_float = if f.wFormatTag == 3 {
            true
        } else if f.wFormatTag == 0xfffe {
            let ext = &*(raw as *const WAVEFORMATEXTENSIBLE);
            ext.SubFormat.data1 == 3
        } else {
            false
        };
        if channels == 0
            || rate == 0
            || block != channels * (bits / 8)
            || !(is_float && bits == 32 || !is_float && [16, 32].contains(&bits))
        {
            return Err("This endpoint's mix format is not supported by the speaker test. Choose a 16-bit PCM or 32-bit shared-mode format in Windows Sound settings.".into());
        }
        client
            .Initialize(AUDCLNT_SHAREMODE_SHARED, 0, 1_000_000, 0, raw, None)
            .map_err(err)?;
        let render: IAudioRenderClient = client.GetService().map_err(err)?;
        let buffer_size = client.GetBufferSize().map_err(err)?;
        let mut frame = 0usize;
        let total = rate * 350 / 1000;
        client.Start().map_err(err)?;
        let started = Instant::now();
        let result = (|| -> Result<(), String> {
            while frame < total && started.elapsed() < Duration::from_secs(2) {
                let available =
                    buffer_size.saturating_sub(client.GetCurrentPadding().map_err(err)?);
                let count = available.min((total - frame) as u32);
                if count == 0 {
                    std::thread::sleep(Duration::from_millis(8));
                    continue;
                }
                let buffer = render.GetBuffer(count).map_err(err)?;
                for i in 0..count as usize {
                    let t = (frame + i) as f32 / rate as f32;
                    let envelope = (t / 0.025).min(1.0) * ((0.350 - t) / 0.050).clamp(0.0, 1.0);
                    let sample = (t * 440.0 * std::f32::consts::TAU).sin() * 0.06 * envelope;
                    for ch in 0..channels {
                        let p = buffer.add(i * block + ch * (bits / 8));
                        if is_float {
                            std::ptr::write_unaligned(p.cast::<f32>(), sample);
                        } else if bits == 16 {
                            std::ptr::write_unaligned(
                                p.cast::<i16>(),
                                (sample * i16::MAX as f32) as i16,
                            );
                        } else {
                            std::ptr::write_unaligned(
                                p.cast::<i32>(),
                                (sample * i32::MAX as f32) as i32,
                            );
                        }
                    }
                }
                render.ReleaseBuffer(count, 0).map_err(err)?;
                frame += count as usize;
            }
            while client.GetCurrentPadding().map_err(err)? > 0
                && started.elapsed() < Duration::from_secs(2)
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(())
        })();
        let _ = client.Stop();
        result
    }
}
