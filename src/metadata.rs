// Copyright © 2024 Tobias J. Prisching <tobias.prisching@icloud.com> and CONTRIBUTORS
// See https://github.com/TechnikTobi/little_exif#license for licensing details

use std::path::Path;

use crate::endian::*;
use crate::exif_tag::ExifTag;
use crate::exif_tag::ExifTagGroup;
use crate::exif_tag_format::ExifTagFormat;
use crate::exif_tag_format::INT16U;
use crate::filetype::get_file_type;
use crate::filetype::FileExtension;
use crate::general_file_io::*;
use crate::jxl;
use crate::u8conversion::*;

use crate::jpg;
use crate::png;
use crate::webp;

const IFD_ENTRY_LENGTH: u32 = 12;
const IFD_END: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

#[derive(Clone)]
pub struct Metadata {
    data: Vec<ExifTag>,
    endian: Endian,
}

impl Metadata {
    /// Constructs a new, empty `Metadata` object.
    ///
    /// This uses little endian notation by default.
    ///
    /// # Examples
    /// ```no_run
    /// use little_exif::metadata::Metadata;
    ///
    /// let mut metadata: Metadata = Metadata::new();
    /// ```
    pub fn new() -> Metadata {
        Metadata {
            endian: Endian::Little,
            data: Vec::new(),
        }
    }

    fn general_decoding_wrapper(
        raw_pre_decode_general: Result<Vec<u8>, std::io::Error>,
    ) -> Result<Metadata, std::io::Error> {
        if let Ok(pre_decode_general) = raw_pre_decode_general {
            let decoding_result = Self::decode_metadata_general(&pre_decode_general);
            if let Ok((endian, data)) = decoding_result {
                return Ok(Metadata { endian, data });
            } else {
                eprintln!("{}", decoding_result.err().unwrap());
            }
        } else {
            eprintln!(
                "Error during decoding: {:?}",
                raw_pre_decode_general.err().unwrap()
            );
        }

        eprintln!("WARNING: Can't read metadata - Create new & empty struct");
        return Ok(Metadata::new());
    }

    /// Constructs a new `Metadata` object with the metadata from an image that is stored as a `Vec<u8>`
    /// - If unable to handle the file vector (e.g. unsupported file type, etc.), this (currently) panics.
    /// - If unable to decode the metadata, a new, empty object gets created and returned.
    /// # Examples
    /// ```no_run
    /// use std::fs;
    /// use little_exif::metadata::Metadata;
    /// use little_exif::filetype::FileExtension;
    ///
    /// let file_data = fs::read("image.jpg").unwrap();
    /// let mut metadata: Metadata = Metadata::new_from_vec(&file_data, FileExtension::JPEG).unwrap();
    /// ```
    #[allow(unreachable_patterns)]
    pub fn new_from_vec(
        file_buffer: &Vec<u8>,
        file_type: FileExtension,
    ) -> Result<Metadata, std::io::Error> {
        let raw_pre_decode_general = match file_type {
            FileExtension::JPEG => jpg::read_metadata(file_buffer),
            FileExtension::JXL => jxl::read_metadata(file_buffer),
            FileExtension::PNG { as_zTXt_chunk: _ } => png::vec::read_metadata(file_buffer),
            FileExtension::WEBP => webp::vec::read_metadata(file_buffer),
            _ => {
                return io_error!(
                    Other,
                    format!(
                        "Function 'new_from_vec' not yet implemented for {:?}",
                        file_type
                    )
                )
            }
        };

        return Self::general_decoding_wrapper(raw_pre_decode_general);
    }

    /// Constructs a new `Metadata` object with the metadata from the image at the specified path.
    /// - If unable to read the file (e.g. does not exist, unsupported file type, etc.), this (currently) panics.
    /// - If unable to decode the metadata, a new, empty object gets created and returned.
    ///
    /// # Examples
    /// ```no_run
    /// use little_exif::metadata::Metadata;
    ///
    /// let mut metadata: Metadata = Metadata::new_from_path(std::path::Path::new("image.png")).unwrap();
    /// ```
    #[allow(unreachable_patterns)]
    pub fn new_from_path(path: &Path) -> Result<Metadata, std::io::Error> {
        let file_type = get_file_type(path)?;

        // Call the file specific decoders as a starting point for obtaining
        // the raw EXIF data that gets further processed
        let raw_pre_decode_general = match file_type {
            FileExtension::JPEG => jpg::file_read_metadata(&path),
            FileExtension::JXL => jxl::file_read_metadata(&path),
            FileExtension::PNG { as_zTXt_chunk: _ } => png::file::read_metadata(&path),
            FileExtension::WEBP => webp::file::read_metadata(&path),
            _ => {
                return io_error!(
                    Other,
                    format!(
                        "Function 'new_from_path' not yet implemented for {:?}",
                        file_type
                    )
                )
            }
        };

        return Self::general_decoding_wrapper(raw_pre_decode_general);
    }

    /// Constructs a new `Metadata` object with the metadata from the image at the specified path.
    /// - If unable to read the file (e.g. does not exist, unsupported file type, etc.), this (currently) panics.
    /// - If unable to decode the metadata, a new, empty object gets created and returned.
    #[allow(unreachable_patterns)]
    pub fn new_from_path_with_filetype(
        path: &Path,
        file_type: FileExtension,
    ) -> Result<Metadata, std::io::Error> {
        // Call the file specific decoders as a starting point for obtaining
        // the raw EXIF data that gets further processed
        let raw_pre_decode_general = match file_type {
            FileExtension::JPEG => jpg::file_read_metadata(&path),
            FileExtension::JXL => jxl::file_read_metadata(&path),
            FileExtension::PNG { as_zTXt_chunk: _ } => png::file::read_metadata(&path),
            FileExtension::WEBP => webp::file::read_metadata(&path),
            _ => {
                return io_error!(
                    Other,
                    format!(
                        "Function 'new_from_path' not yet implemented for {:?}",
                        file_type
                    )
                )
            }
        };

        return Self::general_decoding_wrapper(raw_pre_decode_general);
    }

    /// Gets a shared reference to the list of all tags currently stored in the object.
    ///
    /// # Examples
    /// ```no_run
    /// use little_exif::metadata::Metadata;
    ///
    /// let metadata = Metadata::new_from_path(std::path::Path::new("image.png")).unwrap();
    /// for tag in metadata.data()
    /// {
    ///     // do something with the tags
    /// }
    /// ```
    pub fn data(&self) -> &Vec<ExifTag> {
        &self.data
    }

    /// Gets the endianness of the metadata
    ///
    /// # Examples
    /// ```no_run
    /// use little_exif::metadata::Metadata;
    ///
    /// let metadata = Metadata::new_from_path(std::path::Path::new("image.png")).unwrap();
    /// let tag_data = metadata.get_tag_by_hex(0x010e).unwrap().value_as_u8_vec(metadata.get_endian());
    /// ```
    pub fn get_endian(&self) -> &Endian {
        &self.endian
    }

    /// Gets the stored tag in the metadata for the given tag.
    /// Returns `None` if the tag is not present in the metadata struct.
    ///
    /// # Examples
    /// ```no_run
    /// use little_exif::metadata::Metadata;
    /// use little_exif::exif_tag::ExifTag;
    ///
    /// let metadata = Metadata::new_from_path(std::path::Path::new("image.png")).unwrap();
    /// let tag = metadata.get_tag(&ExifTag::ImageDescription(String::new()));
    /// ```
    pub fn get_tag(&self, input_tag: &ExifTag) -> Option<&ExifTag> {
        self.get_tag_by_hex(input_tag.as_u16())
    }

    /// Gets the sorted tag in the metadata by its hex value.
    /// Returns `None`if the tag is not present in the metadata struct.
    ///
    /// # Examples
    /// ```no_run
    /// // Note that the tag identifier of course does not need to be written in hex format
    /// // Hex notation only used in this example for more clarity
    /// use little_exif::metadata::Metadata;
    ///
    /// let metadata = Metadata::new_from_path(std::path::Path::new("image.png")).unwrap();
    /// let tag = metadata.get_tag_by_hex(0x010e);
    /// ```
    pub fn get_tag_by_hex(&self, input_tag_hex: u16) -> Option<&ExifTag> {
        for tag in &self.data {
            if tag.as_u16() == input_tag_hex {
                return Some(tag);
            }
        }
        return None;
    }

    /// Sets the tag in the metadata struct. If the tag is already in there it gets replaced
    ///
    /// # Examples
    /// ```no_run
    /// use little_exif::metadata::Metadata;
    /// use little_exif::exif_tag::ExifTag;
    ///
    /// let mut metadata = Metadata::new();
    /// metadata.set_tag(
    ///     ExifTag::ISO(vec![1234])
    /// );
    /// ```
    pub fn set_tag(&mut self, input_tag: ExifTag) {
        self.data.retain(|tag| tag.as_u16() != input_tag.as_u16());
        self.data.push(input_tag);

        // Sort the tags by the IFD they will go into the file later on
        self.data.sort_by(|a, b| {
            if a.get_group() == b.get_group() {
                // Same group, but unknown should go last
                if a.is_unknown() == b.is_unknown() {
                    std::cmp::Ordering::Equal
                } else if !a.is_unknown() && b.is_unknown() {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            } else {
                if a.get_group() < b.get_group()
                // e.g. IFD0 < ExifIFD
                {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            }
        });
    }

    /// Converts the metadata into a file specific vector of bytes
    /// Only to be used in combination with some other library/code that is
    /// able to handle the specific file type.
    /// Simply writing this to a file often is not enough, e.g. with WebP you
    /// have to determine where to write this, update the file size information
    /// and so on - check file type specific implementations or documentation
    /// for further details
    #[allow(unreachable_patterns)]
    pub fn as_u8_vec(&self, for_file_type: FileExtension) -> Vec<u8> {
        let general_encoded_metadata = self.encode_metadata_general();

        match for_file_type {
            FileExtension::PNG { as_zTXt_chunk } => {
                png::as_u8_vec(&general_encoded_metadata, as_zTXt_chunk)
            }
            FileExtension::JPEG => jpg::as_u8_vec(&general_encoded_metadata),
            FileExtension::WEBP => webp::as_u8_vec(&general_encoded_metadata),
            _ => Vec::new(),
        }
    }

    #[allow(unreachable_patterns)]
    pub fn clear_metadata(
        file_buffer: &mut Vec<u8>,
        file_type: FileExtension,
    ) -> Result<(), std::io::Error> {
        match file_type {
            FileExtension::JPEG => jpg::clear_metadata(file_buffer),
            FileExtension::JXL => jxl::clear_metadata(file_buffer),
            FileExtension::PNG { as_zTXt_chunk: _ } => png::vec::clear_metadata(file_buffer),
            FileExtension::WEBP => webp::vec::clear_metadata(file_buffer),
            _ => {
                return io_error!(
                    Other,
                    format!(
                        "Function 'clear_metadata' not yet implemented for {:?}",
                        file_type
                    )
                )
            }
        }
    }

    #[allow(unreachable_patterns)]
    pub fn file_clear_metadata(path: &Path) -> Result<(), std::io::Error> {
        let file_type = get_file_type(path)?;

        match file_type {
            FileExtension::JPEG => jpg::file_clear_metadata(&path),
            FileExtension::JXL => jxl::file_clear_metadata(&path),
            FileExtension::PNG { as_zTXt_chunk: _ } => png::file::clear_metadata(&path),
            FileExtension::WEBP => webp::file::clear_metadata(&path),
            _ => {
                return io_error!(
                    Other,
                    format!(
                        "Function 'file_clear_metadata' not yet implemented for {:?}",
                        file_type
                    )
                )
            }
        }
    }

    /// Writes the metadata to an image stored as a Vec<u8>
    /// For now, this only works for JPGs
    #[allow(unreachable_patterns)]
    pub fn write_to_vec(
        &self,
        file_buffer: &mut Vec<u8>,
        file_type: FileExtension,
    ) -> Result<(), std::io::Error> {
        match file_type {
            FileExtension::JPEG => {
                jpg::write_metadata(file_buffer, &self.encode_metadata_general())
            }
            FileExtension::JXL => jxl::write_metadata(file_buffer, &self.encode_metadata_general()),
            FileExtension::PNG { as_zTXt_chunk: _ } => {
                png::vec::write_metadata(file_buffer, &self.encode_metadata_general())
            }
            FileExtension::WEBP => {
                webp::vec::write_metadata(file_buffer, &self.encode_metadata_general())
            }
            _ => {
                return io_error!(
                    Other,
                    format!(
                        "Function 'file_clear_metadata' not yet implemented for {:?}",
                        file_type
                    )
                )
            }
        }
    }

    /// Writes the metadata to the specified file.
    /// This could return an error for multiple reasons:
    /// - The file does not exist at the given path
    /// - Interpreting the given path fails
    /// - The file type is not supported
    #[allow(unreachable_patterns)]
    pub fn write_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let file_type = get_file_type(path)?;

        match file_type {
            FileExtension::JPEG => jpg::file_write_metadata(&path, &self.encode_metadata_general()),
            FileExtension::JXL => jxl::file_write_metadata(&path, &self.encode_metadata_general()),
            FileExtension::PNG { as_zTXt_chunk: _ } => {
                png::file::write_metadata(&path, &self.encode_metadata_general())
            }
            FileExtension::WEBP => {
                webp::file::write_metadata(&path, &self.encode_metadata_general())
            }
            _ => {
                return io_error!(
                    Other,
                    format!(
                        "Function 'file_clear_metadata' not yet implemented for {:?}",
                        file_type
                    )
                )
            }
        }
    }

    fn decode_metadata_general(
        encoded_data: &Vec<u8>,
    ) -> Result<(Endian, Vec<ExifTag>), std::io::Error> {
        // Ensure that we have enough data
        if encoded_data.len() < (EXIF_HEADER.len() + Endian::Big.header().len() + 2 + IFD_END.len())
        {
            return io_error!(Other, "Not enough data for encoding!");
        }

        // Validate EXIF header
        for i in 0..EXIF_HEADER.len() {
            if encoded_data[i] != EXIF_HEADER[i] {
                return io_error!(Other, "Could not validate EXIF header!");
            }
        }

        // Determine endian
        let endian;
        if encoded_data[6] == 0x49 && encoded_data[7] == 0x49
        // "II"
        {
            endian = Endian::Little;
        } else if encoded_data[6] == 0x4d && encoded_data[7] == 0x4d
        // "MM"
        {
            endian = Endian::Big;
        } else {
            return io_error!(Other, "Illegal endian information!");
        }

        // Decode all the tags
        let mut all_tags = Vec::new();

        // Get offset to first IFD
        let ifd0_offset = from_u8_vec_macro!(u32, &encoded_data[10..14].to_vec(), &endian);

        // Start with IFD0
        let ifd0_decode_result = Self::decode_ifd(
            &encoded_data[6..].to_vec(),
            &ExifTagGroup::IFD0,
            ifd0_offset as usize,
            &endian,
        );

        if let Ok(ifd0_and_subifd_tags) = ifd0_decode_result {
            all_tags.extend(ifd0_and_subifd_tags);
        } else {
            return io_error!(
                Other,
                format!(
                    "Could not get IFD0 tags:\n {}",
                    ifd0_decode_result.err().unwrap()
                )
            );
        }

        return Ok((endian, all_tags));
    }

    fn decode_ifd(
        encoded_data: &Vec<u8>,
        group: &ExifTagGroup,
        ifd_start: usize,
        endian: &Endian,
    ) -> Result<Vec<ExifTag>, std::io::Error> {
        // Return an empty vector if there is not enough data to decode an IFD
        if encoded_data.len() <= 8 {
            return Ok(Vec::new());
        }

        // The first two bytes give us the number of entries in this IFD
        let number_of_entries = from_u8_vec_macro!(
            u16,
            &encoded_data[ifd_start..ifd_start + 2].to_vec(),
            endian
        );

        // Assert that we have enough data to unpack
        assert!(
            2 + IFD_ENTRY_LENGTH as usize * number_of_entries as usize + IFD_END.len()
                <= encoded_data.len() - ifd_start
        );

        let mut tags: Vec<ExifTag> = Vec::new();
        for i in 0..number_of_entries {
            // index within the given data where the current entry starts
            let entry_start_index = ifd_start + (2 + (i as u32) * IFD_ENTRY_LENGTH) as usize;

            // Decode the first 8 bytes with the tag, format and component number
            let hex_tag = from_u8_vec_macro!(
                u16,
                &encoded_data[(entry_start_index)..(entry_start_index + 2)].to_vec(),
                endian
            );
            let hex_format = from_u8_vec_macro!(
                u16,
                &encoded_data[(entry_start_index + 2)..(entry_start_index + 4)].to_vec(),
                endian
            );
            let hex_component_number = from_u8_vec_macro!(
                u32,
                &encoded_data[(entry_start_index + 4)..(entry_start_index + 8)].to_vec(),
                endian
            );

            // Decoding the format
            let format;
            if let Some(decoded_format) = ExifTagFormat::from_u16(hex_format) {
                format = decoded_format;
            } else {
                return io_error!(Other, format!("Illegal format value: {}", hex_format));
            }

            // Calculating the number of required bytes to determine if next
            // 4 bytes are data or an offset to data
            // Note: It is expected that the format here is "correct" in the
            // sense that it tells us whether or not an offset is used for the
            // data even if the given format in the image file is not the
            // right/default one for the currently processed tag according to
            // the exif specification.
            let byte_count = format.bytes_per_component() * hex_component_number;

            let raw_data;
            if byte_count > 4 {
                // Compute the offset
                let hex_offset = from_u8_vec_macro!(
                    u32,
                    &encoded_data[(entry_start_index + 8)..(entry_start_index + 12)].to_vec(),
                    endian
                );
                raw_data = encoded_data
                    [(hex_offset as usize)..((hex_offset + byte_count) as usize)]
                    .to_vec();
            } else {
                // The 4 bytes are the actual data
                // Note: This may actually be *less* than 4 bytes! This is why
                // The second index isn't just entry_start_index+12
                raw_data = encoded_data
                    [(entry_start_index + 8)..(entry_start_index + 8 + byte_count as usize)]
                    .to_vec();
            }

            // If this is a known tag...
            if let Ok(tag) = ExifTag::from_u16(hex_tag) {
                // ...for a SubIFD...
                if let Some(subifd_group) = tag.is_offset_tag() {
                    // ...perform a recursive call
                    let offset = from_u8_vec_macro!(u32, &raw_data, endian) as usize;

                    let subifd_decode_result =
                        Self::decode_ifd(&encoded_data, &subifd_group, offset, endian);

                    if let Ok(subifd_result) = subifd_decode_result {
                        tags.extend(subifd_result);
                        continue;
                    } else {
                        return io_error!(
                            Other,
                            format!(
                                "Could not decode SubIFD:\n  {}",
                                subifd_decode_result.err().unwrap()
                            )
                        );
                    }
                }
            }

            // At this point we have established that the tag is *not* a
            // SubIFD offset Tag like e.g. GPSInfo
            // But: The tag
            // - may be unknown
            // - may require conversion, e.g. INT16U -> INT32U

            // Check if the tag is known and compatible with the given format
            // Return error if incompatible and not a special case
            // Use one of the unknown tags if unknown
            if let Ok(tag) = ExifTag::from_u16(hex_tag) {
                if tag.format().as_u16() != format.as_u16() {
                    // The expected format and the given format in the file
                    // do *not* match. Check special cases (INT16U -> INT32U)
                    // If no special cases match, return an error
                    if tag.format() == ExifTagFormat::INT32U && format == ExifTagFormat::INT16U {
                        let int16u_data =
                            <INT16U as U8conversion<INT16U>>::from_u8_vec(&raw_data, endian);
                        let int32u_data = int16u_data
                            .into_iter()
                            .map(|x| x as u32)
                            .collect::<Vec<u32>>();
                        tags.push(tag.set_value_to_int32u_vec(int32u_data).unwrap());
                        continue;
                    }
                    // Other special cases
                    else {
                        return io_error!(
                            Other,
                            format!(
                                "Illegal format for known tag! Tag: {:?} Expected: {:?} Got: {:?}",
                                tag,
                                tag.format(),
                                format
                            )
                        );
                    }
                }
            }

            tags.push(
                ExifTag::from_u16_with_data(hex_tag, &format, &raw_data, &endian, group).unwrap(),
            );
        }

        return Ok(tags);
    }

    fn encode_ifd(
        &self,                       // The metadata struct, containing the tags
        group: ExifTagGroup, // The group the specific tags need to belong to (e.g. IFD0, ExifIFD, ...)
        given_offset: u32,   // How much offset already exists
        next_ifd_link: &[u8; 4], // A link to the next IFD (e.g. IFD1 for IFD0) or 4 bytes of 0x00 to signal "no next IFD"
        subifd_tag: Option<ExifTag>, // An optional ExifTag signaling that a SubIFD will follow
    ) -> Option<(u32, Vec<u8>)> {
        // Start Interop IFD with number of entries
        // If there are none, return None
        let mut ifd_vec: Vec<u8> = Vec::new();
        let mut count_entries = subifd_tag.is_some() as u16;
        for tag in &self.data {
            if tag.is_writable() && tag.get_group() == group {
                count_entries += 1;
            }
        }

        if count_entries == 0 {
            return None;
        }

        // Start by adding the number of entries
        ifd_vec.extend(to_u8_vec_macro!(u16, &count_entries, &self.endian).iter());
        assert_eq!(ifd_vec.len(), 2);

        // Compute first offset value and provide offset area in case its needed
        let mut next_offset: u32 = 0 as u32
            + given_offset as u32
            + ifd_vec.len() as u32
            + IFD_ENTRY_LENGTH * count_entries as u32
            + next_ifd_link.len() as u32;
        let mut ifd_offset_area: Vec<u8> = Vec::new();

        // Write directory entries to the vector
        for tag in &self.data {
            // Skip tags that can't be written or don't belong to the group
            if !tag.is_writable() || tag.get_group() != group {
                continue;
            }

            let value = tag.value_as_u8_vec(&self.endian);

            // Add Tag & Data Format /                                          2 + 2 bytes
            ifd_vec.extend(to_u8_vec_macro!(u16, &tag.as_u16(), &self.endian).iter());
            ifd_vec.extend(to_u8_vec_macro!(u16, &tag.format().as_u16(), &self.endian).iter());

            // Add number of components /                                       4 bytes
            let number_of_components: u32 = tag.number_of_components();
            ifd_vec.extend(to_u8_vec_macro!(u32, &number_of_components, &self.endian).iter());

            // Optional string padding (i.e. string is shorter than it should be)
            let mut string_padding: Vec<u8> = Vec::new();
            if tag.is_string() {
                for _ in 0..(number_of_components - value.len() as u32) {
                    string_padding.push(0x00);
                }
            }

            // Add offset or value /                                            4 bytes
            // Depending on the amount of data, either put it directly into
            // next 4 bytes or write an offset where the data can be found
            let byte_count: u32 = number_of_components * tag.format().bytes_per_component();
            if byte_count > 4 {
                ifd_vec.extend(to_u8_vec_macro!(u32, &next_offset, &self.endian).iter());
                ifd_offset_area.extend(value.iter());
                ifd_offset_area.extend(string_padding.iter());

                next_offset += byte_count;
            } else {
                let pre_length = ifd_vec.len();

                ifd_vec.extend(value.iter());
                ifd_vec.extend(string_padding.iter());

                let post_length = ifd_vec.len();

                // Make sure that this area is indeed *exactly* 4 bytes long
                for _ in 0..(4 - (post_length - pre_length)) {
                    ifd_vec.push(0x00);
                }
            }
        }

        // In case we have to write a SubIFD (e.g. ExifIFD) next
        // Do NOT mix this up with link to next IFD (like e.g. IFD1)
        if let Some(tag) = subifd_tag {
            // Write the offset tag & data format /                             2 + 2 bytes
            ifd_vec.extend(to_u8_vec_macro!(u16, &tag.as_u16(), &self.endian).iter());
            ifd_vec.extend(to_u8_vec_macro!(u16, &tag.format().as_u16(), &self.endian).iter());

            // Add number of components /                                       4 bytes
            ifd_vec.extend(to_u8_vec_macro!(u32, &tag.number_of_components(), &self.endian).iter());

            // Add the offset /                                                 4 bytes
            // We assume (know) that this is one component which has exactly
            // 4 bytes, thus fitting perfectly into the directory entry
            ifd_vec.extend(to_u8_vec_macro!(u32, &next_offset, &self.endian).iter());
        }

        // Write link and offset data
        ifd_vec.extend(next_ifd_link.iter());
        ifd_vec.extend(ifd_offset_area.iter());

        // Return next_offset as well to where to start with the offset
        // in the subordinate IFDs
        return Some((next_offset, ifd_vec));
    }

    #[allow(unused_assignments)]
    fn encode_metadata_general(&self) -> Vec<u8> {
        // Start construction with TIFF header
        let mut exif_vec: Vec<u8> = Vec::from(self.endian.header());
        let mut current_offset: u32 = 8;

        // IFD0
        if let Some((offset_post_ifd0, ifd0_data)) = self.encode_ifd(
            ExifTagGroup::IFD0,
            current_offset,            // For the TIFF header
            &[0x00, 0x00, 0x00, 0x00], // For now no link to IFD1
            Some(ExifTag::ExifOffset(vec![0])),
        ) {
            current_offset = offset_post_ifd0;
            exif_vec.extend(ifd0_data.iter());
        }

        // ExifIFD
        if let Some((offset_post_exififd, exififd_data)) = self.encode_ifd(
            ExifTagGroup::ExifIFD,
            current_offset, // Don't need +8 as already accounted for in this value due to previous function call
            &[0x00, 0x00, 0x00, 0x00],
            Some(ExifTag::InteropOffset(vec![0])),
        ) {
            current_offset = offset_post_exififd;
            exif_vec.extend(exififd_data.iter());
        }

        // InteropIFD
        if let Some((offset_post_interopifd, interopifd_data)) = self.encode_ifd(
            ExifTagGroup::InteropIFD,
            current_offset, // Don't need +8 as already accounted for in this value due to previous function call
            &[0x00, 0x00, 0x00, 0x00],
            None,
        ) {
            current_offset = offset_post_interopifd;
            exif_vec.extend(interopifd_data.iter());
        }

        // Other directories here... (someday)

        return exif_vec;
    }
}
