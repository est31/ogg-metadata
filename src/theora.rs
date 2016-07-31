// Ogg metadata reader written in Rust
//
// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

use std::io::Cursor;
use byteorder::{ReadBytesExt, BigEndian};
use std::fmt;
use OggMetadataError;

/**
Metadata for the Theora video codec.
*/
pub struct Metadata {
	pub pixels_width :u32,
	pub pixels_height :u32,
}

impl fmt::Debug for Metadata {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "size {}x{}", self.pixels_width, self.pixels_height)
	}
}

pub struct IdentHeader {
	pub picture_region_width :u32,
	pub picture_region_height :u32,
}

#[allow(unused_variables)]
pub fn read_header_ident(packet :&[u8]) -> Result<IdentHeader, OggMetadataError> {
	let mut rdr = Cursor::new(packet);
	// Major, minor and revision parts of the version
	let vmaj = try!(rdr.read_u8());
	let vmin = try!(rdr.read_u8());
	let vrev = try!(rdr.read_u8());

	// Width/height of the frame in macro blocks
	let fmbw = try!(rdr.read_u16::<BigEndian>());
	let fmbh = try!(rdr.read_u16::<BigEndian>());

	// Width of the picture region in pixels
	let picw = try!(rdr.read_uint::<BigEndian>(3)) as u32;
	// Height of the picture region in pixels
	let pich = try!(rdr.read_uint::<BigEndian>(3)) as u32;

	let hdr :IdentHeader = IdentHeader {
		picture_region_width : picw,
		picture_region_height : pich,
	};
	return Ok(hdr);
}

