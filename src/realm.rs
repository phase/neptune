use std::fs;

use fs_extra::dir::CopyOptions;
use yaml_rust::Yaml;

use crate::gamemode::GameMode;
use crate::read_yaml;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Realm {
    id: String,
    name: String,
    gamemode: GameMode,
}

impl Realm {
    pub fn read(realm_name: String) -> Realm {
        Realm::new(realm_name.clone(), &read_yaml(format!("realm/{}.yml", realm_name))[0])
    }

    pub fn new(id: String, yaml: &Yaml) -> Realm {
        let name = yaml["name"].as_str().expect("no name found in realm file").to_string();
        let gamemode_name = yaml["gamemode"].as_str().expect("no gamemode found in realm file").to_string();

        Realm {
            id,
            name,
            gamemode: GameMode::read(gamemode_name),
        }
    }

    pub fn content_folder(&self) -> String {
        format!("realm/{}/", self.id)
    }

    pub fn output_folder(&self) -> String {
        format!("out/{}", self.id)
    }

    pub fn copy_realm_files(&self) {
        let result = fs::read_dir(format!("realm/{}/", self.id));
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

    pub fn build_files(self) {
        fs::create_dir_all(format!("{}/plugins/", self.output_folder())).expect("failed to create plugin output directory");
        self.gamemode.copy_server_files(&self);
        self.gamemode.copy_plugins(&self);
        self.gamemode.copy_gamemode_files(&self);
        self.copy_realm_files();
        self.write_eula();
    }
}
