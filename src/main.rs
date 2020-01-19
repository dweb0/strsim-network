use clap::{App, Arg, ArgMatches};
use colored::*;
use failure::bail;
use failure::ResultExt;
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use strsim_network::CooMatrix;

mod config;
use config::{OutputFormat, StrsimAlgorithm};

fn build_app() -> App<'static, 'static> {
    App::new("strsim-network")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author("dweb0")
        .args(&[
            Arg::with_name("strings")
                .index(1)
                .required(true)
                .takes_value(true)
                .help("Line separated list of strings to network. Use - for STDIN"),
            Arg::with_name("algorithm")
                .long("algorithm")
                .short("a")
                .takes_value(true)
                .required(true)
                .possible_values(StrsimAlgorithm::enumerated())
                .help("String similarity algorithm to use"),
            Arg::with_name("min-distance")
                .long("min-distance")
                .short("m")
                .required(true)
                .takes_value(true)
                .help("Nodes must be >= THIS distance to be considered a link"),
            Arg::with_name("max-distance")
                .long("max-distance")
                .short("M")
                .required(true)
                .takes_value(true)
                .help("Nodes must be <= least THIS distance to be considered a link"),
            Arg::with_name("format")
                .long("format")
                .short("f")
                .takes_value(true)
                .required(true)
                .default_value("gml")
                .possible_values(OutputFormat::enumerated())
                .help("Output file format")
        ])
}

fn main() {
    let app = build_app();
    let matches = app.get_matches();

    if let Err(e) = real_main(&matches) {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn real_main(matches: &ArgMatches) -> Result<(), failure::Error> {
    let output_format = matches
        .value_of("format")
        .unwrap()
        .parse::<OutputFormat>()
        .unwrap();

    let algorithm = matches
        .value_of("algorithm")
        .unwrap()
        .parse::<StrsimAlgorithm>()
        .unwrap();

    let min_distance = matches.value_of("min-distance").unwrap();
    let max_distance = matches.value_of("max-distance").unwrap();

    let input: Box<dyn Read> = match matches.value_of("strings").unwrap() {
        "-" => Box::new(std::io::stdin()),
        file => match File::open(file) {
            Ok(file) => Box::new(file),
            Err(e) => bail!("Could not read strings from file. {}", e),
        },
    };

    let mut rdr = BufReader::new(input);
    let mut buffer = String::new();
    rdr.read_to_string(&mut buffer)
        .expect("Problem reading strings to buffer");
    let strings: Vec<_> = buffer.lines().collect();

    // TODO: I'm sure theres better way to do this
    match algorithm {
        StrsimAlgorithm::Levenshtein => {
            let (min, max) = parse_min_and_max_usize(min_distance, max_distance)?;
            let matrix =
                CooMatrix::from_strings_with_progressbar(&strings, min, max, &strsim::levenshtein);
            save_matrix(matrix, &output_format, &strings)
        }
        StrsimAlgorithm::DamerauLevenshtein => {
            let (min, max) = parse_min_and_max_usize(min_distance, max_distance)?;
            let matrix = CooMatrix::from_strings_with_progressbar(
                &strings,
                min,
                max,
                &strsim::damerau_levenshtein,
            );
            save_matrix(matrix, &output_format, &strings)
        }
        StrsimAlgorithm::Jaro => {
            let (min, max) = parse_min_and_max_float(min_distance, max_distance)?;
            let matrix =
                CooMatrix::from_strings_with_progressbar(&strings, min, max, &strsim::jaro);
            save_matrix(matrix, &output_format, &strings)
        }
        StrsimAlgorithm::JaroWinkler => {
            let (min, max) = parse_min_and_max_float(min_distance, max_distance)?;
            let matrix =
                CooMatrix::from_strings_with_progressbar(&strings, min, max, &strsim::jaro_winkler);
            save_matrix(matrix, &output_format, &strings)
        }
        StrsimAlgorithm::NormalizedDamerauLevenshtein => {
            let (min, max) = parse_min_and_max_float(min_distance, max_distance)?;
            let matrix = CooMatrix::from_strings_with_progressbar(
                &strings,
                min,
                max,
                &strsim::normalized_damerau_levenshtein,
            );
            save_matrix(matrix, &output_format, &strings)
        }
        StrsimAlgorithm::NormalizedLevenshtein => {
            let (min, max) = parse_min_and_max_float(min_distance, max_distance)?;
            let matrix = CooMatrix::from_strings_with_progressbar(
                &strings,
                min,
                max,
                &strsim::normalized_levenshtein,
            );
            save_matrix(matrix, &output_format, &strings)
        }
        StrsimAlgorithm::OsaDistance => {
            let (min, max) = parse_min_and_max_usize(min_distance, max_distance)?;
            let matrix =
                CooMatrix::from_strings_with_progressbar(&strings, min, max, &strsim::osa_distance);
            save_matrix(matrix, &output_format, &strings)
        }
        StrsimAlgorithm::Hamming => {
            let (min, max) = parse_min_and_max_usize(min_distance, max_distance)?;
            let matrix = CooMatrix::from_strings_with_progressbar(&strings, min, max, &hamming);
            save_matrix(matrix, &output_format, &strings)
        }
    }
    .with_context(|e| format!("Encountered a problem while saving graph. {}", e))?;

    Ok(())
}

/// Save matrix to the specified output format
///
/// TODO: Adding extra overhead by requiring T to implement Display,
/// when that is only needed with `into_graph`.
fn save_matrix<T>(
    coo_matrix: CooMatrix<T>,
    output_format: &OutputFormat,
    strings: &[&str],
) -> Result<(), failure::Error>
where
    T: Serialize + std::fmt::Display,
{
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    match output_format {
        OutputFormat::Gml => {
            let graph = coo_matrix.into_graph(&strings);
            // Since gml is many small writes, better to use BufWriter
            let mut writer = BufWriter::new(&mut stdout);
            graph.to_gml_pretty(&mut writer)
        }
        OutputFormat::Csr => {
            let csr_matrix = coo_matrix.into_csr_matrix(strings.len());
            csr_matrix.to_json(&mut stdout)
        }
        OutputFormat::Json => {
            let graph = coo_matrix.into_graph(&strings);
            graph.to_node_link_json(&mut stdout)
        }
    }
}

/// Parse MIN and MAX distance as usize and ensure they
/// form a valid range
fn parse_min_and_max_usize(min: &str, max: &str) -> Result<(usize, usize), failure::Error> {
    let min = min
        .parse::<usize>()
        .with_context(|e| format!("Could not parse uint for MIN distance. {}", e))?;
    let max = max
        .parse::<usize>()
        .with_context(|e| format!("Could not parse uint for MAX distance. {}", e))?;

    if min > max {
        bail!("MIN distance cannot be greater than MAX distance");
    }

    Ok((min, max))
}

/// Parse MIN and MAX distance as float and ensure they
/// form a valid range
fn parse_min_and_max_float(min: &str, max: &str) -> Result<(f64, f64), failure::Error> {
    let min = min
        .parse::<f64>()
        .with_context(|e| format!("Could not parse float for MIN distance. {}", e))?;
    let max = max
        .parse::<f64>()
        .with_context(|e| format!("Could not parse float for MAX distance. {}", e))?;

    if min < 0.0 || min > 1.0 {
        bail!("For this algorithm, MIN distance must be between 0.0 and 1.0 (inclusive).")
    }

    if max < 0.0 || max > 1.0 {
        bail!("For this algorithm, MAX distance must be between 0.0 and 1.0 (inclusive).")
    }

    if min > max {
        bail!("MIN distance cannot be greater than MAX distance");
    }

    Ok((min, max))
}

// Wrappper for hamming that handles the error where
// two strings are not the same size
fn hamming(a: &str, b: &str) -> usize {
    strsim::hamming(a, b).unwrap_or(std::usize::MAX)
}
