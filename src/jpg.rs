// Copyright © 2024 Tobias J. Prisching <tobias.prisching@icloud.com> and CONTRIBUTORS
// See https://github.com/TechnikTobi/little_exif#license for licensing details

use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use crate::endian::Endian;
use crate::u8conversion::*;
use crate::general_file_io::*;

pub(crate) const JPG_SIGNATURE: [u8; 2] = [0xff, 0xd8];

const JPG_MARKER_PREFIX: u8  = 0xff;
const JPG_APP1_MARKER:   u16 = 0xffe1;



fn
encode_metadata_jpg
(
	exif_vec: &Vec<u8>
)
-> Vec<u8>
{
	// vector storing the data that will be returned
	let mut jpg_exif: Vec<u8> = Vec::new();

	// Compute the length of the exif data (includes the two bytes of the
	// actual length field)
	let length = 2u16 + (EXIF_HEADER.len() as u16) + (exif_vec.len() as u16);

	// Start with the APP1 marker and the length of the data
	// Then copy the previously encoded EXIF data 
	jpg_exif.extend(to_u8_vec_macro!(u16, &JPG_APP1_MARKER, &Endian::Big));
	jpg_exif.extend(to_u8_vec_macro!(u16, &length, &Endian::Big));
	jpg_exif.extend(EXIF_HEADER.iter());
	jpg_exif.extend(exif_vec.iter());

	return jpg_exif;
}



fn
check_signature
(
	file_buffer: &Vec<u8>
)
-> Result<(), std::io::Error>
{
	// Check the signature
	let signature_is_valid = file_buffer[0..2].iter()
		.zip(JPG_SIGNATURE.iter())
		.filter(|&(read, constant)| read == constant)
		.count() == JPG_SIGNATURE.len();

	if !signature_is_valid
	{
		return io_error!(InvalidData, "Can't open JPG file - Wrong signature!");
	}

	// Signature is valid - can proceed using as JPG file
	return Ok(());
}

fn
file_check_signature
(
	path: &Path
)
-> Result<File, std::io::Error>
{
	let mut file = open_read_file(path)?;
	
	// Check the signature
	let mut signature_buffer = [0u8; 2];
	file.read(&mut signature_buffer)?;
	check_signature(&signature_buffer.to_vec())?;

	// Signature is valid - can proceed using the file as JPG file
	return Ok(file);
}



pub(crate) fn
clear_metadata
(
	file_buffer: &mut Vec<u8>
)
-> Result<(), std::io::Error>
{
	check_signature(&file_buffer)?;

	// Setup of variables necessary for going through the file
	let mut buffer_iterator = file_buffer.iter();                               // Iterator for processing the bytes of the file
	let mut seek_counter = 0u64;                                                // A counter for keeping track of where in the file we currently are
	let mut byte_buffer = [0u8; 1];                                             // A buffer for reading in a byte of data from the file
	let mut previous_byte_was_marker_prefix = false;                            // A boolean for remembering if the previous byte was a marker prefix (0xFF)

	loop
	{
		// Read next byte into buffer
		if let Some(byte) = buffer_iterator.next() 
		{
			byte_buffer[0] = byte.clone();
		}

		if previous_byte_was_marker_prefix
		{
			match byte_buffer[0]
			{
				0xe1	=> {
					// APP1 marker

					// Read in the length of the segment
					// (which follows immediately after the marker)
					let mut length_buffer = [0u8; 2];

					if let (Some(&byte1), Some(&byte2)) = (buffer_iterator.next(), buffer_iterator.next()) 
					{
						length_buffer = [byte1, byte2];
					}

					// Decode the length to determine how much more data there is
					let length = from_u8_vec_macro!(u16, &length_buffer.to_vec(), &Endian::Big);
					let remaining_length = length - 2;

					// Skip the segment
					if remaining_length > 0 
					{
						if buffer_iterator.nth((remaining_length - 1) as usize).is_none()
						{
							panic!("Could not skip to end of APP1 segment!");
						}
					} 
					else 
					{
						unreachable!("If rem_len is <= 0 then it's not a valid\
						JPEG - it must have at least a single SOS after APP1")
					}

					// ...copy data from there onwards into a buffer...
					let mut file_buffer_clone = file_buffer.clone();
					let (_, buffer) = file_buffer_clone.split_at_mut(
						  (seek_counter     as usize)                           // Skip what has already been sought
						+ (remaining_length as usize)                           // Skip current segment
						+ 2                                                     // Skip Marker Prefix and APP1 marker
						+ 2                                                     // Skip the two length bytes
					);
					let buffer: Vec<u8> = buffer.to_vec();

					// This essentially shifts the right-most bytes n bytes to the left
					// This seeks inside the file_buffer to the position 
					// (seek_counter as usize), i.e. all bytes that have 
					// previously been read. 
					// Then a chunk of the length of the buffer vector is
					// selected and replaced with the buffer contents, shifting
					// the contents to the left
					file_buffer
						[(seek_counter as usize)..]
						[..buffer.len()]
						.copy_from_slice(&buffer);

					// Cut off right-most bytes that are now duplicates due 
					// to the previous shift-to-left operation
					let cutoff_index = (seek_counter as usize) + buffer.len();
					file_buffer.truncate(cutoff_index);

					// Reassign iterator to the new file buffer and seek to the
					// current position
					buffer_iterator = file_buffer.iter();
					buffer_iterator.nth(seek_counter as usize);

					// Account for the fact that we stepped back the prefix
					// marker and the marker itself (note the increment at the
					// end of the iteration, which is why we remove two as one
					// gets added back again there)
					seek_counter -= 2;
				},
				0xd9	=> break,                                               // EOI marker
				_		=> (),                                                  // Every other marker
			}

			previous_byte_was_marker_prefix = false;
		}
		else
		{
			previous_byte_was_marker_prefix = byte_buffer[0] == JPG_MARKER_PREFIX;
		}

		seek_counter += 1;

	}

	return Ok(());
}

pub(crate) fn
file_clear_metadata
(
	path: &Path
)
-> Result<(), std::io::Error>
{
	// Load the entire file into memory instead of reading one byte at a time
	// to improve the overall speed
	// Thanks to Xuf3r for this improvement!
	let mut file_buffer: Vec<u8> = std::fs::read(path)?;

	// Clear the metadata from the file buffer
	clear_metadata(&mut file_buffer)?;
	
	// Write the file
	// Possible to optimize further by returning the purged bytestream itself?
	let mut file = std::fs::OpenOptions::new().write(true).truncate(true).open(path)?;
	perform_file_action!(file.write_all(&file_buffer));

	return Ok(());
}

/// Provides the JPEG specific encoding result as vector of bytes to be used
/// by the user (e.g. in combination with another library)
pub(crate) fn
as_u8_vec
(
	general_encoded_metadata: &Vec<u8>
)
-> Vec<u8>
{
	encode_metadata_jpg(general_encoded_metadata)
}



pub(crate) fn
write_metadata
(
	file_buffer: &mut Vec<u8>,
	general_encoded_metadata: &Vec<u8>
)
-> Result<(), std::io::Error>
{
	// Remove old metadata
	clear_metadata(file_buffer)?;

	// Encode the data specifically for JPG
	let mut encoded_metadata = encode_metadata_jpg(general_encoded_metadata);

	// Insert the metadata right after the signature
	crate::util::insert_multiple_at(file_buffer, 2, &mut encoded_metadata);

	return Ok(());
}

/// Writes the given generally encoded metadata to the JP(E)G image file at 
/// the specified path. 
/// Note that any previously stored metadata under the APP1 marker gets removed
/// first before writing the "new" metadata. 
pub(crate) fn
file_write_metadata
(
	path: &Path,
	general_encoded_metadata: &Vec<u8>
)
-> Result<(), std::io::Error>
{
	// Load the entire file into memory instead of performing multiple read, 
	// seek and write operations
	let mut file = open_write_file(path)?;
	let mut file_buffer: Vec<u8> = Vec::new();
	perform_file_action!(file.read_to_end(&mut file_buffer));

	// Writes the metadata to the file_buffer vec
	// The called function handles the removal of old metadata and the JPG
	// specific encoding, so we pass only the generally encoded metadata here
	write_metadata(&mut file_buffer, general_encoded_metadata)?;

	// Seek back to start & write the file
	perform_file_action!(file.seek(SeekFrom::Start(0)));
	perform_file_action!(file.write_all(&file_buffer));

	return Ok(());
}

pub(crate) fn
read_metadata
(
	file_buffer: &Vec<u8>
)
-> Result<Vec<u8>, std::io::Error>
{
	check_signature(file_buffer)?;

	let mut cursor = Cursor::new(file_buffer);
	cursor.set_position(2);

	return generic_read_metadata(&mut cursor);
}

pub(crate) fn
file_read_metadata
(
	path: &Path
)
-> Result<Vec<u8>, std::io::Error>
{
	// Use a buffered reader to speed up operations, see issue #21
	let mut buffered_file = BufReader::new(file_check_signature(path)?);
	return generic_read_metadata(&mut buffered_file);
}

/// Skips the entropy-coded segment (ECS) that is followed by a start of scan
/// segment (SOS) and positions the cursor at the start of the next segment,
/// i.e. a 0xFF byte that is followed by a marker that is NOT 0xD0-0xD7 or 0x00.
/// Assumes that the given cursor is positioned at the start of the ECS
fn 
skip_ecs
<T: Seek + Read>
(
	cursor: &mut T
)
-> Result<(), std::io::Error>
{
	
	let mut byte_buffer = [0u8; 1];                                             // A buffer for reading in a byte of data from the file
	let mut previous_byte_was_marker_prefix = false;                            // A boolean for remembering if the previous byte was a marker prefix (0xFF)

	loop
	{
		// Read next byte into buffer
		cursor.read_exact(&mut byte_buffer)?;

		if previous_byte_was_marker_prefix
		{
			match byte_buffer[0]
			{
				0xd0 | 0xd1 | 0xd2 | 0xd3 | 0xd4 | 0xd5 | 0x6 | 0xd7 |
				0x00 => {
					// Continue
				},

				_ => {
					// Position back to where the 0xFF byte is located
					cursor.seek_relative(-2)?;
					return Ok(()); 
				},
			}

			previous_byte_was_marker_prefix = false;
		}
		else
		{
			previous_byte_was_marker_prefix = byte_buffer[0] == JPG_MARKER_PREFIX;
		}
	}
}


fn
generic_read_metadata
<T: Seek + Read>
(
	cursor: &mut T
)
-> Result<Vec<u8>, std::io::Error>
{
	// Setup of variables necessary for going through the data
	let mut byte_buffer = [0u8; 1];                                             // A buffer for reading in a byte of data from the file
	let mut previous_byte_was_marker_prefix = false;                            // A boolean for remembering if the previous byte was a marker prefix (0xFF)

	loop
	{
		// Read next byte into buffer
		cursor.read_exact(&mut byte_buffer)?;

		if previous_byte_was_marker_prefix
		{
			// Check if this is the end of the file. In that case, the length
			// data can't be read and we need to return prematurely. 
			// This is why this case can't be included in the match afterwards.
			if byte_buffer[0] == 0xd9                                           // EOI marker
			{
				// No more data to read in
				return io_error!(Other, "No EXIF data found!");
			}

			// Read in the length of the segment
			// (which follows immediately after the marker)
			let mut length_buffer = [0u8; 2];
			cursor.read_exact(&mut length_buffer)?;

			// Decode the length to determine how much more data there is
			let length = from_u8_vec_macro!(u16, &length_buffer.to_vec(), &Endian::Big);
			let remaining_length = (length - 2) as usize;

			match byte_buffer[0]
			{
				0xe1 => {                                                       // APP1 marker
					// Read in & return the remaining data
					let mut app1_buffer = vec![0u8; remaining_length];
					cursor.read_exact(&mut app1_buffer)?;

					return Ok(app1_buffer);
				},

				0xda => {                                                       // SOS marker
					// The start of scan (SOS) segment is followed by a blob of
					// image data, the entropy-coded segment (ECS), which has no
					// information regarding its length (as it may easily be 
					// bigger than the max segment length of 64kb)

					// So, we have to scan byte-for-byte at this point until
					// a marker prefix comes up that is NOT
					// - followed by a restart marker (D0 - D7) or 
					// - a data FF (followed by 00)

					// So, start by skipping the SOS segment
					cursor.seek_relative(remaining_length as i64)?;

					// And skip the ECS
					skip_ecs(cursor)?;
				}

				_ => {                                                          // Every other marker
					// Skip this segment
					cursor.seek_relative(remaining_length as i64)?;
				},
			}

			previous_byte_was_marker_prefix = false;
		}
		else
		{
			previous_byte_was_marker_prefix = byte_buffer[0] == JPG_MARKER_PREFIX;
		}
	}
}