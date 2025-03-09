use clap::Parser;
use encoding_rs::UTF_8;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    paths: Vec<PathBuf>,

    #[arg(short, long, value_name = "EXTENSION")]
    ext: Option<String>,

    #[arg(
        short = 'E',
        long,
        help = "Excludes specified file names and/or extensions"
    )]
    excludes: Vec<String>,

    #[arg(short, long, default_value_t = 0)]
    tree: u8,
}

#[derive(Default)]
struct PBTree {
    paths: Vec<PathBuf>,
    nodes: Vec<PBTree>,
    curr_path: Option<PathBuf>,
}

fn main() -> Result<(), Error> {
    let mut args = Args::parse();
    let curr_path = std::env::current_dir()?;
    if args.paths.is_empty() {
        args.paths = vec![curr_path.clone()]
    };
    let mut t: PBTree = PBTree::default();
    if args.tree == 0 {
        t.curr_path = Some(curr_path);
    }
    make_tree(
        &mut t,
        &mut args.paths,
        &args.excludes,
        if args.tree > 0 {
            (args.tree + 1).into()
        } else {
            args.tree.into()
        },
    );
    walk(t, &args.ext, &args.excludes, args.tree.into())?;
    println!("\ndone!");
    Ok(())
}

fn make_tree(tree: &mut PBTree, paths: &mut Vec<PathBuf>, excludes: &Vec<String>, t_deep: i32) {
    if t_deep == -1 {
        return;
    }

    let m_nodes = Mutex::new(Vec::new());
    let m_paths = Mutex::new(Vec::new());

    paths.par_iter().for_each(|p| {
        if !is_path_need_exclude(p, &None, excludes) {
            if p.is_dir() || t_deep != 0 {
                let mut t = PBTree {
                    curr_path: Some(p.clone()),
                    ..Default::default()
                };
                let d = std::fs::read_dir(p);
                let mut dv: Vec<PathBuf> = d.unwrap().map(|e| e.unwrap().path()).collect();
                make_tree(&mut t, &mut dv, excludes, t_deep - 1);
                m_nodes.lock().unwrap().push(t);
            }
            m_paths.lock().unwrap().push(p.to_path_buf());
        }
    });

    tree.nodes = m_nodes.into_inner().unwrap();
    tree.paths = m_paths.into_inner().unwrap();
}

fn walk(
    tree: PBTree,
    ext: &Option<String>,
    excludes: &Vec<String>,
    t_deep: i32,
) -> Result<(), Error> {
    let curr_path = &tree.curr_path;
    if !tree.paths.is_empty()
        && curr_path.is_some()
        && (*curr_path.as_ref().unwrap() != std::env::current_dir()? || t_deep == 0)
    {
        print(
            if curr_path.is_none() {
                String::new()
            } else {
                curr_path.as_ref().unwrap().display().to_string()
            },
            tree.paths,
            ext,
            excludes,
        )?;
    }

    if tree.nodes.is_empty() {
        return Ok(());
    }
    for t in tree.nodes {
        walk(t, ext, excludes, t_deep)?;
    }
    Ok(())
}

fn print(
    curr_path: String,
    paths: Vec<PathBuf>,
    ext: &Option<String>,
    excludes: &[String],
) -> Result<(), Error> {
    let file_index = &mut Vec::<PathBuf>::new();
    make_index(paths, file_index, ext, excludes)?;
    if file_index.is_empty() {
        return Ok(());
    }
    println!("\ncounting {} files in {}...", file_index.len(), curr_path);
    let mut hmap = HashMap::new();
    let res = count(file_index, &mut hmap)?;
    println!("{:#?}", res);
    Ok(())
}

fn is_path_need_exclude(path: &Path, extension: &Option<String>, excludes: &[String]) -> bool {
    let file_name = path
        .file_name()
        .unwrap_or("".as_ref())
        .to_str()
        .unwrap_or("")
        .to_string();
    let ext = ".".to_string()
        + &*path
            .extension()
            .unwrap_or("".as_ref())
            .to_str()
            .unwrap_or("")
            .to_string();
    file_name.starts_with('.')
        || excludes.contains(&file_name)
        || excludes.contains(&ext)
        || (extension.is_some() && *extension.as_ref().unwrap() != ext && ext != *"")
}

fn make_index<'a>(
    paths: Vec<PathBuf>,
    vec: &'a mut Vec<PathBuf>,
    extension: &Option<String>,
    excludes: &'a [String],
) -> Result<&'a mut Vec<PathBuf>, Error> {
    for path in paths {
        if is_path_need_exclude(&path, &None, excludes) {
            continue;
        }
        if path.is_file() {
            vec.push(path);
            continue;
        }
        let dir = std::fs::read_dir(path)?;
        for entry in dir {
            let entry = entry?;
            if is_path_need_exclude(&entry.path(), extension, excludes) {
                continue;
            }
            if entry.path().is_dir() {
                make_index(vec![entry.path()], vec, extension, excludes)?;
            }
            if !entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains('.')
            {
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
        let fl = match count_file_lines(file_path) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let res = if let Some(i) = map.get_mut(&ext) {
            *i
        } else {
            0
        } + fl;
        if res != 0 {
            map.insert(ext, res);
        }
    }
    Ok(map)
}

fn count_file_lines(file_path: &mut PathBuf) -> Result<i32, Error> {
    let mut file = String::new();

    File::read_to_string(&mut File::open(file_path)?, &mut file)?;
    let res = file.split('\n').fold(0, |sum, _| sum + 1);
    Ok(res)
}

fn is_text_file(path: PathBuf) -> bool {
    let f = || {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let text_threshold = 0.8; // TODO very strange stuff
        let (cow, _, had_errors) = UTF_8.decode(&buffer);
        if had_errors {
            return Ok(false);
        }
        let printable_chars = cow
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .count();
        let total_chars = cow.chars().count();
        Ok::<bool, Error>(printable_chars as f64 / total_chars as f64 > text_threshold)
    };
    f().unwrap_or(false)
}
