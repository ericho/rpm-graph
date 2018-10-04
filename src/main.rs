extern crate walkdir;
#[macro_use]
extern crate structopt;
extern crate rayon;
extern crate regex;
extern crate rusted_cypher;

mod rpm;

use rpm::RPM;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::io::{Error, ErrorKind};
use std::process::Command;

use rayon::prelude::*;
use structopt::StructOpt;
use walkdir::WalkDir;

type AtomicGraph = Arc<Mutex<HashMap<String, Vec<String>>>>;

// //\\oO//\\ <- Spider
#[derive(StructOpt, Debug)]
#[structopt(name = "rpmgraph", about = "A tool to create a graph of rpm dependencies.")]
struct Opt {
    // Directory to look for rpms
    #[structopt(short = "d", long = "directory", parse(from_os_str))]
    dir: PathBuf,
}

fn check_prerequisites() -> Result<(), Error> {
    let output = Command::new("which")
        .arg("rpm")
        .output()
        .expect("Cannot run command");
    match output.status.success() {
        true => Ok(()),
        _ => Err(Error::new(ErrorKind::NotFound, "rpm not found")),
    }
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    let mut files = Vec::new();

    check_prerequisites()?;
    println!("Searching for rpms in {:?}...", opt.dir);

    // Get all files. Walkdir doesn't support rayon yet, because
    // of this is needed to get this big list and then iterate
    // over it.
    for entry in WalkDir::new(opt.dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let f_name = entry.file_name().to_string_lossy();
        if f_name.ends_with("src.rpm") {
            files.push(entry.path().to_owned());
        }
    }

    println!("Getting info from rpms...");

    let rpms: AtomicGraph = Arc::new(Mutex::new(HashMap::new()));

    // Get the info from the rpms and store it in a Vec.
    files.par_iter().clone().for_each(|f| {
        let rpm = RPM::new(f).unwrap();
        let mut rpms = rpms.lock().unwrap();
        for i in &rpm.deps {
            if rpms.contains_key(i) {
                let e = rpms.get_mut(i).unwrap();
                e.push(rpm.name.clone());
            } else {
                rpms.insert(i.clone(), vec![rpm.name.clone()]);
            }
        }
    });

    let mut unique = HashMap::new();
    for (a, b) in rpms.lock().unwrap().iter() {
        {
            unique.entry(a.clone()).or_insert(0);
        }
        for x in b {
            unique.entry(x.clone()).or_insert(0);
        }
    }
    Ok(())
}
