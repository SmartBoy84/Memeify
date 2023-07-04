use imageinfo::ImageInfo; // ugh, file extensions are not a given apparently
use scraper::{Html, Selector};
use std::fs::File;
use std::io::{Read, Write};
use ureq;

fn get_file(url: &str) -> String {
    match ureq::get(url).call() {
        Ok(data) => data.into_string().unwrap_or_default(),
        Err(err) => {
            println!("[WARNING] failed to download web page {:?}!", url);
            err.to_string()
        }
    }
}

fn main() {
    println!("One sec!");

    let site_url = "https://gbatemp.net";
    let max_image_size: u64 = 10_000_000; // bytes => 10mb

    let body: String = get_file(site_url);

    let fragment = Html::parse_fragment(body.as_str());
    let elements: Vec<_> = fragment
        .select(&Selector::parse(".memethumb").unwrap())
        .filter_map(|e| match e.value().attr("href") {
            Some(url) => {
                if !url.contains("attachments") {
                    println!("Malformed URL in memebox element: {:?}", url);
                    return None;
                }

                let image_url = format!("{site_url}/{url}");
                // println!("Downloading image: {image_url}");

                match ureq::get(&image_url).call() {
                    Ok(resp) => {

                        let expected_read_size: usize = match resp.header("Content-Length") {
                            Some(len_header) => match len_header.to_string().parse() {
                                Ok(length) => {
                                    if length > max_image_size as usize {
                                        println!("Why is the image {length} bytes?!");
                                        return None;
                                    } else {
                                        length
                                    }
                                }
                                Err(err) => {
                                    println!("Failed to parse Content-Length value: {err}");
                                    return None;
                                }
                            },
                            None => {
                                // println!("[Warning] Content-Length header not found!");
                                // return None; // this isn't for the match, rather the outer function
                                0 // don't fail if not found since Gbatemp doesn't return a content-length header so this whole endeavour was wasted...
                            }
                        };

                        // phew - we finally made it (I love rust!)
                        let mut bytes: Vec<u8> = Vec::with_capacity(expected_read_size);
                        match resp.into_reader().take(max_image_size).read_to_end(&mut bytes) {
                            Ok(read_size) => {
                                if expected_read_size > 0 && read_size != expected_read_size {
                                    println!("Mismatching payload sizes := expected {expected_read_size} but got {read_size}");
                                    return None;
                                }
                            },
                            Err(err) => {
                                println!("Failed to read image data: ${err}");
                                return None;
                            }
                        };

                    match ImageInfo::from_raw_data(&bytes) {
                        Ok(_) => return Some(bytes),
                        Err(err) => {
                            println!("Failed to read image header: {err}");
                            return None;
                        }
                    }}
                    Err(err) => {
                        println!("Failed to download image: {err}");
                        return None;
                    }
                };
            }
            None => {
                println!("[WARNING] Url not found in memebox element");
                None
            }
        })
        .collect();

    for (index, meme) in elements.iter().enumerate() {
        let image_type = ImageInfo::from_raw_data(&meme)
            .unwrap()
            .mimetype
            .split("/")
            .nth(1)
            .unwrap();

        let file_name = &format!("{}.{}", index + 1, image_type);
        let mut file = File::create(file_name).expect("Failed to create file");

        match file.write(&meme) {
            Ok(_) => (),
            Err(err) => println!("Failed to save image {}: {err}", file_name),
        }
    }
    // println!("{:?}", body)
    // memethumb
}
