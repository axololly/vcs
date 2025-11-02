use std::{ffi::OsStr, fs, path::{Path, PathBuf}};
use regex::Regex;

// Taken from https://docs.rs/wildpath, but cleaned up to abide by Clippy logs.
pub fn resolve_wildcard_path(path: &Path) -> Vec<PathBuf> {
    let mut fin: Vec<PathBuf> = Vec::new();

    match path.iter().next() {
        Some(e) => {
            let mut t = PathBuf::new();

            t.push(e);
            
            _ = t.canonicalize();
            
            fin.push(t);
        }

        None => return fin
    }

    for pe in path.iter().skip(1) {
        if fin.is_empty() {
            return fin;
        }

        fin = get_next_file_layer(fin, pe);
    }
    
    fin
}

fn get_next_file_layer(current_layer: Vec<PathBuf>, next_element: &OsStr) -> Vec<PathBuf> {
    let mut new_layer: Vec<PathBuf> = Vec::new();

    for p in &current_layer {
        if p.is_file() { continue; }
        
        let mut candidates = if next_element.to_str().unwrap().contains("*") {
            let re = Regex::new(
                &format!("^{}$", &next_element.to_str().unwrap().replace(".", "[.]").replace("*", ".*"))
            ).unwrap_or_else(|_| {
                panic!("Failed to create regex from {:?}", next_element);
            });

            let regex_filter = |x: PathBuf| -> Option<PathBuf> {
                if re.is_match(x.iter().next_back().unwrap_or_else(|| {
                    panic!("Failed to parse with regex {:?}", x);
                }).to_str().unwrap()) {
                    Some(p.join(x))
                } else {
                    None
                }
            };

            fs::read_dir(p)
                .unwrap_or_else(|e| panic!("Failed to read directory: {} (error: {e:?})", p.display()))
                .map(|x| PathBuf::from(x.unwrap().file_name()))
                .filter_map(regex_filter)
                .collect()
        }
        else if p.join(next_element).try_exists().unwrap_or_else(|e| {
            panic!("Failed to determine if {:?} exists in {:?} (error: {e})", next_element, p);
        }) {
            vec![PathBuf::from(next_element)]
        } else {
            vec![]
        };

        candidates = candidates.into_iter()
            .map(|x|
                if x.is_symlink() {
                    x.read_link().unwrap_or_else(|e| {
                        panic!("Failed to follow symlink from {x:?} (error: {e})")
                    })
                }
                else {
                    x
                }
            )
            .map(|x| p.join(x))
            .collect();

        new_layer.append(&mut candidates);
    }
    
    new_layer
}