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
    let frequencies = Arc::new(Mutex::new(vec![None; 12]));

    let next_value = move |frequencies: &mut [Option<TimedFrequency>],
                           options: &mut SampleRequestOptions| {
        options.tick();

        let count_non_zero = frequencies.iter().filter_map(|d| d.as_ref()).count();
        if count_non_zero == 0 {
            0.0
        } else {
            frequencies
                .iter_mut()
                .filter_map(|x| x.as_mut())
                .map(|freq| {
                    freq.tick(options.sample_rate);
                    freq.tone(options.sample_rate)
                })
                .sum::<f32>()
                / (count_non_zero as f32)
        }
    };

    let stream = build_output_audio_stream(Arc::clone(&frequencies), next_value);
    stream.play().unwrap();

    run_event_loop(frequencies);
}

fn run_event_loop(frequencies: Arc<Mutex<Vec<Option<TimedFrequency>>>>) {
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
                        frequencies[index]
                            .or_else(|| Some(TimedFrequency::new(fundamental_freq(index))))
                    } else {
                        None
                    };

                    println!("Frequencies: {:?}", frequencies);
                }
            }
            _ => {}
        }
    });
}

/// Returns frequency for A4, A#4, ..., G#5
fn fundamental_freq(index: usize) -> f32 {
    if index >= 12 {
        panic!("Wrong note index: {}", index);
    }

    (440.0_f32 * (2.0_f32.powf(index as f32 / 12.0))).round()
}

fn data_fn(
    data: &mut [f32],
    options: &mut SampleRequestOptions,
    frequencies: &Mutex<Vec<Option<TimedFrequency>>>,
    next_value: &mut impl FnMut(&mut [Option<TimedFrequency>], &mut SampleRequestOptions) -> f32,
) {
    let mut freqs = frequencies.lock().unwrap();

    for frame in data.chunks_mut(options.num_channels) {
        let value = Sample::from(&next_value(freqs.as_mut(), options));

        // eprintln!("{}", value);
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}

fn build_output_audio_stream(
    frequencies: Arc<Mutex<Vec<Option<TimedFrequency>>>>,
    mut next_value: impl FnMut(&mut [Option<TimedFrequency>], &mut SampleRequestOptions) -> f32
        + Send
        + 'static,
) -> Stream {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");
    println!("Output device : {}", device.name().unwrap());
    let supported_config = device.default_output_config().unwrap();
    println!("Default output config : {:?}", supported_config);

    let err_fn = |err| eprintln!("Something bad: {}", err);

    let mut options = SampleRequestOptions {
        num_channels: supported_config.channels() as usize,
        sample_rate: supported_config.sample_rate().0 as f32,
        sample_clock: 0.0,
    };
    let stream = device
        .build_output_stream(
            &supported_config.config(),
            move |data, _| data_fn(data, &mut options, &frequencies, &mut next_value),
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

#[derive(Clone, Debug, Copy)]
struct TimedFrequency {
    frequency: f32,
    sample_clock: f32,
}

impl TimedFrequency {
    fn new(frequency: f32) -> Self {
        Self {
            frequency,
            sample_clock: 0.0,
        }
    }

    fn tone(&self, sample_rate: f32) -> f32 {
        let t = self.sample_clock / sample_rate;
        let arg = t * self.frequency * 2.0 * std::f32::consts::PI;
        arg.sin()
    }

    fn tick(&mut self, sample_rate: f32) {
        self.sample_clock = (self.sample_clock + 1.0) % sample_rate;
    }
}
