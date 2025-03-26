#[allow(dead_code)] mod track; use track::*;
#[allow(dead_code)] mod load; use load::*;
#[allow(dead_code)] mod player; use player::*;
// #[allow(dead_code)] mod filter; use filter::*;

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use symphonia::core::{audio::{AudioBufferRef, Signal}, codecs::DecoderOptions, conv::IntoSample, formats::FormatOptions, io::{MediaSourceStream, MediaSourceStreamOptions}, meta::MetadataOptions, probe::Hint};
use rubato::Resampler;



const RESAMPLER_BLOCK_SIZE: usize = 1024;







#[allow(deprecated)]
fn main() {
	let device = cpal::default_host().default_output_device().expect("No default device found");
	
	let audio_player = AudioPlayer::new(device);
	
	let dir = std::env::home_dir().unwrap().join("OneDrive/Music/cd/Pink Floyd/The Wall [Disc 1]");
	let mut files = vec![];
	for entry in std::fs::read_dir(dir).unwrap() {
		let entry = entry.unwrap();
		if entry.metadata().unwrap().is_file() {
			files.push(entry.path());
		}
	}
	let audio = load_audio(&files).unwrap();
	
	
	// let audio = load_audio(&[
	// 	std::env::home_dir().unwrap().join("OneDrive/Music/cd/Coldplay/Viva La Vida Or Death And All His Friends/01 Life In Technicolor.flac"),
	// 	std::env::home_dir().unwrap().join("OneDrive/Music/cd/Coldplay/Viva La Vida Or Death And All His Friends/02 Cemeteries of London.flac"),
	// ]).unwrap();
	
	// let audio = load_audio(&[
	// 	std::env::home_dir().unwrap().join("OneDrive/Music/cd/Pierce The Veil/Collide With The Sky/01 May These Noises Startle You In Your Sleep Tonight.flac"),
	// 	std::env::home_dir().unwrap().join("OneDrive/Music/cd/Pierce The Veil/Collide With The Sky/02 Hell Above.flac"),
	// ]).unwrap();
	
	// let filtered_audio = test_filter(&audio);
	
	
	println!("Playing");
	
	
	
	let tracks = audio_player.add_tracks(audio.into_iter());
	
	audio_player.play_track(tracks[0]);
	for i in 1..tracks.len() { audio_player.queue_track(tracks[i]); }
	
	// audio_player.seek(145.0);
	
	
	loop {
		if !audio_player.is_playing() { break }
	}
	
	println!("Done!");
}
