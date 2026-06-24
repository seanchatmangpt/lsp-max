use crate::lsif::Element;
use oxigraph::model::*;
use oxigraph::store::Store;
use std::io;

pub struct LsifStore {
    store: Store,
    graph_name: GraphName,
}

impl LsifStore {
    pub fn new() -> io::Result<Self> {
        let store = Store::new().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let graph_name = GraphName::DefaultGraph;
        Ok(Self { store, graph_name })
    }

    pub fn insert_element(&mut self, element: &Element) -> io::Result<()> {
        let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let lsif_ns = "lsif:";

        let v = serde_json::to_value(element)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let is_edge = matches!(element, Element::Edge(_));
        
        let id = v.get("id").and_then(|i| {
            if let Some(n) = i.as_i64() {
                Some(format!("{}", n))
            } else if let Some(s) = i.as_str() {
                Some(s.to_string())
            } else {
                None
            }
        }).unwrap_or_else(|| "unknown".to_string());
        
        let label = v.get("label").and_then(|l| l.as_str()).unwrap_or("unknown");

        let id_str = if is_edge {
            format!("urn:lsif:e:{}", id)
        } else {
            format!("urn:lsif:v:{}", id)
        };
        let subject = NamedNode::new(id_str).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let label_node = NamedNode::new(format!("{}{}", lsif_ns, label))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
        self.store.insert(&Quad::new(
            subject.clone(),
            rdf_type.clone(),
            label_node.clone(),
            self.graph_name.clone(),
        )).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        if is_edge {
            let out_v = v.get("outV").and_then(|i| {
                if let Some(n) = i.as_i64() { Some(format!("{}", n)) } else if let Some(s) = i.as_str() { Some(s.to_string()) } else { None }
            });
            let in_v = v.get("inV").and_then(|i| {
                if let Some(n) = i.as_i64() { Some(format!("{}", n)) } else if let Some(s) = i.as_str() { Some(s.to_string()) } else { None }
            });
            let in_vs = v.get("inVs").and_then(|i| i.as_array());

            if let Some(out) = out_v {
                let out_node = NamedNode::new(format!("urn:lsif:v:{}", out))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                if let Some(inv) = in_v {
                    let in_node = NamedNode::new(format!("urn:lsif:v:{}", inv))
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    self.store.insert(&Quad::new(
                        out_node.clone(),
                        label_node.clone(),
                        in_node.clone(),
                        self.graph_name.clone(),
                    )).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                }

                if let Some(invs) = in_vs {
                    for inv in invs {
                        let inv_str = if let Some(n) = inv.as_i64() { format!("{}", n) } else if let Some(s) = inv.as_str() { s.to_string() } else { continue };
                        let in_node = NamedNode::new(format!("urn:lsif:v:{}", inv_str))
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                        self.store.insert(&Quad::new(
                            out_node.clone(),
                            label_node.clone(),
                            in_node.clone(),
                            self.graph_name.clone(),
                        )).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn dump_to_turtle(&self, path: &std::path::Path) -> io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.store.dump_graph_to_writer(
            &self.graph_name,
            oxigraph::io::RdfFormat::Turtle,
            &mut file,
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }

    pub fn execute_sparql(&self, query_str: &str) -> Result<Vec<serde_json::Value>, String> {
        use oxigraph::sparql::QueryResults;
        
        let results = self.store.query(query_str).map_err(|e| e.to_string())?;
        
        let mut json_results = Vec::new();
        
        if let QueryResults::Solutions(solutions) = results {
            for solution in solutions {
                let solution = solution.map_err(|e| e.to_string())?;
                let mut map = serde_json::Map::new();
                for (var, term) in solution.iter() {
                    let term_str = match term {
                        oxigraph::model::Term::NamedNode(n) => n.as_str().to_string(),
                        oxigraph::model::Term::BlankNode(b) => b.as_str().to_string(),
                        oxigraph::model::Term::Literal(l) => l.value().to_string(),
                        _ => term.to_string(),
                    };
                    map.insert(var.as_str().to_string(), serde_json::Value::String(term_str));
                }
                json_results.push(serde_json::Value::Object(map));
            }
        }
        
        Ok(json_results)
    }

    #[allow(deprecated)]
    pub fn query(&self, query_str: &str) -> Result<oxigraph::sparql::QueryResults<'_>, oxigraph::sparql::QueryEvaluationError> {
        self.store.query(query_str)
    }
}
