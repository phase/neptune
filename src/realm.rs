use std::fs;

use fs_extra::dir::CopyOptions;
use yaml_rust::Yaml;

use crate::gamemode::GameMode;
use crate::{read_yaml, visit_dirs};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::io::{Read, Write};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Realm {
    id: String,
    name: String,
    gamemode: GameMode,
    attributes: HashMap<String, String>,
}

impl Realm {
    pub fn read(realm_name: String) -> Realm {
        Realm::new(realm_name.clone(), &read_yaml(format!("realm/{}.yml", realm_name))[0])
    }

    pub fn new(id: String, yaml: &Yaml) -> Realm {
        let name = yaml["name"].as_str().expect("no name found in realm file").to_string();
        let gamemode_name = yaml["gamemode"].as_str().expect("no gamemode found in realm file").to_string();

        let mut attributes = HashMap::new();
        if let Some(hash) = yaml["attributes"].as_hash() {
            for (key, value) in hash.iter() {
                let key = key.as_str().unwrap().to_string();
                let value = value.as_str().unwrap().to_string();
                attributes.insert(key, value);
            }
        }

        Realm {
            id,
            name,
            gamemode: GameMode::read(gamemode_name),
            attributes,
        }
    }

    pub fn content_folder(&self) -> String {
        format!("realm/{}/", self.id)
    }

    pub fn output_folder(&self) -> String {
        format!("out/server/{}", self.id)
    }

    pub fn copy_realm_files(&self) {
        let result = fs::read_dir(self.content_folder());
        // this directory is optional, don't panic if we can't find it
        if let Err(_) = result { return; }

        let paths = result.unwrap();
        for path in paths {
            let path = path.expect("error reading server path").path();
            if path.is_file() {
                fs::copy(path.to_str().unwrap(), format!("{}/{}", self.output_folder(), path.file_name().unwrap().to_str().unwrap()))
                    .expect("failed to copy server files");
            } else if path.is_dir() {
                let mut options = CopyOptions::new();
                options.overwrite = true;
                fs_extra::dir::copy(path, self.output_folder(), &options).expect("failed to copy gamemode files");
            }
        }
    }

    pub fn write_eula(&self) {
        // yes we accept the eula
        fs::write(format!("{}/eula.txt", self.output_folder()), "eula=true\n").expect("unable to accept eula")
    }

    pub fn make_replacements(&self) {
        visit_dirs(self.output_folder().as_ref(), &|path| {
            if path.ends_with(".yml")
                || path.ends_with(".yaml")
                || path.ends_with(".json")
                || path.ends_with(".txt") {
                let mut replacements = self.gamemode.get_replacements();
                for (key, value) in self.attributes.iter() {
                    replacements.insert(key.clone(), value.clone());
                }
                replacements.insert("id".to_string(), self.id.clone());
                replacements.insert("name".to_string(), self.name.clone());

                // open the file and read to a buffer
                let file_path = Path::new(path);
                let mut src = File::open(&file_path).unwrap();
                let mut data = String::new();
                src.read_to_string(&mut data).unwrap();
                drop(src);  // Close the file early

                // make replacements
                for (from, to) in replacements {
                    let from = format!("$$REALM_{}$$", from.to_ascii_uppercase().replace("-", "_"));
                    data = data.replace(&from, &to);
                }

                let mut dest = File::create(&file_path).unwrap();
                dest.write(data.as_bytes()).unwrap();
            }
        }).unwrap();
    }

    pub fn build_files(self) {
        fs::create_dir_all(format!("{}/plugins/", self.output_folder())).expect("failed to create plugin output directory");
        self.gamemode.copy_server_files(&self);
        self.gamemode.copy_plugins(&self);
        self.gamemode.copy_gamemode_files(&self);
        self.copy_realm_files();
        self.make_replacements();
        self.write_eula();
    }
}
