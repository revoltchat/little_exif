use std::fs::copy;
use std::fs::remove_file;
use std::path::Path;

extern crate little_exif;
use little_exif::metadata::Metadata;
use little_exif::exif_tag::ExifTag;

fn main() 
{
	let jpg_path = Path::new("./rsrc/A0579575.jpg");

	let metadata = Metadata::new_from_path(&jpg_path).unwrap();

	for tag in metadata.data()
	{
		println!("{:?}", tag);
	}
}
