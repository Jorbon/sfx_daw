

use crate::*;

pub fn load_audio<P>(paths: &[P]) -> Result<Vec<AudioTrack<2>>, String> where P: AsRef<std::path::Path> {
	
	let num_tracks = paths.len();
	
	let mut files = vec![];
	let mut total_frames = 0;
	let mut track_ends = vec![];
	
	for path in paths {
		let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
		let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions { buffer_len: 1024 * 1024 });
		let mut format = FormatOptions::default();
		format.enable_gapless = true;
		
		let probe = symphonia::default::get_probe().format(&Hint::new(), mss, &format, &MetadataOptions::default()).map_err(|e| e.to_string())?;
		let track = probe.format.default_track().unwrap_or(&probe.format.tracks()[0]).clone();
		
		let n_frames = track.codec_params.n_frames.ok_or("No frame count")? as usize;
		total_frames += n_frames;
		files.push((probe, track));
		
		track_ends.push(total_frames);
	}
	
	
	let padded_length = ((total_frames - 1) / RESAMPLER_BLOCK_SIZE + 1) * RESAMPLER_BLOCK_SIZE;
	let mut audio_track = AudioTrack::<2>::new(padded_length);
	
	let rate = files[0].1.codec_params.sample_rate.unwrap();
	let channels = files[0].1.codec_params.channels.unwrap().count();
	
	
	let mut frame = 0;
	
	for (mut probe, track) in files {
		let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| e.to_string())?;
		
		loop {
			let packet = match probe.format.next_packet() {
				Ok(p) => p,
				Err(symphonia::core::errors::Error::IoError(_)) => break,
				Err(_) => panic!(),
			};
			
			if packet.track_id() != track.id { continue }
			
			let audio_buf = decoder.decode(&packet).map_err(|e| e.to_string())?;
			
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
	}
	
	
	
	
	
	let resample_ratio = 48000.0 / rate as f64;
	let max_resampled_block_size = (RESAMPLER_BLOCK_SIZE as f64 * resample_ratio) as usize + 10;
	
	// Can't just resample n_frames to get exact length, extra steps needed to avoid rounding errors
	let mut resampled_tracks = (0..num_tracks).map(|i| 
		AudioTrack::new((track_ends[i] as f64 * resample_ratio) as usize - (if i == 0 {0} else {track_ends[i - 1]} as f64 * resample_ratio) as usize)
	).collect::<Vec<_>>();
	
	
	
	let mut resampled_edge_buffer = AudioTrack::new((RESAMPLER_BLOCK_SIZE as f64 * resample_ratio) as usize + 10);
	
	let mut resampler = rubato::FastFixedIn::<f32>::new(resample_ratio, 1.0, rubato::PolynomialDegree::Septic, RESAMPLER_BLOCK_SIZE, channels).unwrap();
	// let mut resampler = rubato::SincFixedIn::<f32>::new(resample_ratio, 1.0, rubato::SincInterpolationParameters {
	// 	sinc_len: 256,
	// 	f_cutoff: 0.95,
	// 	interpolation: rubato::SincInterpolationType::Cubic,
	// 	oversampling_factor: 256,
	// 	window: rubato::WindowFunction::BlackmanHarris2,
	// }, block_size, channels).unwrap();
	
	
	let mut i = 0;
	let mut frame = 0;
	let mut resampled_track_frame = 0;
	
	'a: loop {
		
		let track_length = resampled_tracks[i].length();
		let resampled_frames_left_in_track = track_length - resampled_track_frame;
		
		if resampled_frames_left_in_track >= max_resampled_block_size {
			// Resample block directly into track data
			
			let (_, n) = resampler.process_into_buffer(
				&audio_track.get_slice(frame..(frame + RESAMPLER_BLOCK_SIZE)),
				&mut resampled_tracks[i].get_slice_mut(resampled_track_frame..(resampled_track_frame + max_resampled_block_size)),
				None
			).map_err(|e| e.to_string())?;
			
			resampled_track_frame += n;
			
		} else {
			// Probably not enough space to fit resampled frames, read to buffer instead
			
			let (_, n) = resampler.process_into_buffer(
				&audio_track.get_slice(frame..(frame + RESAMPLER_BLOCK_SIZE)),
				&mut resampled_edge_buffer.data,
				None
			).map_err(|e| e.to_string())?;
			
			if resampled_frames_left_in_track >= n {
				// Rare case where the end of the track actually fell in the safety margin just after this block, copy whole buffer and move on
				resampled_tracks[i].copy_from_range(resampled_track_frame..(resampled_track_frame + n), &resampled_edge_buffer, 0..n);
				resampled_track_frame += n;
				
			} else {
				// Track ends in the middle of the buffer, do partial copies into multiple tracks
				
				resampled_tracks[i].copy_from_range(resampled_track_frame..track_length, &resampled_edge_buffer, 0..resampled_frames_left_in_track);
				resampled_track_frame = n - resampled_frames_left_in_track;
				
				loop {
					i += 1;
					if i >= num_tracks { break 'a }
					
					let buffer_progress = n - resampled_track_frame;
					let track_length = resampled_tracks[i].length();
					if resampled_track_frame > track_length {
						resampled_tracks[i].copy_from_range(0..track_length, &resampled_edge_buffer, buffer_progress..(buffer_progress + track_length));
						resampled_track_frame -= track_length;
					} else {
						resampled_tracks[i].copy_from_range(0..resampled_track_frame, &resampled_edge_buffer, buffer_progress..n);
						break
					}
				}
				
			}
		}
		
		frame += RESAMPLER_BLOCK_SIZE;
	}
	
	
	
	Ok(resampled_tracks)
	
}

