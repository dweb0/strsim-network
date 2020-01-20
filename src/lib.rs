use rayon::prelude::*;
use serde::Serialize;
use std::io::Write;


/// Coordinate matrix, which contains the coordinate (row, col) and
/// value for each link
#[derive(Debug)]
pub struct CooMatrix<T>(Vec<Coordinate<T>>);

impl CooMatrix<u32> {

    #[cfg(feature = "cli")]
    pub fn from_strings_levenshtein_fst_progressbar(
        strings: &[&str],
        max_dist: u32
    ) -> Self
    {

        use fst::{IntoStreamer, Map};
        use fst_levenshtein::Levenshtein;

        let pb = indicatif::ProgressBar::new(strings.len() as u64);

        let result: Vec<_> = (0..strings.len())
            .into_par_iter()
            .flat_map(|row| {
                pb.inc(1);
                let lev = Levenshtein::new(&strings[row], max_dist).unwrap();
                let iter = strings.iter().take(row).enumerate().map(|(i, j)| (j, i as u64));
                let map = Map::from_iter(iter).unwrap();
                let stream = map.search(&lev).into_stream();
                
                stream.into_values().into_par_iter().map(move |col| Coordinate { row, col: col as usize, value: max_dist })
            })
            .collect();

        pb.finish_and_clear();

        Self(result)
    }
}

impl<T> CooMatrix<T> {
    /// Find all combinations of strings within a specified
    /// acceptable distance and return a new Coordinate Matrix
    pub fn from_strings<F>(strings: &[&str], min_dist: T, max_dist: T, distance_func: &F) -> Self
    where
        T: Sync + Send + PartialOrd + Copy,
        F: Sync + Send + Fn(&str, &str) -> T,
    {
        let result: Vec<_> = (0..strings.len())
            .into_par_iter()
            .flat_map(|row| {
                (0..row).into_par_iter().filter_map(move |col| {
                    let dist = distance_func(&strings[row], &strings[col]);
                    if dist >= min_dist && dist <= max_dist {
                        Some(Coordinate {
                            row,
                            col,
                            value: dist,
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();
        Self(result)
    }

    /// Find all combinations of strings within a specified
    /// acceptable distance and return a new Coordinate Matrix
    ///
    /// This function includes a progressbar using indicatif libary
    #[cfg(feature = "cli")]
    pub fn from_strings_with_progressbar<F>(
        strings: &[&str],
        min_dist: T,
        max_dist: T,
        distance_func: &F,
    ) -> Self
    where
        T: Sync + Send + PartialOrd + Copy,
        F: Sync + Send + Fn(&str, &str) -> T,
    {
        let pb = indicatif::ProgressBar::new(strings.len() as u64);

        let result: Vec<_> = (0..strings.len())
            .into_par_iter()
            .flat_map(|row| {
                pb.inc(1);
                (0..row).into_par_iter().filter_map(move |col| {
                    let dist = distance_func(&strings[row], &strings[col]);
                    if dist >= min_dist && dist <= max_dist {
                        Some(Coordinate {
                            row,
                            col,
                            value: dist,
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        pb.finish_and_clear();

        Self(result)
    }

    /// Convert this CsrMatrix to a Graph
    ///
    /// TODO: Check that I got link accurate
    pub fn into_graph<'a>(self, strings: &'a [&'a str]) -> Graph<T> {
        let n_row = strings.len();

        let nodes: Vec<_> = (0..n_row)
            .map(|i| Node {
                id: i,
                label: &strings[i],
            })
            .collect();

        let links: Vec<_> = self
            .0
            .into_iter()
            .map(|c| Link {
                source: c.row,
                target: c.col,
                weight: c.value,
            })
            .collect();

        Graph { nodes, links }
    }

    /// Convert this coordinate matrix into a compressed
    /// sparse matrix (CSR)
    /// Based off of scipy's implementation:
    /// https://github.com/scipy/scipy/blob/v1.4.1/scipy/sparse/sparsetools/coo.h
    pub fn into_csr_matrix(self, n_row: usize) -> CsrMatrix<T> {
        let mut row_ptr = vec![0; n_row + 1];
        let nnz = self.0.len();
        for i in 0..nnz {
            row_ptr[self.0[i].row] += 1;
        }
        let mut cumsum = 0;
        for elem in row_ptr.iter_mut().take(n_row) {
            let tmp = *elem;
            *elem = cumsum;
            cumsum += tmp;
        }
        row_ptr[n_row] = nnz;
        let mut values = Vec::with_capacity(nnz);
        let mut col_indices = Vec::with_capacity(nnz);
        for coordinate in self.0 {
            values.push(coordinate.value);
            col_indices.push(coordinate.col);
        }
        CsrMatrix {
            n_row,
            values,
            col_indices,
            row_ptr,
        }
    }
}

#[derive(Debug)]
struct Coordinate<T> {
    row: usize,
    col: usize,
    value: T,
}

#[derive(Debug, Serialize)]
pub struct Graph<'a, T> {
    nodes: Vec<Node<'a>>,
    links: Vec<Link<T>>,
}

impl<'a, T> Graph<'a, T>
where
    T: Serialize,
{
    /// Writes a json with fields for nodes and links. Useful
    /// as input data for d3's forceSimulation
    pub fn to_node_link_json(&self, mut writer: impl Write) -> Result<(), failure::Error> {
        Ok(serde_json::to_writer(&mut writer, &self)?)
    }
}

impl<'a, T> Graph<'a, T>
where
    T: std::fmt::Display,
{
    pub fn to_gml_pretty(&self, mut writer: impl Write) -> Result<(), failure::Error> {
        writeln!(&mut writer, "graph [\n  multigraph 0")?;
        for node in &self.nodes {
            writeln!(
                &mut writer,
                "  node [\n    id {}\n    label \"{}\"\n  ]",
                node.id, node.label
            )?;
        }
        for link in &self.links {
            writeln!(
                &mut writer,
                "  edge [\n    source {}\n    target {}\n    weight {}\n  ]",
                link.source, link.target, link.weight
            )?;
        }
        writeln!(&mut writer, "]")?;
        Ok(())
    }

    /// Writes this matrix to the gml format
    ///
    /// TODO: Should probably use serde here and derive a new
    /// GML object for each link and node
    pub fn to_gml(&self, mut writer: impl Write) -> Result<(), failure::Error> {
        write!(&mut writer, "graph[multigraph 0 ")?;
        for node in &self.nodes {
            write!(&mut writer, "node[id {} label \"{}\"]", node.id, node.label)?;
        }
        for link in &self.links {
            write!(
                &mut writer,
                "edge[source {} target {} weight {}]",
                link.source, link.target, link.weight
            )?;
        }
        write!(&mut writer, "]")?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct Node<'a> {
    id: usize,
    label: &'a str,
}

#[derive(Debug, Serialize)]
pub struct Link<T> {
    source: usize,
    target: usize,
    weight: T,
}

#[derive(Debug, Serialize)]
pub struct CsrMatrix<T> {
    n_row: usize,
    values: Vec<T>,
    col_indices: Vec<usize>,
    row_ptr: Vec<usize>,
}

impl<T> CsrMatrix<T>
where
    T: Serialize,
{
    /// Writes each field in this CsrMatrix as a json
    pub fn to_json(&self, mut writer: impl Write) -> Result<(), failure::Error> {
        Ok(serde_json::to_writer(&mut writer, &self)?)
    }
}
