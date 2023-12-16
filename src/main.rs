use regex::Regex;
use scraper::{self, ElementRef, Html, Selector};
use std::fs;
use std::sync::OnceLock;
use ureq;

const URL: &str = "https://gbatemp.net/";
const SELECTOR: &str = ".memethumb";

static EXTENSION_EXTRACTOR: OnceLock<Regex> = OnceLock::new();

fn extract_link(ele: ElementRef<'_>) -> Option<String> {
    if ele.value().id().is_some_and(|id| id == "mememorelink") {
        None
    } else {
        ele.value()
            .attr("href")
            .and_then(|link| Some(format!("{}{}", URL, link)))
            .or_else(|| {
                println!("Malformed meme; {:?}", ele);
                None
            })
    }
}

fn download_image(link: &str) -> Option<(Vec<u8>, String)> {
    let resp = match ureq::get(&link).call() {
        Ok(resp) => resp,
        Err(err) => {
            println!("Failed to download image; {err}");
            return None;
        }
    };

    let expected_size = resp
        .header("content-length")
        .and_then(|val| str::parse::<usize>(val).ok())
        .unwrap_or(0);

    let Some(extension) = resp.header("Content-Disposition").and_then(|string| {
        EXTENSION_EXTRACTOR
            .get()
            .unwrap()
            .captures(string)
            .and_then(|captures| {
                captures
                    .get(0)
                    .and_then(|matched| Some(matched.as_str().to_string()))
            })
    }) else {
        println!("Failed to find image extenstion from content-disposition header");
        return None;
    };

    let mut img_buf: Vec<u8> = Vec::with_capacity(expected_size);
    match resp.into_reader().read_to_end(&mut img_buf) {
        Ok(size) if expected_size > 0 && size > expected_size => {
            println!(
                "Expected {} byte but got {} bytes, failed",
                size, expected_size
            );
            return None;
        }
        Err(err) => {
            println!("Failed to read image data; {err}");
            return None;
        }
        _ => (),
    };

    Some((img_buf, extension)) // a bit of cloning but whatever
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = EXTENSION_EXTRACTOR.set(Regex::new(r"\.(\w+)")?);

    println!("Downloading webpage");
    let webpage = &ureq::get(URL).call()?.into_string()?;

    println!("Parsing webpage");
    let webpage = Html::parse_document(&webpage);

    println!("Extracting image links");
    let selector = Selector::parse(SELECTOR)?;
    let image_links = webpage.select(&selector).filter_map(extract_link);

    println!("Downloading memes");
    for (index, link) in image_links.enumerate() {
        let Some((data, extension)) = download_image(&link) else {
            continue;
        };

        if let Err(err) = fs::write(format!("{}{}", index, extension), data) {
            println!("Failed to write image to disk; {err}")
        }
    }

    Ok(())
}
