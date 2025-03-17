#[allow(dead_code)] mod track; use track::*;
#[allow(dead_code)] mod player; use player::*;

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use symphonia::core::{audio::{AudioBufferRef, Signal}, codecs::DecoderOptions, conv::IntoSample, formats::FormatOptions, io::{MediaSourceStream, MediaSourceStreamOptions}, meta::MetadataOptions, probe::Hint};
use rubato::Resampler;





pub fn load_audio<P>(path: P) -> Result<AudioTrack<2>, String> where P: AsRef<std::path::Path> {
	let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
	let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions { buffer_len: 1024 * 1024 });
	let mut format = FormatOptions::default();
	format.enable_gapless = true;
	
	let probe = symphonia::default::get_probe().format(&Hint::new(), mss, &format, &MetadataOptions::default()).map_err(|e| e.to_string())?;
	let mut format = probe.format;
	let track = format.default_track().unwrap_or(&format.tracks()[0]);
	
	let n_frames = track.codec_params.n_frames.unwrap() as usize;
	let rate = track.codec_params.sample_rate.unwrap();
	let channels = track.codec_params.channels.unwrap().count();
	
	
	let block_size = 1024;
	
	let padded_length = ((n_frames - 1) / block_size + 1) * block_size;
	
	let mut audio_track = AudioTrack::<2>::new(padded_length);
	audio_track.length = n_frames;
	
	let mut frame = 0;
	
	let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| e.to_string())?;
	
	loop {
		let packet = match format.next_packet() {
			Ok(p) => p,
			Err(symphonia::core::errors::Error::IoError(_)) => break,
			Err(_) => panic!(),
		};
		
		let audio_buf = decoder.decode(&packet).expect("Packet decoding error");
		
		let frames_now = packet.dur as usize;
		
		match audio_buf {
			AudioBufferRef::F32(buf) => {
				for channel in 0..2 {
					audio_track.data[channel][frame..(frame + frames_now)].copy_from_slice(buf.chan(channel));
				}
			}
			AudioBufferRef::F64(buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
			AudioBufferRef::S8 (buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
			AudioBufferRef::S16(buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
			AudioBufferRef::S24(buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
			AudioBufferRef::S32(buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
			AudioBufferRef::U8 (buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
			AudioBufferRef::U16(buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
			AudioBufferRef::U24(buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
			AudioBufferRef::U32(buf) => for channel in 0..2 { let samples = buf.chan(channel); for i in 0..frames_now { audio_track.data[channel][frame + i] = samples[i].into_sample(); } }
		}
		
		frame += packet.dur as usize;
		
	}
	
	
	
	println!("Resampling...");
	
	let resample_ratio = 48000.0 / rate as f64;
	
	let mut resampled_track = AudioTrack::new((audio_track.padded_length() as f64 * resample_ratio) as usize + 10);
	
	// let mut resampler = rubato::FastFixedIn::<f32>::new(resample_ratio, 1.0, rubato::PolynomialDegree::Septic, block_size, channels).unwrap();
	let mut resampler = rubato::SincFixedIn::<f32>::new(resample_ratio, 1.0, rubato::SincInterpolationParameters {
		sinc_len: 256,
		f_cutoff: 0.95,
		interpolation: rubato::SincInterpolationType::Cubic,
		oversampling_factor: 256,
		window: rubato::WindowFunction::BlackmanHarris2,
	}, block_size, channels).unwrap();
	
	
	let mut frame = 0;
	let mut frame_resampled = 0;
	loop {
		if frame + block_size > audio_track.padded_length() {
			break
		}
		
		let n = resampler.process_into_buffer(
			&audio_track.get_slice(frame..(frame + block_size)), 
			&mut resampled_track.get_slice_mut(frame_resampled..resampled_track.padded_length()),
			None
		).unwrap();
		
		frame += block_size;
		frame_resampled += n.1;
	}
	
	
	dbg!(audio_track.padded_length(), audio_track.length, frame, resampled_track.padded_length(), resampled_track.length, frame_resampled);
	
	
	Ok(resampled_track)
}






#[allow(deprecated)]
fn main() {
	let device = cpal::default_host().default_output_device().expect("No default device found");
	
	let audio_player = AudioPlayer::new(device);
	
	let track = audio_player.add_track(load_audio(std::env::home_dir().unwrap().join("OneDrive/Music/cd/Coldplay/Viva La Vida Or Death And All His Friends/01 Life In Technicolor.flac")).unwrap());
	let track2 = audio_player.add_track(load_audio(std::env::home_dir().unwrap().join("OneDrive/Music/cd/Coldplay/Viva La Vida Or Death And All His Friends/02 Cemeteries of London.flac")).unwrap());
	audio_player.play_track(track);
	audio_player.queue_track(track2);
	
	loop {
		if !audio_player.is_playing() { break }
	}
	
	println!("Done!");
}
