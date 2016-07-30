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
	pub length_in_48khz_samples :u64,
}

impl AudioMetadata for Metadata {
	fn get_output_channel_count(&self) -> u8 {
		self.output_channels
	}
	/// Returns the duration of the vorbis audio piece.
	fn get_duration(&self) -> Duration {
		Duration::from_millis(
				((self.length_in_48khz_samples as f64) / 48_000. * 1000.0)
			as u64)
	}
}

impl fmt::Debug for Metadata {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let duration_raw_secs = (self.length_in_48khz_samples as f64) / 48_000.;
		let duration_mins = f64::floor(duration_raw_secs / 60.);
		let duration_secs = duration_raw_secs % 60.;
		write!(f, "{} channels, with duration of {:02}:{:02.0} secs",
			self.output_channels, duration_mins, duration_secs)
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

