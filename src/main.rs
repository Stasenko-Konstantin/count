use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, Read};
use std::path::PathBuf;
use encoding_rs::UTF_8;
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    paths: Vec<PathBuf>,
    
    #[arg(short, long, value_name = "EXTENSION")]
    ext: Option<String>,
    
    #[arg(short = 'E', long, help = "Excludes specified file names and/or extensions")]
    excludes: Vec<String>,
}

fn main() -> Result<(), Error> {
    let mut args = Args::parse();
    if args.paths.is_empty() {
        args.paths = vec![std::env::current_dir()?]
    };
    let file_index = &mut Vec::<PathBuf>::new();
    make_index(args.paths, file_index, args.ext, args.excludes)?;
    println!("counting {} files...\n", file_index.len());
    let mut hmap = HashMap::new();
    let res = count(file_index, &mut hmap)?;
    println!("{:#?}", res);
    println!("\ndone!");
    Ok(())
}

fn is_path_need_exclude(path: PathBuf, extension: Option<String>, excludes: Vec<String>) -> bool {
    let file_name = path.file_name().unwrap_or("".as_ref())
        .to_str().unwrap_or("").to_string();
    let ext = ".".to_string() + &*path.extension().unwrap_or("".as_ref())
        .to_str().unwrap_or("").to_string();
    file_name.starts_with('.') ||
        excludes.contains(&file_name) ||
        excludes.contains(&ext) ||
        (extension.is_some() && extension.unwrap() != ext)
}

fn make_index(paths: Vec<PathBuf>, 
              vec: &mut Vec<PathBuf>, 
              extension: Option<String>, 
              excludes: Vec<String>) 
    -> Result<&mut Vec<PathBuf>, Error> {
    for path in paths {
        if is_path_need_exclude(path.clone(), None, excludes.clone()) {
            continue;
        }
        if path.is_file() {
            vec.push(path);
            continue;
        }
        let dir = std::fs::read_dir(path.clone())?;
        for entry in dir {
            let entry = entry?;
            if is_path_need_exclude(entry.path(), extension.clone(), excludes.clone()) {
                continue;
            }
            if entry.path().is_dir() {
                make_index(vec![entry.path()], vec, extension.clone(), excludes.clone())?;
            }
            if !entry.path().file_name().unwrap().to_str().unwrap().contains('.') {
                continue;
            }
            if !is_text_file(entry.path()) {
                continue;
            }
            vec.push(entry.path());
        }
    }
    Ok(vec)
}

fn count<'a>(
    file_index: &mut Vec<PathBuf>,
    map: &'a mut HashMap<String, i32>,
) -> Result<&'a mut HashMap<String, i32>, Error> {
    for file_path in file_index {
        let mut ext = ".".to_string();
        if let Some(ext_os_str) = file_path.extension() {
            if let Some(ext_str) = ext_os_str.to_str() {
                ext = ext_str.to_string();
            }
        }
        let res = if let Some(i) = map.get_mut(&ext) {
            *i
        } else {
            0
        } + count_file_lines(file_path)?;
        if res != 0 {
            map.insert(ext, res);
        }
    }
    Ok(map)
}

fn count_file_lines(file_path: &mut PathBuf) -> Result<i32, Error> {
    let mut file = String::new(); 
    File::read_to_string(&mut File::open(file_path)?, &mut file)?;
    let res = file.split('\n').fold(0, |sum, _| sum+1);
    Ok(res)
}

fn is_text_file(path: PathBuf) -> bool {
    let f = || {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let text_threshold = 0.7; // TODO very strange stuff
        let (cow, _, had_errors) = UTF_8.decode(&buffer);
        if had_errors {
            return Ok(false);
        }
        let printable_chars = cow.chars().filter(|c| c.is_alphanumeric() 
            || c.is_whitespace()).count();
        let total_chars = cow.chars().count();
        Ok::<bool, Error>(printable_chars as f64 / total_chars as f64 > text_threshold)
    };
    f().unwrap_or(false)
}
