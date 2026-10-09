//! The file system holds an image. sinteract reads its header, which says what
//! kind it is and how big, and the bytes go on to the drawing as a data URI.

use sinteract::asset::{data_uri, image_head};

pub fn load_bitmap(path: String) -> (f64, f64, String) {
    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error reading {path}: {err}");
            return (0.0, 0.0, String::new());
        }
    };
    match image_head(&data).and_then(|head| Ok((head, data_uri(&data)?))) {
        Ok((head, uri)) if head.width > 0 && head.height > 0 => {
            (f64::from(head.width), f64::from(head.height), uri)
        }
        Ok(_) => {
            eprintln!("Error: {path} has no pixels");
            (0.0, 0.0, String::new())
        }
        Err(err) => {
            eprintln!("Error reading {path}: {err}");
            (0.0, 0.0, String::new())
        }
    }
}
