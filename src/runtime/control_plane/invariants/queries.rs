pub const QUERY_INVARIANT_1: &str = "
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
ASK {
  {
    ?s ?p ?o .
    FILTER(STRSTARTS(STR(?p), \"https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/\"))
    FILTER(isIRI(?o) || isBlank(?o))
    FILTER NOT EXISTS { ?o ?any_p ?any_o }
  }
  UNION
  {
    GRAPH ?g {
      ?s ?p ?o .
      FILTER(STRSTARTS(STR(?p), \"https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/\"))
      FILTER(isIRI(?o) || isBlank(?o))
      FILTER NOT EXISTS { ?o ?any_p ?any_o }
    }
  }
}
";

pub const QUERY_INVARIANT_1_SELECT: &str = "
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
SELECT ?s ?p ?o ?g WHERE {
  {
    ?s ?p ?o .
    FILTER(STRSTARTS(STR(?p), \"https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/\"))
    FILTER(isIRI(?o) || isBlank(?o))
    FILTER NOT EXISTS { ?o ?any_p ?any_o }
  }
  UNION
  {
    GRAPH ?g {
      ?s ?p ?o .
      FILTER(STRSTARTS(STR(?p), \"https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/\"))
      FILTER(isIRI(?o) || isBlank(?o))
      FILTER NOT EXISTS { ?o ?any_p ?any_o }
    }
  }
}
";

pub const QUERY_INVARIANT_2: &str = "
PREFIX prov: <http://www.w3.org/ns/prov#>
PREFIX max:  <urn:lsp-max:core:>
ASK {
  {
    { ?artifact a max:Artifact } UNION { ?artifact a max:Diagnostic }
    FILTER NOT EXISTS {
      ?artifact prov:wasGeneratedBy ?receipt .
      ?receipt a max:Receipt .
    }
  }
  UNION
  {
    GRAPH ?g {
      { ?artifact a max:Artifact } UNION { ?artifact a max:Diagnostic }
      FILTER NOT EXISTS {
        ?artifact prov:wasGeneratedBy ?receipt .
        ?receipt a max:Receipt .
      }
    }
  }
}
";

pub const QUERY_INVARIANT_2_SELECT: &str = "
PREFIX prov: <http://www.w3.org/ns/prov#>
PREFIX max:  <urn:lsp-max:core:>
SELECT ?artifact ?g WHERE {
  {
    { ?artifact a max:Artifact } UNION { ?artifact a max:Diagnostic }
    FILTER NOT EXISTS {
      ?artifact prov:wasGeneratedBy ?receipt .
      ?receipt a max:Receipt .
    }
  }
  UNION
  {
    GRAPH ?g {
      { ?artifact a max:Artifact } UNION { ?artifact a max:Diagnostic }
      FILTER NOT EXISTS {
        ?artifact prov:wasGeneratedBy ?receipt .
        ?receipt a max:Receipt .
      }
    }
  }
}
";

pub const QUERY_INVARIANT_3: &str = "
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
PREFIX max:  <urn:lsp-max:core:>
ASK {
  {
    ?range a lsif:Range .
    ?range lsif:textDocument_definition ?defResult .
    FILTER NOT EXISTS {
      ?projection a max:Projection ;
                  max:sourceRange ?range .
    }
  }
  UNION
  {
    GRAPH ?g {
      ?range a lsif:Range .
      ?range lsif:textDocument_definition ?defResult .
      FILTER NOT EXISTS {
        ?projection a max:Projection ;
                    max:sourceRange ?range .
      }
    }
  }
}
";

pub const QUERY_INVARIANT_3_SELECT: &str = "
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
PREFIX max:  <urn:lsp-max:core:>
SELECT ?range ?g WHERE {
  {
    ?range a lsif:Range .
    ?range lsif:textDocument_definition ?defResult .
    FILTER NOT EXISTS {
      ?projection a max:Projection ;
                  max:sourceRange ?range .
    }
  }
  UNION
  {
    GRAPH ?g {
      ?range a lsif:Range .
      ?range lsif:textDocument_definition ?defResult .
      FILTER NOT EXISTS {
        ?projection a max:Projection ;
                    max:sourceRange ?range .
      }
    }
  }
}
";

pub const QUERY_INVARIANT_4: &str = "
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
ASK {
  {
    ?s ?p ?o .
    FILTER(STRSTARTS(STR(?p), \"https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/\"))
    FILTER(?p NOT IN (
      lsif:contains,
      lsif:next,
      lsif:moniker,
      lsif:attach,
      lsif:packageInformation,
      lsif:item,
      lsif:document,
      lsif:property,
      lsif:resultSet,
      lsif:textDocument_definition,
      lsif:textDocument_references,
      lsif:textDocument_hover,
      lsif:textDocument_declaration,
      lsif:textDocument_implementation,
      lsif:textDocument_typeDefinition,
      lsif:textDocument_callHierarchy,
      lsif:textDocument_typeHierarchy,
      lsif:textDocument_foldingRange,
      lsif:textDocument_documentLink,
      lsif:textDocument_documentSymbol,
      lsif:textDocument_diagnostic,
      lsif:textDocument_semanticTokens_full,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/definition>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/references>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/hover>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/declaration>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/implementation>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/typeDefinition>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/callHierarchy>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/typeHierarchy>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/foldingRange>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/documentLink>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/documentSymbol>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/diagnostic>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/semanticTokens/full>
    ))
  }
  UNION
  {
    GRAPH ?g {
      ?s ?p ?o .
      FILTER(STRSTARTS(STR(?p), \"https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/\"))
      FILTER(?p NOT IN (
        lsif:contains,
        lsif:next,
        lsif:moniker,
        lsif:attach,
        lsif:packageInformation,
        lsif:item,
        lsif:document,
        lsif:property,
        lsif:resultSet,
        lsif:textDocument_definition,
        lsif:textDocument_references,
        lsif:textDocument_hover,
        lsif:textDocument_declaration,
        lsif:textDocument_implementation,
        lsif:textDocument_typeDefinition,
        lsif:textDocument_callHierarchy,
        lsif:textDocument_typeHierarchy,
        lsif:textDocument_foldingRange,
        lsif:textDocument_documentLink,
        lsif:textDocument_documentSymbol,
        lsif:textDocument_diagnostic,
        lsif:textDocument_semanticTokens_full,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/definition>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/references>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/hover>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/declaration>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/implementation>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/typeDefinition>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/callHierarchy>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/typeHierarchy>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/foldingRange>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/documentLink>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/documentSymbol>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/diagnostic>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/semanticTokens/full>
      ))
    }
  }
}
";

pub const QUERY_INVARIANT_4_SELECT: &str = "
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>
SELECT ?s ?p ?o ?g WHERE {
  {
    ?s ?p ?o .
    FILTER(STRSTARTS(STR(?p), \"https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/\"))
    FILTER(?p NOT IN (
      lsif:contains,
      lsif:next,
      lsif:moniker,
      lsif:attach,
      lsif:packageInformation,
      lsif:item,
      lsif:document,
      lsif:property,
      lsif:resultSet,
      lsif:textDocument_definition,
      lsif:textDocument_references,
      lsif:textDocument_hover,
      lsif:textDocument_declaration,
      lsif:textDocument_implementation,
      lsif:textDocument_typeDefinition,
      lsif:textDocument_callHierarchy,
      lsif:textDocument_typeHierarchy,
      lsif:textDocument_foldingRange,
      lsif:textDocument_documentLink,
      lsif:textDocument_documentSymbol,
      lsif:textDocument_diagnostic,
      lsif:textDocument_semanticTokens_full,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/definition>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/references>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/hover>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/declaration>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/implementation>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/typeDefinition>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/callHierarchy>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/typeHierarchy>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/foldingRange>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/documentLink>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/documentSymbol>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/diagnostic>,
      <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/semanticTokens/full>
    ))
  }
  UNION
  {
    GRAPH ?g {
      ?s ?p ?o .
      FILTER(STRSTARTS(STR(?p), \"https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/\"))
      FILTER(?p NOT IN (
        lsif:contains,
        lsif:next,
        lsif:moniker,
        lsif:attach,
        lsif:packageInformation,
        lsif:item,
        lsif:document,
        lsif:property,
        lsif:resultSet,
        lsif:textDocument_definition,
        lsif:textDocument_references,
        lsif:textDocument_hover,
        lsif:textDocument_declaration,
        lsif:textDocument_implementation,
        lsif:textDocument_typeDefinition,
        lsif:textDocument_callHierarchy,
        lsif:textDocument_typeHierarchy,
        lsif:textDocument_foldingRange,
        lsif:textDocument_documentLink,
        lsif:textDocument_documentSymbol,
        lsif:textDocument_diagnostic,
        lsif:textDocument_semanticTokens_full,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/definition>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/references>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/hover>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/declaration>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/implementation>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/typeDefinition>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/callHierarchy>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/typeHierarchy>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/foldingRange>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/documentLink>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/documentSymbol>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/diagnostic>,
        <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument/semanticTokens/full>
      ))
    }
  }
}
";

pub const QUERY_INVARIANT_5: &str = "
PREFIX max:  <urn:lsp-max:core:>
ASK {
  {
    ?receipt a max:Receipt ;
             max:resultHash ?expectedResultHash ;
             max:queryHash ?qHash ;
             max:graphHash ?gHash .
    
    ?replay a max:Replay ;
            max:queryHash ?qHash ;
            max:graphHash ?gHash ;
            max:resultHash ?actualResultHash .
            
    FILTER(?expectedResultHash != ?actualResultHash)
  }
  UNION
  {
    GRAPH ?g {
      ?receipt a max:Receipt ;
               max:resultHash ?expectedResultHash ;
               max:queryHash ?qHash ;
               max:graphHash ?gHash .
      
      ?replay a max:Replay ;
              max:queryHash ?qHash ;
              max:graphHash ?gHash ;
              max:resultHash ?actualResultHash .
              
      FILTER(?expectedResultHash != ?actualResultHash)
    }
  }
}
";

pub const QUERY_INVARIANT_5_SELECT: &str = "
PREFIX max:  <urn:lsp-max:core:>
SELECT ?receipt ?expectedResultHash ?actualResultHash ?g WHERE {
  {
    ?receipt a max:Receipt ;
             max:resultHash ?expectedResultHash ;
             max:queryHash ?qHash ;
             max:graphHash ?gHash .
    
    ?replay a max:Replay ;
            max:queryHash ?qHash ;
            max:graphHash ?gHash ;
            max:resultHash ?actualResultHash .
            
    FILTER(?expectedResultHash != ?actualResultHash)
  }
  UNION
  {
    GRAPH ?g {
      ?receipt a max:Receipt ;
               max:resultHash ?expectedResultHash ;
               max:queryHash ?qHash ;
               max:graphHash ?gHash .
      
      ?replay a max:Replay ;
              max:queryHash ?qHash ;
              max:graphHash ?gHash ;
              max:resultHash ?actualResultHash .
              
      FILTER(?expectedResultHash != ?actualResultHash)
    }
  }
}
";
