use crate::file_filter;
use std::fs;
/// 从 fbs 文件生成目标代码
///
use std::path::Path;
extern crate flatc_rust;

fn _generate(output_path: &str, path_array: &[&Path], lang: &str) -> Result<(), std::io::Error> {
    println!(
        "xxxxxfbs2code Gen Code: {}, 111path_arry_len: {}, path_arry: {:?}",
        output_path,
        path_array.len(),
        path_array
    );
    flatc_rust::run(flatc_rust::Args {
        lang,               // `rust` is the default, but let's be explicit
        inputs: path_array, // &[Path::new("./flatbuffers/monster.fbs")],
        out_dir: Path::new(output_path),
        ..Default::default()
    })?;

    Ok(())
}

pub fn generate(fbs_dir: &str, output_dir: &str, lang: &str) -> Result<(), std::io::Error> {
    if !Path::new(output_dir).is_dir() {
        fs::create_dir(output_dir)?;
    }

    let file_paths = file_filter::get_all_files(fbs_dir, "fbs", false);
    let mut path_vec: Vec<&Path> = Vec::new();

    for file_path in &file_paths {
        let path = file_path.as_path();
        path_vec.push(path);
    }

    let path_array = &path_vec[..];

    _generate(output_dir, path_array, lang)?;

    Ok(())
}
