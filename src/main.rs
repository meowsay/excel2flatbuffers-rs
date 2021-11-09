use std::time::Instant;
// use excel2flatbuffers_rs::UnlockLvConfig_generated;
// use excel2flatbuffers_rs::file_filter;
// use std::path::PathBuf;
use excel2flatbuffers_rs::data::RawTable;
use excel2flatbuffers_rs::file_filter;
// use std::io;
// use std::io::prelude::*;
// use std::fs::File;
use excel2flatbuffers_rs::fbs2code;

extern crate flatbuffers;
use std::fs;
use std::thread;

extern crate clap;
use clap::{App, Arg};

fn main() -> Result<(), std::io::Error> {
    let matches = App::new("My Super Program")
        .version("1.0")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Does awesome things")
        .arg(
            Arg::with_name("lang")
                .short("lang")
                .long("lang")
                .value_name("lang")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("excel")
                .short("excel")
                .long("excel")
                .value_name("excel")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("fbs")
                .short("fbs")
                .long("fbs")
                .value_name("fbs")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bytes")
                .short("bytes")
                .long("bytes")
                .value_name("bytes")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("code")
                .short("code")
                .long("code")
                .value_name("code")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("namespace")
                .short("namespace")
                .long("namespace")
                .value_name("namespace")
                .takes_value(true),
        )
        .get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let namespace = matches.value_of("namespace").unwrap_or("");
    let lang = matches.value_of("lang").unwrap_or("csharp");
    let fbs_dir = matches.value_of("fbs").unwrap_or(""); //" ./common/fbs/"
    let bytes_dir = matches.value_of("bytes").unwrap_or(""); // "./common/data_output/"
    let excel_dir = matches.value_of("excel").unwrap_or(""); // "./common/excels/"
    let lang_code_dir = matches.value_of("code").unwrap_or(""); // "./common/csharp_output/"

    // Create Directories
    if fbs_dir != "" {
        fs::create_dir_all(fbs_dir)?;
    }

    if bytes_dir != "" {
        fs::create_dir_all(bytes_dir)?;
    }

    if lang_code_dir != "" {
        fs::create_dir_all(lang_code_dir)?;
    }

    if excel_dir != "" && fbs_dir != "" && bytes_dir != "" && lang_code_dir != "" {
        process_excel_and_fbs_things(excel_dir, fbs_dir, bytes_dir, namespace);
    }

    let now = Instant::now();
    // Generate Bytes file
    if fbs_dir != "" && lang_code_dir != "" && lang != "" {
        fbs2code::generate(&fbs_dir, &lang_code_dir, &lang)?;
    } else {
        println!("ERROR: 确实必要参数，无法执行生成!");
    }
    println!("Genrate Target Code: {}", now.elapsed().as_secs_f32());

    Ok(())
}

fn process_excel_and_fbs_things(excel_dir: &str, fbs_dir: &str, bytes_dir: &str, namespace: &str) {
    let file_identifier = Some("WHAT");

    // Get all excels
    let excel_path_vec = file_filter::get_all_files(excel_dir, "xlsx", false);

    // Start thread to process every excel
    let mut thread_vec = Vec::new();
    for excel_file in excel_path_vec.iter() {
        let excel_path = String::from(excel_file.to_str().unwrap());
        let fbs_path = String::from(fbs_dir);
        let bytes_path = String::from(bytes_dir);
        let fbs_namespace = String::from(namespace);
        thread_vec.push(thread::spawn(move || {
            let table = RawTable::new(&excel_path, &fbs_namespace);
            table.write_to_fbs_file(&fbs_path).unwrap();
            table.pack_data(&bytes_path, file_identifier).unwrap();
        }));
    }

    // wait excel process
    for child in thread_vec {
        let _ = child.join();
    }
}
