#[allow(dead_code)] mod player; use player::*;


use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::{audio::{SampleBuffer, SignalSpec}, codecs::DecoderOptions, formats::FormatOptions, io::{MediaSourceStream, MediaSourceStreamOptions}, meta::MetadataOptions, probe::Hint};


pub struct AudioBuffer<T> {
	pub data: Vec<T>,
	pub channels: usize,
	pub sample_rate: u32,
}

pub fn load_audio<P>(path: P) -> Result<AudioBuffer<f32>, String> where P: AsRef<std::path::Path> {
	let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
	let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions { buffer_len: 64 * 1024 });
	
	let probe = symphonia::default::get_probe().format(&Hint::new(), mss, &FormatOptions::default(), &MetadataOptions::default()).map_err(|e| e.to_string())?;
	let mut format = probe.format;
	let track = format.default_track().unwrap_or(&format.tracks()[0]);
	// dbg!(track.codec_params.channels, track.codec_params.sample_rate, track.codec_params.n_frames, track.codec_params.bits_per_sample);
	
	let n_frames = track.codec_params.n_frames.unwrap();
	let rate = track.codec_params.sample_rate.unwrap();
	let channels = track.codec_params.channels.unwrap();
	
	let mut sample_buf = std::mem::ManuallyDrop::new(SampleBuffer::<f32>::new(n_frames, SignalSpec { rate, channels }));
	
	let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| e.to_string())?;
	
	loop {
		let packet = match format.next_packet() {
			Ok(p) => p,
			Err(symphonia::core::errors::Error::IoError(_)) => break,
			Err(_) => panic!(),
		};
		sample_buf.append_interleaved_ref(decoder.decode(&packet).expect("Packet decoding error"));
	}
	
	Ok(AudioBuffer {
		data: unsafe {
			Vec::from_raw_parts(sample_buf.samples().as_ptr() as *mut f32, sample_buf.len(), sample_buf.capacity())
		},
		channels: channels.count(),
		sample_rate: rate,
	})
}








fn main() {
	let path = "Desktop/mc/14 Aria Math.flac";
	
	println!("Reading {path}...");
	let song_buffer = load_audio(std::env::home_dir().unwrap().join(path)).unwrap();
	println!("File read!");
	
	let audio_host = cpal::default_host();
	let device = audio_host.default_output_device().expect("No default device found");
	let mut supported_configs_range = device.supported_output_configs().expect("No device configs found");
	
	let mut config = None;
	while let Some(c) = supported_configs_range.next() {
		if c.sample_format() == cpal::SampleFormat::F32 {
			config = Some(c);
			break
		}
	}
	let config = config.expect("No F32 sample format available").with_sample_rate(cpal::SampleRate(48000));
	
	
	let (tx, rx) = std::sync::mpsc::channel();
	let mut audio_player: Option<AudioPlayer> = None;
	
	let stream = device.build_output_stream(&config.into(), move |data: &mut [f32], _output_callback_info: &cpal::OutputCallbackInfo| {
		for player in rx.try_iter() {
			audio_player = Some(player);
		}
		if let Some(player) = &mut audio_player {
			player.poll(data);
		}
	}, move |err| { println!("{err}"); }, None).unwrap();
	stream.play().unwrap();
	
	
	let (controller, player) = create_audio_player(song_buffer);
	tx.send(player).unwrap();
	controller.play();
	
	loop {
		if let Some(notice) = controller.read_notice() {
			match notice {
				AudioPlayerNotice::ReachedEnd => break,
			}
		}
	}
	
	println!("Done!");
}
