use std::{
    env, fs, panic,
    path::{Path, PathBuf},
    time::Instant,
};

use clap::Parser;
use glob::glob;
use leptosfmt_formatter::{format_file, FormatterSettings};
use rayon::{iter::ParallelIterator, prelude::IntoParallelIterator};

/// A formatter for Leptos RSX sytnax
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// A file, directory or glob
    input_pattern: String,

    // Maximum width of each line
    #[arg(short, long)]
    max_width: Option<usize>,

    // Number of spaces per tab
    #[arg(short, long)]
    tab_spaces: Option<usize>,

    // Config file
    #[arg(short, long)]
    config_file: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let settings = match settings(&args) {
        Ok(settings) => settings,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    let is_dir = fs::metadata(&args.input_pattern)
        .map(|meta| meta.is_dir())
        .unwrap_or(false);

    let glob_pattern = if is_dir {
        format!("{}/**/*.rs", &args.input_pattern)
    } else {
        args.input_pattern
    };

    let file_paths: Vec<_> = glob(&glob_pattern)
        .expect("failed to read glob pattern")
        .collect();

    let total_files = file_paths.len();
    let start_formatting = Instant::now();
    file_paths.into_par_iter().for_each(|result| {
        let print_err = |path: &Path, err| {
            println!("❌ {}", path.display());
            eprintln!("\t\t{}", err);
        };

        match result {
            Ok(path) => match format_glob_result(&path, settings) {
                Ok(_) => println!("✅ {}", path.display()),
                Err(err) => print_err(&path, &err.to_string()),
            },
            Err(err) => print_err(err.path(), &err.error().to_string()),
        };
    });
    let end_formatting = Instant::now();
    println!(
        "Formatted {} files in {} ms",
        total_files,
        (end_formatting - start_formatting).as_millis()
    )
}

fn settings(args: &Args) -> anyhow::Result<FormatterSettings> {
    let mut settings: FormatterSettings =
        if let Some(config_file) = args.config_file.clone().or_else(find_config) {
            fs::read_to_string(config_file).map(|s| toml::from_str(&s))??
        } else {
            FormatterSettings::default()
        };

    if let Some(max_width) = args.max_width {
        settings.max_width = max_width;
    }

    if let Some(tab_spaces) = args.tab_spaces {
        settings.tab_spaces = tab_spaces;
    }

    Ok(settings)
}

fn find_config() -> Option<PathBuf> {
    let mut path: PathBuf = env::current_dir().ok()?;
    let file = Path::new("leptosfmt.toml");

    loop {
        path.push(file);

        if path.is_file() {
            println!("Discovered config at {}", path.display());
            break Some(path);
        }

        if !(path.pop() && path.pop()) {
            break None;
        }
    }
}

fn format_glob_result(file: &PathBuf, settings: FormatterSettings) -> anyhow::Result<()> {
    let formatted = panic::catch_unwind(|| format_file(file, settings))
        .map_err(|e| anyhow::anyhow!(e.downcast::<String>().unwrap()))??;
    fs::write(file, formatted)?;
    Ok(())
}
