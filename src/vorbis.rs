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
use crate::OggMetadataError;
use crate::AudioMetadata;

/**
Metadata for the Vorbis audio codec.
*/
pub struct Metadata {
	pub channels :u8,
	pub sample_rate :u32,
	pub length_in_samples :Option<u64>,
}

impl AudioMetadata for Metadata {
	fn get_output_channel_count(&self) -> u8 {
		self.channels
	}
	fn get_duration(&self) -> Option<Duration> {
		self.length_in_samples.map(|l|
			Duration::from_millis(
				((l as f64) / (self.sample_rate as f64) * 1000.0)
			as u64)
		)
	}
}

impl fmt::Debug for Metadata {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.length_in_samples {
			Some(l) => {
				let duration_raw_secs = (l as f64) / (self.sample_rate as f64);
				write!(f, "{} channels, with {} Hz sample rate and duration of {}",
					self.channels, self.sample_rate, crate::format_duration(duration_raw_secs))
			},
			None => write!(f, "{} channels, with {} Hz sample rate", self.channels, self.sample_rate),
		}
	}
}

pub struct IdentHeader {
	pub channels :u8,
	pub sample_rate :u32,
}

pub fn read_header_ident(packet :&[u8]) -> Result<IdentHeader, OggMetadataError> {
	let mut rdr = Cursor::new(packet);
	let vorbis_version = rdr.read_u32::<LittleEndian>()?;
	if vorbis_version != 0 {
		Err(OggMetadataError::UnrecognizedFormat)?;
	}
	let audio_channels = rdr.read_u8()?;
	let audio_sample_rate = rdr.read_u32::<LittleEndian>()?;

	let hdr :IdentHeader = IdentHeader {
		channels : audio_channels,
		sample_rate : audio_sample_rate,
	};
	Ok(hdr)
}
