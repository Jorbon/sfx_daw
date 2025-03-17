use std::{collections::VecDeque, sync::{Arc, RwLock}};

use cpal::{Device, OutputCallbackInfo, SampleFormat, SampleRate, Stream};

use crate::*;



#[derive(Clone)]
pub struct PlaybackState {
	pub index: usize,
	pub frame: usize,
	pub playing: bool,
	pub queue: VecDeque<usize>,
}

pub struct AudioPlayer {
	stream: Stream,
	pub tracks: Arc<RwLock<Vec<Option<AudioTrack<2>>>>>,
	pub playback_state: Arc<RwLock<Option<PlaybackState>>>,
}



impl AudioPlayer {
	pub fn new(device: Device) -> Self {
		let mut supported_configs_range = device.supported_output_configs().expect("No device configs found");
		let mut config = None;
		while let Some(c) = supported_configs_range.next() {
			if c.sample_format() == SampleFormat::F32 {
				config = Some(c);
				break
			}
		}
		let config = config.expect("No F32 sample format available").with_sample_rate(SampleRate(48000));
		
		
		let tracks: Arc<RwLock<Vec<Option<AudioTrack<2>>>>> = Arc::new(RwLock::new(vec![]));
		
		let playback_state: Arc<RwLock<Option<PlaybackState>>> = Arc::new(RwLock::new(None));
		
		let tracks_backend = Arc::clone(&tracks);
		let playback_state_backend = Arc::clone(&playback_state);
		
		
		let stream = device.build_output_stream(&config.into(), move |data: &mut [f32], _output_callback_info: &OutputCallbackInfo| {
			let mut state_binding = playback_state_backend.write().unwrap();
			
			if let Some(state_params) = state_binding.as_mut() {
				
				let tracks_binding = tracks_backend.read().unwrap();
				if let Some(Some(track)) = (*tracks_binding).get(state_params.index) {
					if state_params.playing {
						
						let current_frame = state_params.frame;
						let frames_left = track.chapter_ends.last().unwrap_or(&track.padded_length()) - current_frame;
						let frames_now = data.len() / 2;
						
						if frames_left >= frames_now {
							state_params.frame += frames_now;
							drop(state_binding);
							
							let mut i = 0;
							for frame in 0..frames_now {
								for channel in 0..2 {
									data[i] = track.data[channel][current_frame + frame];
									i += 1;
								}
							}
						} else {
							let next_track = 
							if let Some(next_index) = state_params.queue.pop_front() {
								if let Some(Some(next_track)) = (*tracks_binding).get(next_index) {
									state_params.index = next_index;
									state_params.frame = frames_now - frames_left;
									Some(next_track)
								} else {
									state_params.frame = 0;
									state_params.playing = false;
									None
								}
							} else {
								state_params.frame = 0;
								state_params.playing = false;
								None
							};
							
							drop(state_binding);
							
							let mut i = 0;
							for frame in 0..frames_left {
								for channel in 0..2 {
									data[i] = track.data[channel][current_frame + frame];
									i += 1;
								}
							}
							
							if let Some(next_track) = next_track {
								for frame in 0..(frames_now - frames_left) {
									for channel in 0..2 {
										data[i] = next_track.data[channel][current_frame + frame];
										i += 1;
									}
								}
							}
							
							
						}
						
						
					}
					
				} else {
					*state_binding = None;
				}
			}
			
		}, move |err| { println!("{err}"); }, None).unwrap();
		
		
		stream.play().unwrap();
		
		Self {
			stream,
			tracks,
			playback_state,
		}
	}
	
	pub fn resume_stream(&self) {
		self.stream.play().unwrap()
	}
	
	pub fn pause_stream(&self) {
		self.stream.pause().unwrap()
	}
	
	pub fn add_track(&self, track: AudioTrack<2>) -> usize {
		let mut tracks_binding = self.tracks.write().unwrap();
		
		for i in 0..tracks_binding.len() {
			if tracks_binding[i].is_none() {
				tracks_binding[i] = Some(track);
				return i
			}
		}
		
		tracks_binding.push(Some(track));
		return tracks_binding.len() - 1
	}
	
	pub fn add_tracks(&self, tracks: impl Iterator<Item = AudioTrack<2>>) -> Vec<usize> {
		tracks.map(|track| self.add_track(track)).collect()
	}
	
	pub fn remove_track(&self, index: usize) {
		let mut tracks_binding = self.tracks.write().unwrap();
		if index >= tracks_binding.len() { return }
		tracks_binding[index] = None;
		if index == tracks_binding.len() - 1 {
			loop {
				match tracks_binding.last() {
					Some(None) => { tracks_binding.pop(); }
					None => return,
					_ => ()
				}
			}
		}
	}
	
	pub fn play_track(&self, index: usize, chapter: usize) {
		if let Some(Some(track)) = (*self.tracks.read().unwrap()).get(index) {
			*self.playback_state.write().unwrap() = Some(PlaybackState {
				index,
				frame: if chapter == 0 {0} else {*track.chapter_ends.get(chapter - 1).unwrap_or(&0)},
				playing: true,
				queue: VecDeque::new(),
			});
		}
	}
	
	pub fn is_playing(&self) -> bool {
		match *self.playback_state.read().unwrap() {
			Some(PlaybackState { playing: true, .. }) => true,
			_ => false
		}
	}
	
	pub fn queue_track(&self, index: usize) {
		let mut state_binding = self.playback_state.write().unwrap();
		if let Some(PlaybackState { queue, .. }) = state_binding.as_mut() {
			queue.push_back(index);
		}
		
	}
	
}






