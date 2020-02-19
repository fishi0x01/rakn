#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate lazy_static;

extern crate clap;
extern crate regex;
extern crate tempdir;
extern crate walkdir;

use crate::scanner::dpkg::{DpkgBinary, DpkgSource};
use clap::{App, Arg};
use std::path::Path;
use tempdir::TempDir;
use walkdir::{DirEntry, WalkDir};
use crate::scanner::python::PythonPackage;

mod docker;
mod report;
mod scanner;

#[derive(Builder, Clone)]
pub struct ScanResult {
    pub dpkg_binary_packages: Vec<DpkgBinary>,
    pub dpkg_source_packages: Vec<DpkgSource>,
    pub python_packages: Vec<PythonPackage>,
}

fn main() {
    let matches = App::new("rakn")
        .version("0.1.0")
        .author("Karl Fischer <fishi0x01@gmail.com>")
        .about("Simple version scanner")
        .arg(
            Arg::with_name("docker_image")
                .short("i")
                .long("docker-image")
                .value_name("IMAGE")
                .help("Which docker image to scan")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dir")
                .short("d")
                .long("dir")
                .value_name("DIR")
                .help("Which dir to scan recursively")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("exclude")
                .short("e")
                .long("exclude-dir")
                .value_name("DIR")
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("TYPE")
                .help("Allowed are 'vulsio' and 'rakn' (default)")
                .default_value("rakn")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("pretty")
                .short("p")
                .long("pretty")
                .takes_value(false),
        )
        .get_matches();

    // ***************
    // Parse arguments
    // ***************
    let docker_image = matches.value_of("docker_image");
    let scan_dir = match matches.value_of("dir") {
        Some(d) => d,
        // Default
        None => "/",
    };
    let excluded_dirs = match matches.values_of("exclude") {
        Some(values) => values.into_iter().collect::<Vec<&str>>(),
        // Defaults
        None => vec!["/dev", "/proc", "/sys"],
    };

    // *********
    // Execution
    // *********
    let tmp_dir_alloc = TempDir::new(env!("CARGO_PKG_NAME")).unwrap();

    // determine scan root
    let scan_root_dir = match docker_image {
        Some(i) => docker::extract_image(i, &tmp_dir_alloc).unwrap(),
        None => "/".to_string(),
    };

    // collect files eligible for scanning in scan root
    let files_to_scan: Vec<DirEntry> = WalkDir::new(format!("{}/{}", scan_root_dir, scan_dir))
        .follow_links(false)
        .into_iter()
        .filter_entry(|d| {
            // TODO: remove from scan_root_dir prefix
            !excluded_dirs.contains(&d.path().to_str().unwrap())
        })
        .filter_map(|v| v.ok())
        .collect();

    // try parsing /var/lib/dpkg/status
    let (dpkg_binary_packages, dpkg_source_packages) =
        match scanner::dpkg::scan(Path::new(scan_root_dir.as_str())) {
            Err(e) => {
                println!("{}", e);
                (vec![], vec![])
            }
            Ok(p) => p,
        };

    // get python libraries
    let python_packages = match scanner::python::scan(&files_to_scan) {
        Err(e) => {
            println!("{}", e);
            vec![]
        },
        Ok(p) => p,
    };

    let scan_result = ScanResultBuilder::default()
        .dpkg_binary_packages(dpkg_binary_packages)
        .dpkg_source_packages(dpkg_source_packages)
        .python_packages(python_packages)
        .build()
        .unwrap();

    report::rakn::print(&scan_result);
}
