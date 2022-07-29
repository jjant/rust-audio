use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, Stream};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

fn main() {
    let frequency = Arc::new(Mutex::new(0.0));
    let mut next_value = {
        let frequency = Arc::clone(&frequency);

        move |options: &mut SampleRequestOptions| {
            let freq = frequency.lock().unwrap();
            let freq = *freq.deref();

            options.sample_clock = (options.sample_clock + 1.0) % options.sample_rate;
            (options.sample_clock * freq * 2.0 * std::f32::consts::PI / options.sample_rate).sin()
        }
    };

    let stream = build_output_audio_stream(next_value);
    stream.play().unwrap();

    let mut frequencies: [f32; 12] = [0.0; 12];
}

fn data_fn(data: &mut [f32], channels: usize, next_value: &mut impl FnMut() -> f32) {
    for frame in data.chunks_mut(channels) {
        let value = Sample::from(&next_value());

        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}

fn build_output_audio_stream(
    mut next_value: impl FnMut(&mut SampleRequestOptions) -> f32 + Send + 'static,
) -> Stream {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");
    let mut supported_configs_range = device.supported_output_configs().unwrap();
    let supported_config = supported_configs_range
        .next()
        .unwrap()
        .with_max_sample_rate();

    // dbg!(&supported_config);
    let err_fn = |err| eprintln!("Something bad: {}", err);

    let stream = device
        .build_output_stream(
            &supported_config.config(),
            move |data, _| {
                let mut options = SampleRequestOptions {
                    num_channels: supported_config.channels() as usize,
                    sample_rate: supported_config.sample_rate().0 as f32,
                    sample_clock: 0.0,
                };
                data_fn(data, options.num_channels, &mut || next_value(&mut options))
            },
            err_fn,
        )
        .unwrap();

    stream
}

struct SampleRequestOptions {
    sample_rate: f32,
    sample_clock: f32,
    num_channels: usize,
}
