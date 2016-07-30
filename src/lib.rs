// Ogg metadata reader written in Rust
//
// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

#![deny(unsafe_code)]
#![cfg_attr(test, deny(warnings))]

/*!
Metadata parser library for various Ogg formats.

Supported formats:

* Vorbis (Detect, Metadata)
* Opus (Detect, Metadata)
* Theora (Detect)
* Speex (Detect)

Support will be extended in the future, especially for the Theora codec.
*/

extern crate byteorder;
extern crate ogg;

// Comment this out if you want stack traces for your errors, useful for debugging.
/*
macro_rules! try {
	($expr:expr) => (match $expr {
		$crate::std::result::Result::Ok(val) => val,
		$crate::std::result::Result::Err(err) => {
			panic!("Panic on Err turned on for debug reasons. Encountered Err: {:?}", err)
		}
	})
}
// */

mod vorbis;
mod opus;

use std::io;
use ogg::{OggReadError, PacketReader};
use std::time::Duration;

pub use vorbis::Metadata as VorbisMetadata;
pub use opus::Metadata as OpusMetadata;

#[derive(Debug)]
pub enum OggFormat {
	/// The vorbis format ([spec](https://www.xiph.org/vorbis/doc/Vorbis_I_spec.html)).
	Vorbis(VorbisMetadata),
	/// The opus format, as specified by [RFC 6716](https://tools.ietf.org/html/rfc6716),
	/// and [RFC 7845](https://tools.ietf.org/html/rfc7845).
	Opus(OpusMetadata),
	/// The Theora video format ([spec](https://www.theora.org/doc/Theora.pdf)).
	Theora,
	/// The speex format ([spec](http://www.speex.org/docs/manual/speex-manual/)).
	Speex,
}

/// Bare (C-style enum) counterpart to OggFormat
pub enum BareOggFormat {
	Vorbis,
	Opus,
	Theora,
	Speex,
	Skeleton,
}

pub trait AudioMetadata {
	fn get_output_channel_count(&self) -> u8;
	fn get_duration(&self) -> Duration;
}

#[derive(Debug)]
pub enum OggMetadataError {
	/// Bad format or not one recognized by this crate.
	UnrecognizedFormat,
	/// I/O error occured.
	ReadError(std::io::Error),
}

impl std::error::Error for OggMetadataError {
	fn description(&self) -> &str {
		use OggMetadataError::*;
		match self {
			&UnrecognizedFormat => "Unrecognized or invalid format",
			&ReadError(_) => "I/O error",
		}
	}

	fn cause(&self) -> Option<&std::error::Error> {
		match self {
			&OggMetadataError::ReadError(ref err) =>
				Some(err as &std::error::Error),
			_ => None
		}
	}
}

impl std::fmt::Display for OggMetadataError {
	fn fmt(&self, fmt :&mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
		write!(fmt, "{}", std::error::Error::description(self))
	}
}

impl From<std::io::Error> for OggMetadataError {
	fn from(err :std::io::Error) -> OggMetadataError {
		return OggMetadataError::ReadError(err);
	}
}

impl From<OggReadError> for OggMetadataError {
	fn from(err :OggReadError) -> OggMetadataError {
		return match err {
			OggReadError::ReadError(err) => OggMetadataError::ReadError(err),
			_ => OggMetadataError::UnrecognizedFormat,
		};
	}
}

fn get_absgp_of_last_packet<'a, T :io::Read + io::Seek + 'a>(pck_rdr :&mut PacketReader<T>)
		-> Result<u64, OggMetadataError> {
	use std::io::SeekFrom;
	let end_pos = try!(pck_rdr.seek_bytes(SeekFrom::End(0)));
	// 150 kb are enough so that we are guaranteed to find a
	// valid page inside them (unless there is unused space
	// between pages, but then there is no guaranteed limit).
	let end_pos_to_seek = ::std::cmp::min(end_pos, 150 * 1024);
	try!(pck_rdr.seek_bytes(SeekFrom::End(-(end_pos_to_seek as i64))));
	let mut pck = try!(pck_rdr.read_packet());
	// Now read until the last packet, and get its absgp
	while !pck.last_packet {
		pck = try!(pck_rdr.read_packet());
	}
	return Ok(pck.absgp_page);
}

fn identify_packet_data_by_magic(pck_data :&[u8]) -> Option<(usize, BareOggFormat)> {
	// Magic sequences.
	// https://www.xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-620004.2.1
	let vorbis_magic = &[0x01, 0x76, 0x6f, 0x72, 0x62, 0x69, 0x73];
	// https://tools.ietf.org/html/rfc7845#section-5.1
	let opus_magic = &[0x4f, 0x70, 0x75, 0x73, 0x48, 0x65, 0x61, 0x64];
	// https://www.theora.org/doc/Theora.pdf#section.6.2
	let theora_magic = &[0x80, 0x74, 0x68, 0x65, 0x6f, 0x72, 0x61];
	// http://www.speex.org/docs/manual/speex-manual/node8.html
	let speex_magic = &[0x53, 0x70, 0x65, 0x65, 0x78, 0x20, 0x20, 0x20];
	// https://wiki.xiph.org/Ogg_Skeleton_4#Ogg_Skeleton_version_4.0_Format_Specification
	let skeleton_magic = &[0x66, 105, 115, 104, 101, 97, 100, 0];

	if pck_data.len() < 1 {
		return None;
	}

	use BareOggFormat::*;
	let ret :(usize, BareOggFormat) = match pck_data[0] {
		0x01 if pck_data.starts_with(vorbis_magic) => (vorbis_magic.len(), Vorbis),
		0x4f if pck_data.starts_with(opus_magic) => (opus_magic.len(), Opus),
		0x80 if pck_data.starts_with(theora_magic) => (theora_magic.len(), Theora),
		0x53 if pck_data.starts_with(speex_magic) => (speex_magic.len(), Speex),
		0x66 if pck_data.starts_with(skeleton_magic) => (speex_magic.len(), Skeleton),

		_ => return None,
	};
}

/// Reads the format of the file.
pub fn read_format<'a, T :io::Read + io::Seek + 'a>(rdr :&mut T)
		-> Result<OggFormat, OggMetadataError> {
	let mut pck_rdr = PacketReader::new(rdr);
	let pck = try!(pck_rdr.read_packet());

	// TODO get skeletons working.

	let id = identify_packet_data_by_magic(&pck.data);
	let id_inner = match id { Some(v) => v, None =>
		try!(Err(OggMetadataError::UnrecognizedFormat)) };

	use OggFormat::*;
	let ret :OggFormat = match id_inner.1 {
		BareOggFormat::Vorbis => {
			let ident_hdr = try!(vorbis::read_header_ident(
				&pck.data[id_inner.0..]));
			let len = try!(get_absgp_of_last_packet(&mut pck_rdr));
			Vorbis(VorbisMetadata {
				channels : ident_hdr.channels,
				sample_rate : ident_hdr.sample_rate,
				length_in_samples : len,
			})
		},
		BareOggFormat::Opus => {
			let ident_hdr = try!(opus::read_header_ident(
				&pck.data[id_inner.0..]));
			let len = try!(get_absgp_of_last_packet(&mut pck_rdr));
			Opus(OpusMetadata {
				output_channels : ident_hdr.output_channels,
				length_in_48khz_samples : len - (ident_hdr.pre_skip as u64),
			})
		},
		BareOggFormat::Theora => Theora,
		BareOggFormat::Speex => Speex,

		_ => try!(Err(OggMetadataError::UnrecognizedFormat)),
	};

	return Ok(ret);
}
