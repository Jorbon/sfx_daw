use std::sync::mpsc::{Receiver, Sender};

use crate::*;


pub struct AudioPlayer {
	pub buffer: AudioBuffer<f32>,
    pub position: usize,
	pub playing: bool,
	tx: Sender<AudioPlayerNotice>,
	rx: Receiver<AudioPlayerCommand>,
}

pub struct AudioPlayerController {
	tx: Sender<AudioPlayerCommand>,
	rx: Receiver<AudioPlayerNotice>,
}

pub enum AudioPlayerCommand {
	Play,
	Stop,
	Seek(usize),
}

pub enum AudioPlayerNotice {
	ReachedEnd,
}


pub fn create_audio_player(buffer: AudioBuffer<f32>) -> (AudioPlayerController, AudioPlayer) {
	let (controller_tx, rx) = std::sync::mpsc::channel();
	let (tx, controller_rx) = std::sync::mpsc::channel();
	
	(AudioPlayerController {
		tx: controller_tx,
		rx: controller_rx,
	}, AudioPlayer {
		buffer, position: 0, playing: false, tx, rx
	})
}


impl AudioPlayerController {
	pub fn play(&self) {
		self.tx.send(AudioPlayerCommand::Play).unwrap();
	}
	pub fn stop(&self) {
		self.tx.send(AudioPlayerCommand::Stop).unwrap();
	}
	pub fn seek(&self, frame: usize) {
		self.tx.send(AudioPlayerCommand::Seek(frame)).unwrap();
	}
	pub fn read_notice(&self) -> Option<AudioPlayerNotice> {
		self.rx.try_recv().ok()
	}
}


impl AudioPlayer {
	pub fn poll(&mut self, output_buffer: &mut [f32]) -> bool {
		for command in self.rx.try_iter() {
			match command {
				AudioPlayerCommand::Play => self.playing = true,
				AudioPlayerCommand::Stop => self.playing = false,
				AudioPlayerCommand::Seek(frame) => self.position = frame,
			}
		}
		
		if !self.playing { return false }
		
		let n_left = self.buffer.data.len() - self.position;
        if n_left >= output_buffer.len() {
            output_buffer.copy_from_slice(&self.buffer.data[self.position..(self.position + output_buffer.len())]);
			self.position += output_buffer.len();
			false
        } else {
            output_buffer[..n_left].copy_from_slice(&self.buffer.data[self.position..]);
			self.position = 0;
			self.playing = false;
			self.tx.send(AudioPlayerNotice::ReachedEnd).unwrap();
			true
        }
	}
}
