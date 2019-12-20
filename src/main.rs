extern crate fs_extra;
extern crate yaml_rust;

use std::{env, fs, io, thread};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, exit, Stdio};
use std::time::Duration;

use fs_extra::dir::CopyOptions;
use yaml_rust::{Yaml, YamlLoader};

use crate::realm::Realm;

mod realm;
mod gamemode;

fn main() {
    let args: Vec<String> = env::args().collect();
    let usage = format!("stardust neptune\
    \n- neptune gen <realm>\
    \n-neptune run <realm> <dir>");

    let command = args.get(1).expect(&usage);
    match command.as_ref() {
        "gen" => {
            let realm_name = args.get(2).expect(&usage).clone();
            let _ = generate_realm(realm_name);
        }
        "run" => {
            let realm_name = args.get(2).expect(&usage).clone();
            let run_dir = args.get(3).expect(&usage).clone();
            run_server(realm_name, run_dir);
        }
        _ => fail_with_usage(&usage)
    }
}

pub fn run_server(realm_name: String, run_dir: String) -> ! {
    let plugin_dir = format!("{}/plugins/", run_dir);

    fs::create_dir_all(Path::new(&plugin_dir))
        .expect("failed to create plugin directory in run directory");

    loop {
        let (realm, output_dir) = generate_realm(realm_name.clone());

        println!("Removing old jars from run directory.");
        let result = fs::read_dir(&plugin_dir);
        if let Ok(dir) = result {
            for entry in dir {
                let entry = entry.expect("couldn't read file in plugin directory");
                if entry.path().to_str().unwrap().ends_with(".jar") {
                    println!("- {}", entry.path().to_str().unwrap());
                    fs::remove_file(entry.path()).unwrap();
                }
            }
        }

        println!("Copying generated realm files to run directory.");
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

        println!("Generating start script.");
        let mut replacements = HashMap::new();
        let buf = fs::canonicalize(&PathBuf::from(&run_dir)).unwrap();
        let run_dir_full = buf.to_str().unwrap();
        let server_jar_path = format!("{}/server.jar", run_dir_full);
        let start_script_path = format!("{}/start.sh", run_dir_full);
        replacements.insert("SERVER_JAR".to_string(), server_jar_path);
        realm.make_replacements_in_file(&start_script_path, replacements);
        drop(realm);

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

        for t in (1..4).rev() {
            println!("Server restarting in {} seconds...", t);
            thread::sleep(Duration::from_secs(1));
        }
    }
}

fn fail_with_usage(usage: &String) -> ! {
    eprintln!("{}", usage);
    exit(1);
}

fn generate_realm(realm_name: String) -> (Realm, String) {
    let realm = Realm::read(realm_name);
    let output_directory = realm.output_folder();
    println!("{:#?}\n", realm);
    realm.build_files();

    println!("Created files:");
    visit_dirs(output_directory.as_ref(), &|path| {
        println!("- {}", path);
    }).unwrap();
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
