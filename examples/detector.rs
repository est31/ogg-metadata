// Ogg metadata reader written in Rust
//
// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

extern crate ogg_metadata;

use ogg_metadata::{OggMetadataError, read_format};

use std::env;
use std::fs::File;

fn main() {
	match run() {
		Ok(_) =>(),
		Err(err) => println!("Error: {}", err),
	}
}

fn run() -> Result<(), OggMetadataError> {
	let file_path = env::args().nth(1).expect("No arg found. Please specify a file to open.");
	println!("Opening file: {}", file_path);
	let mut f = try!(File::open(file_path));
	let format = read_format(&mut f);
	println!("Format of the file is: {:?}", format);
	return Ok(());
}
