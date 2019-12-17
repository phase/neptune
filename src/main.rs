extern crate fs_extra;
extern crate yaml_rust;

use std::{env, fs, io};
use std::path::Path;

use fs_extra::dir::CopyOptions;
use yaml_rust::{Yaml, YamlLoader};
use crate::realm::Realm;

mod realm;
mod gamemode;

fn main() {
    let args: Vec<String> = env::args().collect();
    let realm_name = args.get(1).expect(&format!("usage: {} <realm>", args.get(0).unwrap())).clone();

    let realm = Realm::read(realm_name);
    let output_directory = realm.output_folder();
    println!("{:#?}\n", realm);
    realm.build_files();

    println!("Created files:");
    visit_dirs(output_directory.as_ref(), &|path| {
        println!("- {}", path);
    }).unwrap();
}

fn read(file: &String) -> String {
    fs::read_to_string(file).expect(&format!("couldn't read file: {}", file))
}

fn read_yaml(file: String) -> Vec<Yaml> {
    let contents = read(&file);
    YamlLoader::load_from_str(&contents).expect(&format!("couldn't parse yaml file: {}", file))
}

fn visit_dirs(dir: &Path, cb: &dyn Fn(&str)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry.path().to_str().unwrap());
            }
        }
    }
    Ok(())
}
