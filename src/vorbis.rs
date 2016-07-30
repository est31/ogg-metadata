// Ogg metadata reader written in Rust
//
// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

use std::io::Cursor;
use byteorder::{ReadBytesExt, LittleEndian};
use std::time::Duration;
use std::fmt;
use OggMetadataError;
use AudioMetadata;

/**
Metadata for the Vorbis audio codec.
*/
pub struct Metadata {
	pub channels :u8,
	pub sample_rate :u32,
	pub length_in_samples :u64,
}

impl AudioMetadata for Metadata {
	fn get_output_channel_count(&self) -> u8 {
		self.channels
	}
	/// Returns the duration of the vorbis audio piece.
	fn get_duration(&self) -> Duration {
		Duration::from_millis(
				((self.length_in_samples as f64) / (self.sample_rate as f64) * 1000.0)
			as u64)
	}
}

impl fmt::Debug for Metadata {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let duration_raw_secs = (self.length_in_samples as f64) / (self.sample_rate as f64);
		let duration_mins = f64::floor(duration_raw_secs / 60.);
		let duration_secs = duration_raw_secs % 60.;
		write!(f, "{} channels, with {} Hz sample rate and duration of {:02}:{:02.0} secs",
			self.channels, self.sample_rate, duration_mins, duration_secs)
	}
}

pub struct IdentHeader {
	pub channels :u8,
	pub sample_rate :u32,
}

pub fn read_header_ident(packet :&[u8]) -> Result<IdentHeader, OggMetadataError> {
	let mut rdr = Cursor::new(packet);
	let vorbis_version = try!(rdr.read_u32::<LittleEndian>());
	if vorbis_version != 0 {
		try!(Err(OggMetadataError::UnrecognizedFormat));
	}
	let audio_channels = try!(rdr.read_u8());
	let audio_sample_rate = try!(rdr.read_u32::<LittleEndian>());

	let hdr :IdentHeader = IdentHeader {
		channels : audio_channels,
		sample_rate : audio_sample_rate,
	};
	return Ok(hdr);
}
