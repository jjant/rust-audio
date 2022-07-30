use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, Stream};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let frequencies = Arc::new(Mutex::new([0.0_f32; 12]));

    let next_value = {
        let frequencies = Arc::clone(&frequencies);

        move |options: &mut SampleRequestOptions| {
            let freqs = frequencies.lock().unwrap();
            let freqs = *freqs.deref();

            //let count_non_zero = freqs.iter().copied().filter(|&d| d != 0.0).count();
            //            if count_non_zero == 0 {
            //                0.0
            //            } else {
            //                freqs
            //                    .iter()
            //                    .copied()
            //                    .map(|freq| sin(options, freq))
            //                    .sum::<f32>()
            //                    / (count_non_zero as f32)
            //            };
            //
            options.tick();

            options.tone(440.) * 0.1 + options.tone(880.) * 0.1
        }
    };

    let stream = build_output_audio_stream(next_value);
    stream.play().unwrap();

    run_event_loop(frequencies);
}

fn run_event_loop(frequencies: Arc<Mutex<[f32; 12]>>) {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new().build(&event_loop).unwrap();

    let keys = {
        use winit::event::VirtualKeyCode::*;

        [Q, W, E, R, T, Y, U, I, O, P, LBracket, RBracket]
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("Closing, bye");
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if let Some(index) = keys.iter().position(|&k| Some(k) == input.virtual_keycode) {
                    let mut frequencies = frequencies.lock().unwrap();

                    frequencies[index] = if input.state == winit::event::ElementState::Pressed {
                        fundamental_freq(index)
                    } else {
                        0.0
                    };

                    println!("Frequencies: {:?}", frequencies);
                }
                println!("Input: {:?}", input);
            }
            _ => {}
        }
    });
}

/// Returns 440, 466, 493, 523, 554, 587, 622, 659, 698, 739, 783, 830
/// (that is, A4, A#4, ..., G#5)
fn fundamental_freq(index: usize) -> f32 {
    if index >= 12 {
        panic!("Wrong note index: {}", index);
    }
    // 440 * (2^(n/12))
    440.0_f32 * (2.0_f32.powf(index as f32 / 12.0))
}

fn data_fn(
    data: &mut [f32],
    options: &mut SampleRequestOptions,
    next_value: &mut impl FnMut(&mut SampleRequestOptions) -> f32,
) {
    for frame in data.chunks_mut(options.num_channels) {
        let value = Sample::from(&next_value(options));

        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
    // println!("{:?}", data);
}

fn build_output_audio_stream(
    mut next_value: impl FnMut(&mut SampleRequestOptions) -> f32 + Send + 'static,
) -> Stream {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");
    println!("Output device : {}", device.name().unwrap());
    let supported_config = device.default_output_config().unwrap();
    println!("Default output config : {:?}", supported_config);
    println!("Channels: {}", supported_config.channels());
    println!("Sample rate: {:?}", supported_config.sample_rate());

    let err_fn = |err| eprintln!("Something bad: {}", err);

    let mut options = SampleRequestOptions {
        num_channels: supported_config.channels() as usize,
        sample_rate: supported_config.sample_rate().0 as f32,
        sample_clock: 0.0,
    };
    let stream = device
        .build_output_stream(
            &supported_config.config(),
            move |data, _| {
                // println!("Data len: {}", data.len());
                data_fn(data, &mut options, &mut next_value)
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

impl SampleRequestOptions {
    fn tone(&self, frequency: f32) -> f32 {
        (self.sample_clock * frequency * 2.0 * std::f32::consts::PI / self.sample_rate).sin()
    }

    fn tick(&mut self) {
        self.sample_clock = (self.sample_clock + 1.0) % self.sample_rate;
    }
}
