use strsim::levenshtein;
use strsim_network::CooMatrix;

fn main() {
    let strings = vec!["AA", "AB", "XX", "XY", "YY", "QQ"];
    let matrix = CooMatrix::from_strings(&strings, 1, 1, &levenshtein);
    let graph = matrix.into_graph(&strings);

    graph.to_gml_pretty(std::io::stdout()).unwrap();
}
