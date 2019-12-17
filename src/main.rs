extern crate fs_extra;
extern crate yaml_rust;

use std::env;
use std::fs;
use std::path::Path;

use fs_extra::dir::CopyOptions;
use yaml_rust::{Yaml, YamlLoader};

fn main() {
    let args: Vec<String> = env::args().collect();
    let realm_name = args.get(1).expect(&format!("usage: {} <realm>", args.get(0).unwrap())).clone();

    let realm = Realm::read(realm_name);
    println!("{:#?}", realm);
    realm.build_files();
}

fn read(file: String) -> String {
    fs::read_to_string(file.clone()).expect(&format!("couldn't read file: {}", file))
}

fn read_yaml(file: String) -> Vec<Yaml> {
    let contents = read(file.clone());
    YamlLoader::load_from_str(&contents).expect(&format!("couldn't parse yaml file: {}", file))
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Realm {
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
        let paths = fs::read_dir(format!("realm/{}/", self.id)).unwrap();
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

#[derive(Debug, Clone, Eq, PartialEq)]
struct GameMode {
    id: String,
    name: String,
    server: String,
    version: String,
    backup_versions: Vec<String>,
    plugins: Vec<String>,
}

impl GameMode {
    pub fn read(gamemode_name: String) -> GameMode {
        GameMode::new(gamemode_name.clone(), &read_yaml(format!("gamemode/{}.yml", gamemode_name))[0])
    }

    pub fn new(id: String, yaml: &Yaml) -> GameMode {
        let name = yaml["name"].as_str().expect("expected name field in gamemode").to_string();
        let server = yaml["server"].as_str().expect("expected server field in gamemode").to_string();
        let version = yaml["version"].as_str().expect("expected version field in gamemode").to_string();

        let mut backup_versions = Vec::new();
        for x in yaml["backup-versions"].as_vec().unwrap_or(&Vec::new()) {
            backup_versions.push(x.as_str().expect("expected string in backup-versions").to_string());
        }

        let mut plugins = Vec::new();
        for x in yaml["plugins"].as_vec().unwrap_or(&Vec::new()) {
            plugins.push(x.as_str().expect("expected string in plugins list").to_string());
        }

        GameMode {
            id,
            name,
            server,
            version,
            backup_versions,
            plugins,
        }
    }

    pub fn copy_gamemode_files(&self, realm: &Realm) {
        let paths = fs::read_dir(format!("gamemode/{}/", self.id)).unwrap();
        for path in paths {
            let path = path.expect("error reading server path").path();
            if path.is_file() {
                fs::copy(path.to_str().unwrap(), format!("{}/{}", realm.output_folder(), path.file_name().unwrap().to_str().unwrap()))
                    .expect("failed to copy server files");
            } else if path.is_dir() {
                let mut options = CopyOptions::new();
                options.overwrite = true;
                fs_extra::dir::copy(path, realm.output_folder(), &options).expect("failed to copy gamemode files");
            }
        }
    }

    pub fn copy_server_files(&self, realm: &Realm) {
        let paths = fs::read_dir(format!("server/{}/", self.server)).unwrap();
        for path in paths {
            let path = path.expect("error reading server path").path();
            fs::copy(path.to_str().unwrap(), format!("{}/{}", realm.output_folder(), path.file_name().unwrap().to_str().unwrap()))
                .expect("failed to copy server files");
        }
        // rename the server jar to "server.jar"
        let old_server_name = format!("{}/{}.jar", realm.output_folder(), self.server);
        fs::copy(&old_server_name, format!("{}/server.jar", realm.output_folder())).expect("failed to rename server jar");
        fs::remove_file(old_server_name).expect("failed to remove old server jar");
    }

    pub fn copy_plugins(&self, realm: &Realm) {
        fs::create_dir_all(format!("{}/plugins/", realm.output_folder())).expect("failed to create plugin output directory");
        for ((file, folder), plugin_name) in self.plugin_paths() {
            fs::copy(file, format!("{}/plugins/{}.jar", realm.output_folder(), plugin_name)).expect("failed to copy plugin");
            if let Some(folder) = folder {
                let mut options = CopyOptions::new();
                options.overwrite = true;
                fs_extra::dir::copy(folder, format!("{}/plugins/", realm.output_folder()), &options).expect("failed to copy plugin folder");
            }
        }
    }

    /// Collect all of the plugin paths for this GameMode
    pub fn plugin_paths(&self) -> Vec<((String, Option<String>), String)> {
        let mut paths = Vec::with_capacity(self.plugins.len());
        for plugin in self.plugins.iter() {
            paths.push((self.plugin_path(plugin.clone()).expect(&format!("couldn't find suitable plugin {}", plugin)), plugin.clone()));
        }
        paths
    }

    /// Search for a suitable version of the plugin based on the version of the GameMode.
    /// If there is a folder containing files needed for the plugin, that is returned in
    /// the inner Option: Option<(JarPath, Option<FolderPath>)>
    pub fn plugin_path(&self, name: String) -> Option<(String, Option<String>)> {
        let mut versions = Vec::with_capacity(self.backup_versions.len() + 1);
        versions.push(self.version.clone());
        versions.append(&mut self.backup_versions.clone());

        for version in versions {
            let jar_path = format!("plugins/{}/{}.jar", version, name).to_string();
            let folder_path = format!("plugins/{}/{}/", version, name).to_string();
            let folder = if Path::new(&folder_path).is_dir() { Some(folder_path) } else { None };

            let path = Path::new(&jar_path);
            if path.exists() {
                return Some((jar_path, folder));
            }
        }
        let jar_path = format!("plugins/{}.jar", name).to_string();
        let folder_path = format!("plugins/{}/{}/", self.version, name).to_string();
        let folder = if Path::new(&folder_path).is_dir() { Some(folder_path) } else { None };

        let path = Path::new(&jar_path);
        if path.exists() {
            return Some((jar_path, folder));
        }
        return None;
    }
}
