/* 
add timestamp to all HTML includes and the respective file names

*/
#![feature(nll)]
extern crate epub;
extern crate image;
extern crate scraper;
#[macro_use]
extern crate tera;
#[macro_use]
extern crate lazy_static;
extern crate fs_extra;
#[macro_use]
extern crate clap;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use epub::doc::EpubDoc;
use fs_extra::dir::*;
use image::{FilterType, GenericImage};
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tera::{Context, Tera};

const MAX_WIDTH: u32 = 400;
const COVER_WIDTH: u32 = 700;
const ICON_WIDTH: u32 = 192;
static DEFAULT_OUTPUT_FOLDER: &'static str = "web/";

#[derive(Serialize, Deserialize, Clone)]
struct Book {
    info_url: String,
    base_url: String,
    description: String,
    epub: String,
    output_folder: String,
}

#[derive(Serialize, Deserialize)]
struct BatchJob {
    templates: String,
    books: Vec<Book>,
}

lazy_static! {
    pub static ref TERA: Tera = {
        let mut tera = compile_templates!("templates/**/*");
        // and we can add more things to our instance if we want to
        tera.autoescape_on(vec!["html"]);
        tera
    };
}

fn extract_filename(path: &str) -> &str {
    let r = Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    return r;
}

fn copy_index_to_cover(output_root: &Path) {
    fs::copy(
        output_root.join("index.html"),
        output_root.join("cover.html"),
    ).expect("Can't create cover.html");
}

fn move_service_worker(output_root: &Path) {
    fs::copy(
        output_root.join("resources/static/sw.js"),
        output_root.join("sw.js"),
    ).expect("Can't create sw.js");

    //todo: add timestamp renaming here

    // let start = SystemTime::now();
    // let since_the_epoch = start
    //     .duration_since(UNIX_EPOCH)
    //     .expect("Time went backwards");
    // let secs = since_the_epoch.as_secs().to_string();

    // // renaming CSS
    // fs::rename(
    //     output_root.join("resources/static/mobile.css"),
    //     output_root.join(format!("resources/static/mobile.{}.css", &secs)),
    // ).expect("Can't rename mobile.css");

    // fs::rename(
    //     output_root.join("resources/static/reader.css"),
    //     output_root.join(format!("resources/static/reader.{}.css", &secs)),
    // ).expect("Can't rename reader.css");

    // // update all HTML.
}

fn get_metadata(book: &Book) -> HashMap<&str, String> {
    let input_file = &book.epub;
    let doc = EpubDoc::new(input_file).unwrap();
    let mut metadata = HashMap::new();
    let title = doc.mdata("title").unwrap_or_default();
    let author = doc.mdata("creator").unwrap_or_default();
    let date = doc.mdata("date").unwrap_or_default();
    let description = &book.description;
    let base_url = &book.base_url;
    let info_url = &book.info_url;

    metadata.insert("title", title.clone());
    metadata.insert("author", author.clone());
    metadata.insert("date", date.clone());
    metadata.insert("description", description.clone());
    metadata.insert("info_url", info_url.clone());
    metadata.insert("base_url", base_url.clone());

    return metadata;
}

fn compress_cover(book: &Book) {
    let input_file = &book.epub;
    let output_root = Path::new(&book.output_folder);
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let mut doc = doc.unwrap();
    println!("Extracting cover...");
    let cover_data = doc.get_cover().unwrap();

    let f = fs::File::create("temp/cover.jpg");
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let _resp = f.write_all(&cover_data);
    println!("Compressing cover...");

    let img = image::open("temp/cover.jpg").unwrap();
    let resized = img.resize(COVER_WIDTH, COVER_WIDTH, FilterType::Lanczos3);
    resized
        .save(output_root.join("cover.jpg"))
        .expect("Saving image failed");

    let ref mut _background = image::RgbaImage::new(ICON_WIDTH, ICON_WIDTH);
    let img = image::open("temp/cover.jpg").unwrap();
    let resized_icon = img.resize_exact(ICON_WIDTH, ICON_WIDTH, FilterType::Lanczos3);
    // let icon_buf = ImageBuffer::from_vec(
    //     resized_icon.width(),
    //     resized_icon.height(),
    //     resized_icon.raw_pixels(),
    // ).expect("can't create icon buffer");

    // imageops::overlay(background, &icon_buf, ICON_WIDTH / 2, ICON_WIDTH / 2);

    resized_icon
        .save(output_root.join("icon.jpg"))
        .expect("Saving icon failed");

    // create cover html ...
    let metadata = get_metadata(&book);

    let mut ctx = Context::new();
    for (key, val) in metadata.into_iter() {
        ctx.add(key, &val);
    }

    let mut chapter = HashMap::new();
    chapter.insert("title", "Table of Contents");
    chapter.insert("filename", "index.html");

    ctx.add("chapter", &chapter);

    let next_chapter_id = &doc.spine[2];
    let next_chapter = &doc.resources.get(next_chapter_id);

    match next_chapter {
        Some(s) => ctx.add("next", extract_filename(&s.0)),
        None => ctx.add("next", &false),
    }

    let rendered = TERA
        .render("index.html", &ctx)
        .expect("Failed to render template");

    let f = fs::File::create(output_root.join("index.html"));
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let _resp = f.write_all(&rendered.as_bytes());
}

fn copy_raw_resource(input_file: &str, key: &str, path: &str, output_root: &Path) {
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let mut doc = doc.unwrap();
    let filename = Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();

    let data = doc.get_resource(key);

    // write raw file
    let raw_filename = output_root.join("resources").join(&filename);
    let f = fs::File::create(&raw_filename);
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let _resp = f.write_all(&data.unwrap().as_slice());
}

fn process_css_resource(input_file: &str, key: &str, path: &str, output_root: &Path) {
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let mut doc = doc.unwrap();
    let filename = Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();

    //  write fragment
    let str_data = doc.get_resource_str(key);

    let fixed_content = str_data.unwrap().replace("../fonts/", "");

    let full_path = output_root.join("resources").join(&filename);
    let f = fs::File::create(&full_path);
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let _resp = f.write_all(&fixed_content.as_bytes());
}
fn process_manifest(_input_file: &str, metadata: &HashMap<&str, String>, output_root: &Path) {
    let mut ctx = Context::new();
    for (key, val) in metadata.into_iter() {
        ctx.add(key, &val);
    }

    let rendered = TERA
        .render("manifest.webmanifest", &ctx)
        .expect("Failed to render manifest");

    let fragment_filename = output_root.join("manifest.webmanifest");
    let f = fs::File::create(&fragment_filename);
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let _resp = f.write_all(&rendered.as_bytes());
}

fn process_toc(input_file: &str, metadata: &HashMap<&str, String>, key: &str, output_root: &Path) {
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let mut doc = doc.unwrap();
    let mut ctx = Context::new();
    for (key, val) in metadata.into_iter() {
        ctx.add(key, &val);
    }

    let mut chapter = HashMap::new();
    chapter.insert("title", "Table of Contents");
    chapter.insert("filename", "toc.html");

    ctx.add("chapter", &chapter);

    //  write fragment
    let str_data = doc.get_resource_str(key);

    let fixed_content = str_data.unwrap().replace("../images", "images");

    let document = Html::parse_document(&fixed_content);
    let selector = Selector::parse("body").unwrap();
    let body = document.select(&selector).next().unwrap();
    ctx.add("content", &body.inner_html());

    let rendered = TERA
        .render("page.html", &ctx)
        .expect("Failed to render template");

    let fragment_filename = output_root.join("toc.html");
    let f = fs::File::create(&fragment_filename);
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let _resp = f.write_all(&rendered.as_bytes());
}

fn process_html_resource(
    input_file: &str,
    metadata: &HashMap<&str, String>,
    key: &str,
    path: &str,
    output_root: &Path,
) -> usize {
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let mut doc = doc.unwrap();
    let filename = Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();

    let mut ctx = Context::new();
    for (key, val) in metadata.into_iter() {
        ctx.add(key, &val);
    }

    let mut chapter = HashMap::new();
    chapter.insert("title", "");
    chapter.insert("filename", &filename);

    ctx.add("chapter", &chapter);

    //  write fragment
    let str_data = doc.get_resource_str(key);

    let fixed_content = str_data.unwrap().replace("../images", "images");

    let document = Html::parse_document(&fixed_content);
    let selector = Selector::parse("body").unwrap();
    let body = document.select(&selector).next().unwrap();
    ctx.add("content", &body.inner_html());

    let link_selector = Selector::parse("a").unwrap();
    let total_links = document.select(&link_selector).count();

    let current_chapter_position = &doc
        .spine
        .iter()
        .position(|ref mut r| r.as_str() == key)
        .unwrap();

    if (current_chapter_position + 1) < doc.spine.len() {
        let next_chapter_id = &doc.spine[current_chapter_position + 1];
        let next_chapter = &doc.resources.get(next_chapter_id);

        match next_chapter {
            Some(s) => ctx.add("next", extract_filename(&s.0)),
            None => ctx.add("next", &false),
        }
    }

    if *current_chapter_position > 0 {
        let previous_chapter_id = &doc.spine[current_chapter_position - 1];
        let previous_chapter = &doc.resources.get(previous_chapter_id);

        match previous_chapter {
            Some(s) => ctx.add("previous", extract_filename(&s.0)),
            None => ctx.add("previous", &false),
        }
    }

    let rendered = TERA
        .render("page.html", &ctx)
        .expect("Failed to render template");

    let fragment_filename = output_root.join(&filename);
    let f = fs::File::create(&fragment_filename);
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let _resp = f.write_all(&rendered.as_bytes());
    return total_links;
}

fn compress_image_resource(input_file: &str, key: &str, path: &str, output_root: &Path) {
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let mut doc = doc.unwrap();
    // write compressed
    let ext = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default();

    let raw_filename = format!("temp/images/{}.{}", &key, &ext);
    let data = doc.get_resource(key);

    // write raw file
    let f = fs::File::create(&raw_filename);
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let _resp = f.write_all(&data.unwrap().as_slice());

    let img = image::open(raw_filename).unwrap();
    let (width, _height) = img.dimensions();
    let compressed_filename = output_root
        .join("images")
        .join(format!("{}.{}", &key, &ext)); // pay attention to this, it might be the wrong name.

    if width > MAX_WIDTH {
        let resized = img.resize(MAX_WIDTH, MAX_WIDTH, FilterType::Lanczos3);
        resized
            .save(&compressed_filename)
            .expect("Saving image failed");
    } else {
        let data = doc.get_resource(key);

        let f = fs::File::create(&compressed_filename);
        assert!(f.is_ok());
        let mut f = f.unwrap();
        let _resp = f.write_all(&data.unwrap().as_slice());
    }
}

fn copy_template_resources(output_root: &Path) {
    println!("Copying static resources...");
    let mut options = CopyOptions::new();
    options.copy_inside = true;
    options.overwrite = true;
    let handle = |process_info: TransitProcess| {
        println!("  copy: {}", process_info.file_name);
        fs_extra::dir::TransitProcessResult::ContinueOrAbort
    };
    copy_with_progress("static", output_root.join("resources"), &options, handle)
        .expect("failed copying static resource");
}

fn process_book(book: &Book) {
    let doc = EpubDoc::new(&book.epub);
    let output_root = &book.output_folder;
    assert!(doc.is_ok());
    let doc = doc.unwrap();
    let output_root = Path::new(output_root);

    let _resp = fs::create_dir_all("temp/images/"); // needed because resize lib wants to work with files

    // assemble destination folder
    let _resp = fs::create_dir_all(&output_root);
    let _resp = fs::create_dir_all(output_root.join("images/"));
    let _resp = fs::create_dir_all(output_root.join("resources/"));

    let metadata = get_metadata(&book);
    println!(
        "Book: {} - {} ({})",
        &metadata.get("title").unwrap(),
        &metadata.get("author").unwrap(),
        &metadata.get("date").unwrap()
    );

    copy_template_resources(&output_root);

    let num_resources = doc.resources.len();
    println!("Total resources listed in Epub: {}", num_resources);

    compress_cover(&book);

    let resources = doc.resources.clone();
    println!("Extracting resources...");
    println!("Code:\n# -> images\n. -> HTML\nC -> CSS\n@ -> raw file resource\n");

    let mut max_links = 0;
    let mut toc_id = "";
    for (key, val) in resources.iter() {
        let path = &val.0;
        let mime = &doc.get_resource_mime_by_path(&path).unwrap_or_default();

        if mime.contains("image/") {
            print!("#");
            compress_image_resource(&book.epub, &key, &path, &output_root);
        } else if mime.contains("html") {
            print!(".");
            let total_links =
                process_html_resource(&book.epub, &metadata, &key, &path, &output_root);
            if max_links < total_links {
                max_links = total_links;
                toc_id = key;
            }
        } else if mime.contains("css") {
            print!("C");
            process_css_resource(&book.epub, &key, &path, &output_root);
        } else {
            print!("@");
            copy_raw_resource(&book.epub, &key, &path, &output_root);
        }
        let _r = io::stdout().flush();
    }
    process_toc(&book.epub, &metadata, &toc_id, &output_root);
    process_manifest(&book.epub, &metadata, &output_root);
    copy_index_to_cover(&output_root);
    move_service_worker(&output_root);
}

fn process_batch_job(path: &str) {
    // Open the file in read-only mode.
    let file = File::open(path).expect("Can't open batch job json file");

    // Read the JSON contents of the file as an instance of `User`.
    let batch: BatchJob = serde_json::from_reader(file).expect("Can't decode batch job json file");
    let mut processed: Vec<Book> = vec![];
    for book in batch.books {
        process_book(&book);
        &processed.push(book.clone());
        println!("\n");
    }

    // Serialize it to a JSON string.
    let j = serde_json::to_string(&processed).expect("Can't display processed queue");

    // Print, write to a file, or send to an HTTP server.
    println!("{}", j);

    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let secs = since_the_epoch.as_secs().to_string();
    fs::write(format!("batchjob.{}.json", &secs), &j).expect("Can't write batch report");
}

fn main() {
    let matches = clap_app!(epub2pwa =>
        (version: "1.0")
        (author: "Andre Alves Garzia <andre@andregarzia.com")
        (about: "Converts ePub books into PWAs")
        (@arg INFOURL: -i --infourl +takes_value "Info URL for the book")
        (@arg BASEURL: -u --baseurl +takes_value "Base URL for the book")
        (@arg DESCRIPTION: -d --description +takes_value "Description for the book")
        (@arg INPUT: -i --input +takes_value "Sets the input file to use")
        (@arg OUTPUT: -o --output +takes_value "Sets the output folder")
        (@arg BATCH: -b --batch +takes_value "Pass a json for batch jobs")
        (@arg debug: -d ... "Sets the level of debugging information")
    ).get_matches();

    let batch = matches.value_of("BATCH");
    match batch {
        Some(json) => {
            // batch processing.
            process_batch_job(json);
        }
        None => {
            // single book processing
            let epub = matches
                .value_of("INPUT")
                .expect("Must pass ePub file as argument.");
            let output_folder = matches.value_of("OUTPUT").unwrap_or(DEFAULT_OUTPUT_FOLDER);
            let info_url = matches.value_of("INFOURL").unwrap_or("");
            let base_url = matches.value_of("INFOURL").unwrap_or("");
            let description = matches.value_of("DESCRIPTION").unwrap_or("");

            let book: Book = Book {
                info_url: info_url.to_string(),
                base_url: base_url.to_string(),
                epub: epub.to_string(),
                description: description.to_string(),
                output_folder: output_folder.to_string(),
            };

            process_book(&book);
        }
    }
}
