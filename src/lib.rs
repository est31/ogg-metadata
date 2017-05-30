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
* Theora (Detect, Metadata)
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
mod theora;

use std::io;
use ogg::{OggReadError, PacketReader};
use std::time::Duration;

pub use vorbis::Metadata as VorbisMetadata;
pub use opus::Metadata as OpusMetadata;
pub use theora::Metadata as TheoraMetadata;

#[derive(Debug)]
pub enum OggFormat {
	/// The vorbis format ([spec](https://www.xiph.org/vorbis/doc/Vorbis_I_spec.html)).
	Vorbis(VorbisMetadata),
	/// The opus format, as specified by [RFC 6716](https://tools.ietf.org/html/rfc6716),
	/// and [RFC 7845](https://tools.ietf.org/html/rfc7845).
	Opus(OpusMetadata),
	/// The Theora video format ([spec](https://www.theora.org/doc/Theora.pdf)).
	Theora(TheoraMetadata),
	/// The speex format ([spec](http://www.speex.org/docs/manual/speex-manual/)).
	Speex,
	/// The skeleton format with structure information
	/// ([spec](https://wiki.xiph.org/Ogg_Skeleton_4))
	Skeleton,
	/// An format not supported by this crate or the magic code was corrupted.
	Unknown,
}

/// Bare (C-style enum) counterpart to OggFormat
#[derive(Debug, Copy, Clone, PartialEq)]
enum BareOggFormat {
	Vorbis,
	Opus,
	Theora,
	Speex,
	Skeleton,
}

pub trait AudioMetadata {
	fn get_output_channel_count(&self) -> u8;
	fn get_duration(&self) -> Option<Duration>;
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

fn seek_before_end<'a, T :io::Read + io::Seek + 'a>(pck_rdr :&mut PacketReader<T>,
		offs :u64) -> Result<u64, OggMetadataError> {
	use std::io::SeekFrom;
	let end_pos = try!(pck_rdr.seek_bytes(SeekFrom::End(0)));
	let end_pos_to_seek = ::std::cmp::min(end_pos, offs);
	return Ok(try!(pck_rdr.seek_bytes(SeekFrom::End(-(end_pos_to_seek as i64)))));
}

fn get_absgp_of_last_packet<'a, T :io::Read + io::Seek + 'a>(pck_rdr :&mut PacketReader<T>)
		-> Result<u64, OggMetadataError> {
	// 150 kb are enough so that we are guaranteed to find a
	// valid page inside them (unless there is unused space
	// between pages, but then there is no guaranteed limit).
	try!(seek_before_end(pck_rdr, 150 * 1024));
	let mut pck = try!(pck_rdr.read_packet_expected());
	// Now read until the last packet, and get its absgp
	while !pck.last_packet {
		pck = try!(pck_rdr.read_packet_expected());
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

	return Some(ret);
}

/// Returns whether the rich metadata has lenth information,
/// which requires retrieving of the absgp of the last packet.
fn needs_last_packet_absgp(bare_format :BareOggFormat) -> bool {
	match bare_format {
		BareOggFormat::Vorbis => true,
		BareOggFormat::Opus => true,
		BareOggFormat::Theora => false,
		BareOggFormat::Speex => false,
		BareOggFormat::Skeleton => false,
	}
}

fn parse_format(pck_data :&[u8], bare_format :BareOggFormat,
		last_packet_absgp :Option<u64>) -> Result<OggFormat, OggMetadataError> {
	use OggFormat::*;
	Ok(match bare_format {
		BareOggFormat::Vorbis => {
			let ident_hdr = try!(vorbis::read_header_ident(pck_data));
			Vorbis(VorbisMetadata {
				channels : ident_hdr.channels,
				sample_rate : ident_hdr.sample_rate,
				length_in_samples : last_packet_absgp,
			})
		},
		BareOggFormat::Opus => {
			let ident_hdr = try!(opus::read_header_ident(pck_data));
			Opus(OpusMetadata {
				output_channels : ident_hdr.output_channels,
				length_in_48khz_samples : last_packet_absgp.map(
					|l| l - (ident_hdr.pre_skip as u64)),
			})
		},
		BareOggFormat::Theora => {
			let ident_hdr = try!(theora::read_header_ident(pck_data));
			Theora(TheoraMetadata {
				pixels_width : ident_hdr.picture_region_width,
				pixels_height : ident_hdr.picture_region_height,
			})
		},
		BareOggFormat::Speex => Speex,
		BareOggFormat::Skeleton => Skeleton,
	})
}

/// Reads the format of the file.
///
/// The read process is optimized for detecting the format without
/// performing a full read of the file.
/// Instead it guesses when a file may contain more than just one
/// logical stream, and even then only scans as much as it needs.
///
/// This is not 100% correct, as the Ogg physical bitstream may
/// contain further logical bitstreams that contain further data,
/// but if we wanted to be 100% correct, we'd have to scan the
/// entire file. This may be okay for files stored on disk, but
/// if a file is behind a slow internet connection, users expect
/// playback to work even if only a small part is downloaded.
///
/// The approach taken works perfectly with ogg/vorbis and ogg/opus
/// files, as those only contain one logical bitstream.
pub fn read_format<'a, T :io::Read + io::Seek + 'a>(rdr :T)
		-> Result<Vec<OggFormat>, OggMetadataError> {
	let mut pck_rdr = PacketReader::new(rdr);
	let pck = try!(pck_rdr.read_packet_expected());

	let id = identify_packet_data_by_magic(&pck.data);
	let id_inner = match id { Some(v) => v,
		None => return Ok(vec![OggFormat::Unknown]) };

	let mut res = Vec::new();

	let simple_seek_to_end_is_needed = needs_last_packet_absgp(id_inner.1);

	let last_packet_absgp = if simple_seek_to_end_is_needed {
		Some(try!(get_absgp_of_last_packet(&mut pck_rdr)))
	} else {
		None
	};

	res.push(try!(parse_format(&pck.data[id_inner.0..],
		id_inner.1, last_packet_absgp)));

	if id_inner.1 == BareOggFormat::Skeleton {
		use std::collections::HashMap;

		// Loop until the skeleton stream ended
		// and record any opening streams.
		let mut streams = HashMap::new();
		loop {
			let pck_cur = try!(pck_rdr.read_packet_expected());

			if pck_cur.stream_serial == pck.stream_serial {
				/*
				// "fisbone\0"
				let fisbone_magic = [0x66, 0x69, 0x73, 0x62, 0x6f, 0x6e, 0x65, 0x00];
				// "index\0"
				let index_magic = [0x69, 0x6e, 0x64, 0x65, 0x78, 0x00];
				match () {
					() if pck_cur.data.starts_with(&fisbone_magic) => {
						println!("==> bone!");
					},
					() if pck_cur.data.starts_with(&index_magic) => {
						println!("==> index!");
					},
					_ => {},
				}
				*/
				if pck_cur.last_packet {
					break;
				}
			}

			if !pck_cur.first_packet {
				continue;
			}
			let id = identify_packet_data_by_magic(&pck_cur.data);
			let id_inner = match id { Some(v) => v, None => {
				res.push(OggFormat::Unknown);
				continue
			} };
			streams.insert(pck_cur.stream_serial, (id_inner, pck_cur));
		}

		// Now seek to right before the end to get the last packets of the content.
		// 200 kb are just a guessed number and they might be totally wrong.
		// Can this guess be improved??
		try!(seek_before_end(&mut pck_rdr, 200 * 1024));

		'pseudo_return: loop {
			// This pseudo_try is our local replacement for try,
			// so that we don't escalate if we encounter any
			// errors in reading the final absgp positions,
			// like end of file errors (which may occur with
			// partial files)
			macro_rules! pseudo_try {
				($expr:expr) => (match $expr {
					$crate::std::result::Result::Ok(val) => val,
					$crate::std::result::Result::Err(_) => break 'pseudo_return,
				})
			}
			// Now Loop until we have found the
			// last packets of all the streams,
			// recording the streams in the process.
			while !streams.is_empty() {
				// TODO don't use try! here, but something else
				// so that we are more tolerant if there is no more
				// packet to come.
				// Because we will reach the err case of this
				// try if we didn't seek early enough, and didn't
				// catch all end pages of the streams to be
				// still before us.
				// As this failure would be our fault due to our
				// seek distance guess, we should fail gracefully
				// and just pretend the stream does not exist.
				let pck_cur = pseudo_try!(pck_rdr.read_packet_expected());

				// We are only interested in last packets.
				if !pck_cur.last_packet {
					continue;
				}
				let stream = match streams.remove(&pck_cur.stream_serial) {
					Some(v) => v, None => continue };

				if (stream.0).1 == BareOggFormat::Skeleton {
					// Skeleton inside skeleton is invalid.
					try!(Err(OggMetadataError::UnrecognizedFormat));
				}
				let st = try!(parse_format(&(stream.1).data[(stream.0).0..],
					(stream.0).1, Some(pck_cur.absgp_page)));
				res.push(st);
			}
			break;
		}

		// Add all streams we couldn't find a last packet for.
		for (_,stream) in streams.iter() {
			let st = try!(parse_format(&(stream.1).data[(stream.0).0..],
				(stream.0).1, None));
			res.push(st);
		}
	}

	return Ok(res);
}

fn format_duration(duration_raw_secs :f64) -> String {
	let duration_mins = f64::floor(duration_raw_secs / 60.);
	let duration_secs = f64::floor(duration_raw_secs % 60.);
	let duration_secs_fractal = (duration_raw_secs % 1.) * 100.;
	return format!("{:02}:{:02}.{:02.0}",
		duration_mins, duration_secs, duration_secs_fractal);
}
