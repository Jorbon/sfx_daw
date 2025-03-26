use std::ops::Range;


#[derive(Clone)]
pub struct AudioTrack<const N: usize> {
	pub data: [Box<[f32]>; N],
}

impl<const N: usize> AudioTrack<N> {
	pub fn new(frames: usize) -> Self {
		Self {
			data: core::array::from_fn(|_c| vec![0.0; frames].into_boxed_slice()),
		}
	}
	
	pub fn clone_range(track: &AudioTrack<N>, range: Range<usize>) -> Self {
		Self {
			data: core::array::from_fn(|c| track.data[c][range.clone()].iter().cloned().collect()),
		}
	}
	
	pub fn copy_from_range(&mut self, range: Range<usize>, other_track: &AudioTrack<N>, other_range: Range<usize>) {
		for c in 0..N {
			self.data[c][range.clone()].copy_from_slice(&other_track.data[c][other_range.clone()]);
		}
	}
	
	pub fn length(&self) -> usize {
		match self.data.get(0) {
			Some(c) => c.len(),
			None => 0,
		}
	}
	
	pub fn get_slice(&self, range: Range<usize>) -> [&[f32]; N] {
		core::array::from_fn(|c| &self.data[c][range.clone()])
	}
}

impl AudioTrack<2> {
	pub fn get_slice_mut(&mut self, range: Range<usize>) -> [&mut [f32]; 2] {
		let data_mut = self.data.split_at_mut(1);
		[
			data_mut.0[0][range.clone()].as_mut(),
			data_mut.1[0][range].as_mut(),
		]
	}
}



