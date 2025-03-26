

use rustfft::FftPlanner;

use crate::*;



pub fn test_filter(track: &AudioTrack<2>) -> AudioTrack<2> {
	let fft = FftPlanner::new();
	fft.plan_fft_forward(track.padded_length());
	
	todo!()
}



