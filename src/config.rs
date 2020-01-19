//! Configuration options for Command line

use failure::bail;
use std::str::FromStr;

pub(crate) enum StrsimAlgorithm {
    Levenshtein,
    DamerauLevenshtein,
    Jaro,
    JaroWinkler,
    NormalizedDamerauLevenshtein,
    NormalizedLevenshtein,
    OsaDistance,
    Hamming,
}

impl StrsimAlgorithm {
    pub fn enumerated() -> &'static [&'static str] {
        &[
            "levenshtein",
            "damerau_levenshtein",
            "jaro",
            "jaro_winkler",
            "normalized_damerau_levenshtein",
            "normalized_levenshtein",
            "osa_distance",
            "hamming",
        ]
    }
}

impl FromStr for StrsimAlgorithm {
    type Err = failure::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "levenshtein" => StrsimAlgorithm::Levenshtein,
            "damerau_levenshtein" => StrsimAlgorithm::DamerauLevenshtein,
            "jaro" => StrsimAlgorithm::Jaro,
            "jaro_winkler" => StrsimAlgorithm::JaroWinkler,
            "normalized_damerau_levenshtein" => StrsimAlgorithm::NormalizedDamerauLevenshtein,
            "normalized_levenshtein" => StrsimAlgorithm::NormalizedLevenshtein,
            "osa_distance" => StrsimAlgorithm::OsaDistance,
            "hamming" => StrsimAlgorithm::Hamming,
            _ => bail!("Could not decode StrsimAlgorithm from string."),
        })
    }
}

pub(crate) enum OutputFormat {
    Gml,
    Csr,
    Json,
}

impl FromStr for OutputFormat {
    type Err = failure::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "gml" => OutputFormat::Gml,
            "csr" => OutputFormat::Csr,
            "json" => OutputFormat::Json,
            _ => bail!("Could not decode output format from string."),
        })
    }
}

impl OutputFormat {
    pub fn enumerated() -> &'static [&'static str] {
        &["gml", "csr", "json"]
    }
}
