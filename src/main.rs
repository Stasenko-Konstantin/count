use clap::Parser;
use encoding_rs::UTF_8;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, Read};
use std::path::PathBuf;

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
    tree: i32,
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
        args.paths,
        &args.excludes,
        if args.tree > 0 {
            args.tree + 1
        } else {
            args.tree
        },
    )?;
    walk(t, args.ext, &args.excludes, args.tree)?;
    println!("\ndone!");
    Ok(())
}

fn make_tree(
    tree: &mut PBTree,
    paths: Vec<PathBuf>,
    excludes: &Vec<String>,
    t_deep: i32,
) -> Result<(), Error> {
    if t_deep == -1 {
        return Ok(());
    }
    let mut r_paths = Vec::new();
    let mut r_nodes = Vec::new();
    for p in paths {
        if is_path_need_exclude(p.clone(), None, excludes.to_vec()) {
            continue;
        }
        if p.is_file() || t_deep == 0 {
            r_paths.push(p);
            continue;
        }
        let mut t = PBTree::default();
        t.curr_path = Some(p.clone());
        let d = std::fs::read_dir(p.clone());
        let dv: Vec<PathBuf> = d?.map(|e| e.unwrap().path()).collect();
        make_tree(&mut t, dv, excludes, t_deep - 1)?;
        r_nodes.push(t);
    }
    tree.paths = r_paths;
    tree.nodes = r_nodes;
    Ok(())
}

fn walk(
    tree: PBTree,
    ext: Option<String>,
    excludes: &Vec<String>,
    t_deep: i32,
) -> Result<(), Error> {
    if !tree.paths.is_empty()
        && tree.curr_path.clone().is_some()
        && tree.curr_path.clone().unwrap() != std::env::current_dir()?
        || t_deep == 0
    {
        print(
            if tree.curr_path.is_none() {
                String::new()
            } else {
                tree.curr_path.clone().unwrap().display().to_string()
            },
            tree.paths,
            ext.clone(),
            excludes,
        )?;
    }

    if tree.nodes.is_empty() {
        return Ok(());
    }
    for t in tree.nodes {
        walk(t, ext.clone(), excludes, t_deep)?;
    }
    Ok(())
}

fn print(
    curr_path: String,
    paths: Vec<PathBuf>,
    ext: Option<String>,
    excludes: &[String],
) -> Result<(), Error> {
    let file_index = &mut Vec::<PathBuf>::new();
    make_index(paths, file_index, ext, excludes.to_vec())?;
    println!("\ncounting {} files in {}...", file_index.len(), curr_path,);
    let mut hmap = HashMap::new();
    let res = count(file_index, &mut hmap)?;
    println!("{:#?}", res);
    Ok(())
}

fn is_path_need_exclude(path: PathBuf, extension: Option<String>, excludes: Vec<String>) -> bool {
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
        || (extension.is_some() && extension.unwrap() != ext && ext != *"")
}

fn make_index(
    paths: Vec<PathBuf>,
    vec: &mut Vec<PathBuf>,
    extension: Option<String>,
    excludes: Vec<String>,
) -> Result<&mut Vec<PathBuf>, Error> {
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
