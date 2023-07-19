#![allow(dead_code, unused_variables)]

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
fn par_convert_files(root_in: &str, root_out: &str, files: &[String], chunking: usize) {
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
    let mut counter = 0;
    let max_len = list.len();

    if chunking > 0 {
        for chunk in list.chunks(chunking) {
            let c = chunk
                .to_vec()
                .par_iter()
                .map(|(fin, fout)| {
                    let fin_name = Path::new(fin).file_name().unwrap().to_str().unwrap();
                    let fout_name = Path::new(fout).file_name().unwrap().to_str().unwrap();
                    info!("{} -> {}", fin_name, fout_name);

                    if true {
                        let mut cmd = std::process::Command::new("ffmpeg");
                        cmd.arg("-i").arg(fin).arg(fout);
                        match cmd.output() {
                            Ok(output) => {
                                if !output.status.success() {
                                    error!(
                                        "Error converting file: {}",
                                        String::from_utf8_lossy(&output.stderr)
                                    );
                                }
                            },
                            Err(e) => {
                                error!("Error running ffmpeg: {}", e);
                            },
                        }
                    }
                })
                .count();
            counter += c;
            info!("");
            info!("Converted {}/{} files", counter, max_len);
            info!("");
        }
    } else {
        let counter = RwLock::new(0);

        let _c = list
            .par_iter()
            .map(move |(fin, fout)| {
                let fin_name = Path::new(fin).file_name().unwrap().to_str().unwrap();
                let fout_name = Path::new(fout).file_name().unwrap().to_str().unwrap();
                // info!("{} -> {}", fin_name, fout_name);

                {
                    let mut counter = counter.write().unwrap();
                    *counter += 1;
                    info!(
                        "[{:>4} / {}] :: {} -> {}",
                        *counter, max_len, fin_name, fout_name
                    );
                }

                if true {
                    let mut cmd = std::process::Command::new("ffmpeg");
                    cmd.arg("-i").arg(fin).arg(fout);
                    match cmd.output() {
                        Ok(output) => {
                            if !output.status.success() {
                                // error!(
                                //     "Error converting file: {}",
                                //     String::from_utf8_lossy(&output.stderr)
                                // );
                            }
                        },
                        Err(e) => {
                            // error!("Error running ffmpeg: {}", e);
                        },
                    }
                }
            })
            .count();

        info!("");
        info!("Converted {} files", max_len);
        info!("");
    }
}

/// Same as par_convert_files but using async tokio.
async fn async_convert_files(root_in: &str, root_out: &str, files: &Vec<String>) {
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
    let mut counter = 0;
    let max_len = list.len();

    // let mut childs = Vec::new();

    // Using tokio::spawn
    for (fin, fout) in list.into_iter() {
        let fin_name = Path::new(fin).file_name().unwrap().to_str().unwrap();
        let fout_name = Path::new(fout).file_name().unwrap().to_str().unwrap();
        info!("{} -> {}", fin_name, fout_name);

        let mut child = tokio::process::Command::new("ffmpeg")
            .arg("-i")
            .arg(fin)
            .arg(fout)
            .spawn()
            .expect("failed to spawn command");

        tokio::spawn(async move {
            let _status = child
                .wait()
                .await
                .expect("child process encountered an error");

            counter += 1;
            info!("Converted {}/{} files", counter, max_len);
        });

        // let _: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move
        // {
        //
        //     tx.send(response).await?;
        //     Ok(())
        // });
    }
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

    /// Use async tokio
    #[arg(short, long)]
    tokio: bool,

    /// Number of files to convert at once
    #[arg(short, long, default_value = "0")]
    chunking: usize,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    let root = args.input;

    let files = gather_files(&root);

    if args.tokio {
        info!("Using tokio");
        async_convert_files(&root, &args.output, &files).await;
    } else {
        par_convert_files(&root, &args.output, &files, args.chunking);
    }
}
