

extern crate id3;
extern crate mpeg_audio_header;

use std::fmt::format;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use std::fs;
use std::env;
use std::io::{self, Write};
use crate::id3::TagLike;

use std::io::BufReader;
use mpeg_audio_header::{Header, ParseMode};

fn get_bitrate(path: &Path) -> u32 {
    let file = File::open(path).unwrap();
    let mut source = BufReader::new(file);
    let header = Header::read_from_source(&mut source, ParseMode::IgnoreVbrHeaders).unwrap();

    return header.avg_bitrate_bps.unwrap_or(0) / 1024;

}


struct Metadata {
    track_name: String,
    album: String,
    artist: String,
    track_no: u32,
    size: u64,
    path: String,
    bitrate: u32, // in kbps
}

fn handle_duplicates(duplicates: &HashMap<String, Vec<Metadata>>) {
    let mut i = 0;
    let total = duplicates.len();
    use std::fs::OpenOptions;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("deleted.txt")
        .unwrap();

    file.write("NUMBER - NAME - ALBUM - ARTIST,          OLD PATH,          NEW PATH\n".as_bytes()).unwrap();

    for (key, values) in duplicates {
        i += 1;
        println!("==== HANDLE DUPLICIATE: {} ===", key);
        println!("({:?}/{:?}) Duplicate tracks:", i, total);
        for (i, metadata) in values.iter().enumerate() {
            println!("{}. Track name: {}", i + 1, metadata.track_name);
            println!("   Album: {}", metadata.album);
            println!("   Artist: {}", metadata.artist);
            println!("   Path: {}", metadata.path);
            println!("   Bitrate: {}", metadata.bitrate);
            println!("   Track number: {}", metadata.track_no);
        }

        println!("Select an option:");
        for i in 1..=values.len() {
            println!("{}. Keep track {}", i, i);
        }
        // Add options for the remaining tracks
        println!("{}. Do nothing", values.len() + 1);

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input: u32 = match input.trim().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };

        if input >= values.len() as u32 + 1 {
            // Do nothing
            println!("\t[STA] Did nothing.");
            continue;
        }

        // Delete all tracks except the selected one
        let newPath = values.get((input as usize) - 1).unwrap().path.clone();
        for (i, metadata) in values.iter().enumerate() {
            if i as u32 + 1 != input {
                let path = Path::new(&metadata.path);
                
                println!("[REQ] Delete file {} of {} kbps? (y/n)", metadata.path, metadata.bitrate);
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm).unwrap();
                if confirm.trim().to_lowercase() == "y" || confirm.trim().to_lowercase() == "" {
                    println!("[DEL] Delete {}", metadata.path);
                    std::fs::remove_file(path).unwrap();
                    // Add to list
                    file.write(format!("{},          {},          {}\n", key, metadata.path, newPath).as_bytes()).unwrap();

                }
            }
        }
        println!("[STA] Duplicate handled.");
        print!("");
    }
}

fn extract_metadata(path: &Path) -> Metadata {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            println!("Error opening file: {}", error);
            return Metadata {
                track_name: path.to_str().unwrap().to_owned(),
                album: "".to_owned(),
                artist: "".to_owned(),
                track_no: 0,
                size: match fs::metadata(path) {
                    Ok(meta) => meta.len(),
                    Err(error2) => {
                        println!("Error reading file size of {}: {}", path.display(), error2);
                        0
                    }
                },
                path: path.to_str().unwrap().to_owned(),
                bitrate: 0,
            };
        }
    };
    
    let mut tag = match id3::Tag::read_from(&mut file) {
        Ok(tag) => tag,
        Err(error) => {
            println!("Error reading tag for file {}: {}", path.display(), error);
            return Metadata {
                track_name: path.file_name().unwrap_or(path.to_str().to_owned().unwrap().as_ref()).to_str().unwrap().to_owned(),
                album: "".to_owned(),
                artist: "".to_owned(),
                track_no: 0,
                size: match fs::metadata(path) {
                    Ok(meta) => meta.len(),
                    Err(error2) => {
                        println!("Error reading file size of {}: {}", path.display(), error2);
                        0
                    }
                },
                path: path.to_str().unwrap().to_owned(),
                bitrate: 0,
            };
        }
    };

    let track_name = tag.title().unwrap_or("").to_owned();
    if track_name.is_empty() {
        
        let full_path = path.file_name().unwrap_or(path.to_str().to_owned().unwrap().as_ref()).to_str().unwrap().to_owned();
        println!("Error: No track name found for {} so using {}", path.display(), full_path);
        tag.set_title(full_path);
    }

    let size = match fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(error2) => {
            println!("Error reading file size of {}: {}", path.display(), error2);
            0
        }
    };

    Metadata {
        track_name: tag.title().unwrap_or("").to_owned(),
        album: tag.album().unwrap_or("").to_owned(),
        artist: tag.artist().unwrap_or("").to_owned(),
        track_no: tag.track().unwrap_or(0).to_owned(),
        size: match fs::metadata(path) {
            Ok(meta) => meta.len(),
            Err(error2) => {
                println!("Error reading file size of {}: {}", path.display(), error2);
                0
            }
        },
        path: path.to_str().unwrap().to_owned(),
        bitrate:  get_bitrate(&path)
    }
}

fn find_duplicate_songs(dir: &Path) -> HashMap<String, Vec<Metadata>> {
    let mut duplicates = HashMap::new();

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            // Recursively search for duplicates in subdirectory
            let subdir_duplicates = find_duplicate_songs(&path);
            for (key, value) in subdir_duplicates {
                let vec = duplicates.entry(key).or_insert(Vec::new());
                vec.extend(value);
            }
        } else {
            // Check if file is a supported audio file
            let extension = match path.extension() {
                Some(ext) => ext.to_str().unwrap(),
                None => {
                    println!("Error: file {} has no extension", path.to_str().unwrap());
                    continue;
                }
            };
            if extension == "mp3" {
                // Extract metadata from file
                let metadata = extract_metadata(&path);
                let key = format!("{} - {} - {} - {}", metadata.track_no, metadata.track_name, metadata.album, metadata.artist);
                println!("Got key {} for {}", key, path.display());
                let vec = duplicates.entry(key).or_insert(Vec::new());
                vec.push(metadata);
            }   
        }
    }

    // Return only entries with more than one file
    //duplicates.retain(|_, v| v.len() > 1);

    duplicates
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Please provide a directory path as an argument.");
        return;
    }

    let dir = Path::new(&args[1]);
    let mut duplicates = find_duplicate_songs(dir);

    duplicates.retain(|_, v| v.len() > 1);

    let mut size = 0;

    for (key, value) in &duplicates {
        println!("Duplicate song: {}", key);
        for (index, path) in value.iter().enumerate() {
            println!("- {} at {} kbps", path.path, path.bitrate);
            if index != 0 {
                size += path.size
            }
        }
    }

    handle_duplicates(&duplicates);
    

    println!("Total amount of possible savings: {}", size)
}
