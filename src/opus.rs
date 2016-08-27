// Ogg metadata reader written in Rust
//
// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

use std::io::Cursor;
use byteorder::{ReadBytesExt, LittleEndian};
use std::fmt;
use std::time::Duration;
use OggMetadataError;
use AudioMetadata;

/**
Metadata for the Opus audio codec.
*/
pub struct Metadata {
	pub output_channels :u8,
	/// The number of samples in this piece
	///
	/// While opus has a varying sample rate,
	/// the per-page sample counter operates on
	/// units of 48khz.
	pub length_in_48khz_samples :Option<u64>,
}

impl AudioMetadata for Metadata {
	fn get_output_channel_count(&self) -> u8 {
		self.output_channels
	}
	fn get_duration(&self) -> Option<Duration> {
		self.length_in_48khz_samples.map(|l|
			Duration::from_millis(
				((l as f64) / 48_000. * 1000.0)
			as u64)
		)
	}
}

impl fmt::Debug for Metadata {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.length_in_48khz_samples {
			Some(l) => {
				let duration_raw_secs = (l as f64) / 48_000.;
				write!(f, "{} channels, with duration of {}",
				self.output_channels, ::format_duration(duration_raw_secs))
			},
			None => write!(f, "{} channels", self.output_channels),
		}
	}
}

pub struct IdentHeader {
	pub output_channels :u8,
	pub pre_skip :u16,
}

pub fn read_header_ident(packet :&[u8]) -> Result<IdentHeader, OggMetadataError> {
	let mut rdr = Cursor::new(packet);
	let opus_version = try!(rdr.read_u8());
	// The version is internally separated into two halves:
	// The "major" and the "minor" half. We have to be backwards
	// compatible with any version where the major half is 0.
	if opus_version >= 16 {
		try!(Err(OggMetadataError::UnrecognizedFormat));
	}
	let output_channels = try!(rdr.read_u8());
	let pre_skip = try!(rdr.read_u16::<LittleEndian>());

	let hdr :IdentHeader = IdentHeader {
		output_channels : output_channels,
		pre_skip : pre_skip,
	};
	return Ok(hdr);
}

