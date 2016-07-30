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
* Opus (Detect)
* Theora (Detect)
* Speex (Detect)

Support will be extended in the future, especially for the theora and Opus codecs.
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

use std::io;
use ogg::{OggReadError, PacketReader};

pub use vorbis::Metadata as VorbisMetadata;

#[derive(Debug)]
pub enum OggFormat {
	/// The vorbis format ([spec](https://www.xiph.org/vorbis/doc/Vorbis_I_spec.html)).
	Vorbis(VorbisMetadata),
	/// The opus format, as specified by [RFC 6716](https://tools.ietf.org/html/rfc6716),
	/// and [RFC 7845](https://tools.ietf.org/html/rfc7845).
	Opus,
	/// The Theora video format ([spec](https://www.theora.org/doc/Theora.pdf)).
	Theora,
	/// The speex format ([spec](http://www.speex.org/docs/manual/speex-manual/)).
	Speex,
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

/// Reads the format of the file.
pub fn read_format<'a, T :io::Read + io::Seek + 'a>(rdr :&mut T)
		-> Result<OggFormat, OggMetadataError> {
	let mut pck_rdr = PacketReader::new(rdr);
	let pck = try!(pck_rdr.read_packet());
	// Magic sequences.
	// https://www.xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-620004.2.1
	let vorbis_magic = &[0x01, 0x76, 0x6f, 0x72, 0x62, 0x69, 0x73];
	// https://tools.ietf.org/html/rfc7845#section-5.1
	let opus_magic = &[0x4f, 0x70, 0x75, 0x73, 0x48, 0x65, 0x61, 0x64];
	// https://www.theora.org/doc/Theora.pdf#section.6.2
	let theora_magic = &[0x80, 0x74, 0x68, 0x65, 0x6f, 0x72, 0x61];
	// http://www.speex.org/docs/manual/speex-manual/node8.html
	let speex_magic = &[0x53, 0x70, 0x65, 0x65, 0x78, 0x20, 0x20, 0x20];

	if pck.data.len() < 1 {
		// TODO not a recognized format
		try!(Err(OggMetadataError::UnrecognizedFormat));
	}

	use OggFormat::*;
	let ret :OggFormat = match pck.data[0] {
		0x01 if pck.data.starts_with(vorbis_magic) => {
			let ident_hdr = try!(vorbis::read_header_ident(
				&pck.data[vorbis_magic.len()..]));
			let len = try!(get_absgp_of_last_packet(&mut pck_rdr));
			Vorbis(VorbisMetadata {
				channels : ident_hdr.channels,
				sample_rate : ident_hdr.sample_rate,
				length_in_samples : len,
			})
		},
		0x4f if pck.data.starts_with(opus_magic) => Opus,
		0x80 if pck.data.starts_with(theora_magic) => Theora,
		0x53 if pck.data.starts_with(speex_magic) => Speex,

		_ => try!(Err(OggMetadataError::UnrecognizedFormat)),
	};

	return Ok(ret);
}
