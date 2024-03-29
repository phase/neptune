#[macro_use]
extern crate lazy_static;
extern crate fs_extra;
extern crate yaml_rust;

use std::{env, fs, io, thread};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, exit, Stdio};
use std::time::Duration;

use fs_extra::dir::CopyOptions;
use yaml_rust::{Yaml, YamlLoader};
use std::sync::{Arc, Mutex};
use crate::realm::Realm;

mod realm;
mod gamemode;

lazy_static! {
    static ref GEN_LOCK: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let usage = format!("stardust neptune\
    \n- neptune gen <realm>\
    \n- neptune run <realm> <dir>");

    let command = args.get(1).expect(&usage);
    match command.as_ref() {
        "gen" => {
            let realm_name = args.get(2).expect(&usage).clone();
            let _ = generate_realm(realm_name);
        }
        "run" => {
            let realm_name = args.get(2).expect(&usage).clone();
            let run_dir = args.get(3).expect(&usage).clone();
            let realm_name_copy = realm_name.clone();
            let run_dir_copy = run_dir.clone();
            let plugin_dir = format!("{}/plugins/", run_dir);
            thread::spawn(move || {
                loop {
                    let mut lock = GEN_LOCK.lock().unwrap();
                    if !*lock {
                        *lock = true;
                        let _ = generate_realm_run_files(realm_name_copy.clone(), &run_dir_copy.clone(), &plugin_dir);
                        *lock = false;
                    }
                    drop(lock);
                    thread::sleep(Duration::from_secs(10));
                }
            });
            run_server(realm_name.clone(), run_dir);
        }
        _ => fail_with_usage(&usage)
    }
}

pub fn run_server(realm_name: String, run_dir: String) -> ! {
    let plugin_dir = format!("{}/plugins/", run_dir);

    fs::create_dir_all(Path::new(&plugin_dir))
        .expect("failed to create plugin directory in run directory");

    loop {
        let mut lock = GEN_LOCK.lock().unwrap();
        if !*lock {
            *lock = true;
            let (run_dir_full, start_script_path) = generate_realm_run_files(realm_name.clone(), &run_dir, &plugin_dir);

            *lock = false;
            drop(lock);
            println!("Files ready! Starting server...");
            // run start script
            Command::new("sh")
                .arg(&start_script_path)
                .current_dir(&run_dir_full)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .spawn()
                .expect("failed to run start script")
                .wait()
                .expect("failed to wait for server process");
        } else {
            println!("!!! Files were being generated, trying again.");
        }
        for t in (1..4).rev() {
            println!("Server restarting in {} seconds...", t);
            thread::sleep(Duration::from_secs(1));
        }
    }
}

fn generate_realm_run_files(realm_name: String, run_dir: &String, plugin_dir: &String) -> (String, String) {
    let (realm, output_dir) = generate_realm(realm_name);
    let result = fs::read_dir(&plugin_dir);
    if let Ok(dir) = result {
        for entry in dir {
            let entry = entry.expect("couldn't read file in plugin directory");
            if entry.path().to_str().unwrap().ends_with(".jar") {
                fs::remove_file(entry.path()).expect("couldn't remove jar file in plugin directory");
            }
        }
    }
    for entry in fs::read_dir(&output_dir).unwrap() {
        let entry = entry.expect("couldn't read output file");
        let file_type = entry.file_type().expect("failed to get filetype");
        if file_type.is_dir() {
            let mut options = CopyOptions::new();
            options.overwrite = true;
            fs_extra::dir::copy(entry.path(), &run_dir, &options)
                .expect("failed to copy folder from output directory to run directory");
        } else if file_type.is_file() {
            fs::copy(entry.path(), &format!("{}/{}", &run_dir, entry.file_name().to_str().unwrap()))
                .expect("failed to copy file from output directory to run directory");
        }
    };
    let mut replacements = HashMap::new();
    let buf = fs::canonicalize(&PathBuf::from(&run_dir)).unwrap();
    let run_dir_full = buf.to_str().unwrap();
    let server_jar_path = format!("{}/server.jar", run_dir_full);
    let start_script_path = format!("{}/start.sh", run_dir_full);
    replacements.insert("SERVER_JAR".to_string(), server_jar_path);
    realm.make_replacements_in_file(&start_script_path, replacements);
    drop(realm);
    (run_dir_full.to_string(), start_script_path)
}

fn fail_with_usage(usage: &String) -> ! {
    eprintln!("{}", usage);
    exit(1);
}

fn generate_realm(realm_name: String) -> (Realm, String) {
    let realm = Realm::read(realm_name);
    let output_directory = realm.output_folder();
    realm.build_files();

    println!("Generated realm files.");
    (realm, output_directory)
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
