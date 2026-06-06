use lsp_types_max::SymbolKind;
use oxigraph::model::Term;
use oxigraph::sparql::{QueryResults, QuerySolution, SparqlEvaluator};
use oxigraph::store::Store;

pub fn term_to_string(term: &Term) -> String {
    match term {
        Term::NamedNode(n) => n.as_str().to_string(),
        Term::BlankNode(b) => b.as_str().to_string(),
        Term::Literal(l) => l.value().to_string(),
    }
}

pub fn term_to_u32(term: &Term) -> u32 {
    term_to_string(term).parse().unwrap_or(0)
}

pub fn run_select(store: &Store, query: &str) -> Result<Vec<QuerySolution>, String> {
    let evaluator = SparqlEvaluator::new();
    let parsed = evaluator.parse_query(query).map_err(|e| e.to_string())?;
    match parsed
        .on_store(store)
        .execute()
        .map_err(|e| e.to_string())?
    {
        QueryResults::Solutions(solutions) => {
            let mut results = Vec::new();
            for sol in solutions {
                results.push(sol.map_err(|e| e.to_string())?);
            }
            Ok(results)
        }
        _ => Err("Expected solutions query result".to_string()),
    }
}

pub fn parse_symbol_kind(s: &str) -> SymbolKind {
    match s.to_lowercase().as_str() {
        "file" => SymbolKind::FILE,
        "module" => SymbolKind::MODULE,
        "namespace" => SymbolKind::NAMESPACE,
        "package" => SymbolKind::PACKAGE,
        "class" => SymbolKind::CLASS,
        "method" => SymbolKind::METHOD,
        "property" => SymbolKind::PROPERTY,
        "field" => SymbolKind::FIELD,
        "constructor" => SymbolKind::CONSTRUCTOR,
        "enum" => SymbolKind::ENUM,
        "interface" => SymbolKind::INTERFACE,
        "function" => SymbolKind::FUNCTION,
        "variable" => SymbolKind::VARIABLE,
        "constant" => SymbolKind::CONSTANT,
        "string" => SymbolKind::STRING,
        "number" => SymbolKind::NUMBER,
        "boolean" => SymbolKind::BOOLEAN,
        "array" => SymbolKind::ARRAY,
        "object" => SymbolKind::OBJECT,
        "key" => SymbolKind::KEY,
        "null" => SymbolKind::NULL,
        "enummember" | "enum_member" => SymbolKind::ENUM_MEMBER,
        "struct" => SymbolKind::STRUCT,
        "event" => SymbolKind::EVENT,
        "operator" => SymbolKind::OPERATOR,
        "typeparameter" | "type_parameter" => SymbolKind::TYPE_PARAMETER,
        _ => SymbolKind::FUNCTION,
    }
}
