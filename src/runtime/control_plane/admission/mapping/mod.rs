mod edge;
mod vertex;

use super::types::GraphAdmissionError;
use lsp_max_lsif::lsif::Element;

pub fn map_element_to_quads(
    element: &Element,
    graph_name: &oxigraph::model::GraphName,
    quads: &mut Vec<oxigraph::model::Quad>,
) -> Result<(), GraphAdmissionError> {
    match element {
        Element::Vertex(v) => vertex::map_vertex(v, graph_name, quads),
        Element::Edge(e) => edge::map_edge(e, graph_name, quads),
    }
}
