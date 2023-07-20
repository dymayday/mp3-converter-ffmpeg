use std::{ffi::OsStr, fs, path::Path, sync::RwLock};

use clap::Parser;
use log::*;
use rayon::prelude::*;

pub static mut COUNTER: usize = 0;

fn gather_files(path: &str) -> Vec<String> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    dirs.push(path.into());

    while let Some(dir) = dirs.pop() {
        match fs::read_dir(dir) {
            Ok(read_dir) => {
                for entry in read_dir {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_dir() {
                                dirs.push(path);
                            } else {
                                files.push(path.to_str().unwrap().to_string());
                            }
                        },
                        Err(e) => {
                            error!("Error reading directory entry: {}", e);
                        },
                    }
                }
            },
            Err(e) => {
                error!("Error reading directory: {}", e);
            },
        }
    }
    files
}

/// Convert a list of files into a mp3 using ffmpeg.
fn par_convert_files(root_in: &str, root_out: &str, files: &[String], skip: bool) {
    let mut fout_list: Vec<String> = Vec::new();
    for fin in files.iter() {
        let mut fout = fin.replace(root_in, root_out);

        // let ext = fin.split('.').last().unwrap();
        if let Some(ext) = Path::new(&fout).extension().and_then(OsStr::to_str) {
            fout = fout.replace(ext, "mp3");
        }
        // info!("{} -> {}", fin, fout);

        // Create the output directory if it doesn't exist.
        let out_dir = Path::new(&fout).parent().unwrap();
        if !out_dir.exists() {
            fs::create_dir_all(out_dir).unwrap();
        }

        fout_list.push(fout);
    }

    let list: Vec<(&String, &String)> = files.iter().zip(fout_list.iter()).collect();
    let max_len = list.len();

    let counter = RwLock::new(0);

    let _c = list
        .par_iter()
        .map(move |(fin, fout)| {
            let counter: i32 = {
                let mut counter = counter.write().unwrap();
                *counter += 1;
                // info!("[{:>4} / {}] : {}", *counter, max_len, fout_name);
                *counter
            };
            // let fin_name = Path::new(fin).file_name().unwrap().to_str().unwrap();
            let fout_path = Path::new(fout);
            let fout_name = fout_path.file_name().unwrap().to_str().unwrap();

            if fout_path.exists() && skip {
                warn!(
                    "[{:>4} / {}] : {} <** skipped **>",
                    counter, max_len, fout_name
                );
            } else {
                let mut cmd = std::process::Command::new("ffmpeg");
                cmd.arg("-i").arg(fin).arg(fout).arg("-nostdin");

                match cmd.output() {
                    Ok(output) => {
                        if !output.status.success() {
                            // error!(
                            //     "Error converting file: {}",
                            //     String::from_utf8_lossy(&output.stderr)
                            // );
                            error!("[{:>4} / {}] : {}", counter, max_len, fout_name);
                        } else if true {
                            info!("[{:>4} / {}] : {}", counter, max_len, fout_name);
                        }
                    },
                    Err(e) => {
                        error!("Error running ffmpeg: {}", e);
                    },
                }
            }
        })
        .count();

    info!("");
    info!("Converted {} files", max_len);
    info!("");
}


// Handle command line arguments using Clap.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input directory
    #[arg(short, long)]
    input: String,

    /// Output directory
    #[arg(short, long)]
    output: String,

    /// Skip the conversion of files that already exist in the output directory.
    #[arg(short, long)]
    skip: bool,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    let root = args.input;
    let files = gather_files(&root);
    par_convert_files(&root, &args.output, &files, args.skip);
}
