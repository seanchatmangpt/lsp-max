#!/usr/bin/env python3
import os
import re
import sys

# Try to import rdflib for SPARQL syntax validation
try:
    from rdflib.plugins.sparql.parser import parseQuery
    HAS_RDFLIB = True
except ImportError:
    HAS_RDFLIB = False

# List of expected files
EXPECTED_FILES = {
    "README.md",
    "prd.md",
    "logical_architecture.md",
    "ard_decisions.md",
    "data_model.md",
    "invariants.md",
    "sequence_flows.md",
    "verification_and_gate.md"
}

DOCS_DIR = os.path.abspath("docs/v26.6.5/prd-ard")

# Regex pattern for markdown relative links
# Matches standard markdown links [text](url) where url does not start with http/https/urn/file/mailto
LINK_REGEX = re.compile(r'\[[^\]]+\]\(([^)]+)\)')

# Regex pattern for Mermaid flowchart lines
MERMAID_FLOWCHART_LINE = re.compile(
    r'^\s*\w+(?:\[[^\]]+\]|\([^)]+\))?\s*(?:-->|---)\s*\w+(?:\[[^\]]+\]|\([^)]+\))?\s*$'
)
MERMAID_FLOWCHART_NODE = re.compile(r'^\s*\w+(?:\[[^\]]+\]|\([^)]+\))?\s*$')

def check_stubs(content, filepath):
    """Check for TODOs, TBDs, and placeholder stubs."""
    stub_keywords = [r'\bTODO\b', r'\bTBD\b', r'\bplaceholder\b', r'\bstub\b', r'\bFIXME\b', r'\bunimplemented\b']
    violations = []
    lines = content.splitlines()
    for idx, line in enumerate(lines, 1):
        for kw in stub_keywords:
            if re.search(kw, line, re.IGNORECASE):
                # Ensure we don't flag the verification script itself or normal text mentioning them
                # (though in our docs we shouldn't have them at all)
                violations.append((idx, line.strip()))
    return violations

def validate_links(content, filepath):
    """Validate relative markdown links inside the file."""
    links = LINK_REGEX.findall(content)
    violations = []
    for link in links:
        link_url = link.strip()
        # Skip absolute links
        if (link_url.startswith("http://") or 
            link_url.startswith("https://") or 
            link_url.startswith("urn:") or 
            link_url.startswith("file://") or 
            link_url.startswith("mailto:")):
            continue
        
        # Handle self-anchors
        if link_url.startswith("#"):
            continue
            
        # Strip anchor parts
        url_path = link_url.split('#')[0]
        if not url_path:
            continue
            
        # Resolve path relative to the file being checked
        resolved_path = os.path.normpath(os.path.join(os.path.dirname(filepath), url_path))
        if not os.path.isfile(resolved_path):
            violations.append(link_url)
    return violations

def validate_sparql_query(query_str):
    """Validate SPARQL query syntax using rdflib."""
    if not HAS_RDFLIB:
        return "rdflib not installed, skipping strict SPARQL validation"
    try:
        parseQuery(query_str)
        return None
    except Exception as e:
        return str(e)

def run_semantic_invariants_tests(invariants_file_content):
    """
    Extract the SPARQL queries from invariants.md, set up mock graphs,
    and verify that they evaluate correctly.
    """
    if not HAS_RDFLIB:
        print("  [!] Skip: rdflib not installed, cannot perform semantic verification.")
        return True

    import rdflib

    # Extract SPARQL blocks
    sparql_blocks = re.findall(r'```sparql\n(.*?)\n```', invariants_file_content, re.DOTALL)
    if len(sparql_blocks) < 5:
        print(f"  [-] Fail: Expected at least 5 SPARQL queries in invariants.md, found {len(sparql_blocks)}.")
        return False

    # --- Invariant 1 ---
    print("  Testing Invariant 1 (Orphan LSIF Relations) semantically...")
    inv_1_query = sparql_blocks[0]
    
    # Valid graph (no orphans)
    g_valid = rdflib.Graph()
    g_valid.parse(data='''
        @prefix lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/> .
        @prefix urn: <urn:test:> .
        urn:range_1 a lsif:Range ;
                    lsif:next urn:result_1 .
        urn:result_1 a lsif:ResultSet ;
                     lsif:property "definitions" .
    ''', format='turtle')
    
    try:
        res_valid = bool(g_valid.query(inv_1_query))
        if res_valid:
            print("    [-] Fail: Invariant 1 falsely flagged valid structure as having orphans.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 1 failed to execute on valid graph: {e}")
        return False

    # Invalid graph (has orphans)
    g_invalid = rdflib.Graph()
    g_invalid.parse(data='''
        @prefix lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/> .
        @prefix urn: <urn:test:> .
        urn:range_1 a lsif:Range ;
                    lsif:next urn:nonexistent .
    ''', format='turtle')
    
    try:
        res_invalid = bool(g_invalid.query(inv_1_query))
        if not res_invalid:
            print("    [-] Fail: Invariant 1 failed to flag an orphan relation.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 1 failed to execute on invalid graph: {e}")
        return False
    print("    [+] Pass: Invariant 1 semantic test passed.")

    # --- Invariant 2 ---
    print("  Testing Invariant 2 (Unreceipted Graph Consequence) semantically...")
    inv_2_query = sparql_blocks[1]
    
    # Valid graph (all artifacts have receipt)
    g_valid2 = rdflib.Graph()
    g_valid2.parse(data='''
        @prefix prov: <http://www.w3.org/ns/prov#> .
        @prefix max:  <urn:tower-lsp-max:core:> .
        @prefix urn: <urn:test:> .
        urn:art1 a max:Artifact ;
                 prov:wasGeneratedBy urn:rcpt1 .
        urn:rcpt1 a max:Receipt .
    ''', format='turtle')
    
    try:
        res_valid2 = bool(g_valid2.query(inv_2_query))
        if res_valid2:
            print("    [-] Fail: Invariant 2 falsely flagged receipted artifact.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 2 failed on valid graph: {e}")
        return False
        
    # Invalid graph (unreceipted artifact)
    g_invalid2 = rdflib.Graph()
    g_invalid2.parse(data='''
        @prefix max:  <urn:tower-lsp-max:core:> .
        @prefix urn: <urn:test:> .
        urn:art1 a max:Artifact .
    ''', format='turtle')
    
    try:
        res_invalid2 = bool(g_invalid2.query(inv_2_query))
        if not res_invalid2:
            print("    [-] Fail: Invariant 2 failed to flag unreceipted artifact.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 2 failed on invalid graph: {e}")
        return False
    print("    [+] Pass: Invariant 2 semantic test passed.")

    # --- Invariant 3 ---
    print("  Testing Invariant 3 (No Hot-Path SPARQL Dependency) semantically...")
    inv_3_query = sparql_blocks[2]
    
    # Valid graph (has projection)
    g_valid3 = rdflib.Graph()
    g_valid3.parse(data='''
        @prefix lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/> .
        @prefix max:  <urn:tower-lsp-max:core:> .
        @prefix urn: <urn:test:> .
        urn:range1 a lsif:Range ;
                   lsif:textDocument_definition urn:defres1 .
        urn:proj1 a max:Projection ;
                  max:sourceRange urn:range1 .
    ''', format='turtle')
    
    try:
        res_valid3 = bool(g_valid3.query(inv_3_query))
        if res_valid3:
            print("    [-] Fail: Invariant 3 falsely flagged range with projection.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 3 failed on valid graph: {e}")
        return False
        
    # Invalid graph (missing projection)
    g_invalid3 = rdflib.Graph()
    g_invalid3.parse(data='''
        @prefix lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/> .
        @prefix urn: <urn:test:> .
        urn:range1 a lsif:Range ;
                   lsif:textDocument_definition urn:defres1 .
    ''', format='turtle')
    
    try:
        res_invalid3 = bool(g_invalid3.query(inv_3_query))
        if not res_invalid3:
            print("    [-] Fail: Invariant 3 failed to flag missing projection.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 3 failed on invalid graph: {e}")
        return False
    print("    [+] Pass: Invariant 3 semantic test passed.")

    # --- Invariant 4 ---
    print("  Testing Invariant 4 (No Ontology Laundering) semantically...")
    inv_4_query = sparql_blocks[3]
    
    # Valid graph (whitelisted properties)
    g_valid4 = rdflib.Graph()
    g_valid4.parse(data='''
        @prefix lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/> .
        @prefix urn: <urn:test:> .
        urn:range1 lsif:next urn:result1 .
    ''', format='turtle')
    
    try:
        res_valid4 = bool(g_valid4.query(inv_4_query))
        if res_valid4:
            print("    [-] Fail: Invariant 4 falsely flagged whitelisted property.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 4 failed on valid graph: {e}")
        return False
        
    # Invalid graph (ontology laundering)
    g_invalid4 = rdflib.Graph()
    g_invalid4.parse(data='''
        @prefix urn: <urn:test:> .
        urn:range1 <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/customProperty> urn:result1 .
    ''', format='turtle')
    
    try:
        res_invalid4 = bool(g_invalid4.query(inv_4_query))
        if not res_invalid4:
            print("    [-] Fail: Invariant 4 failed to flag ontology laundering.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 4 failed on invalid graph: {e}")
        return False
    print("    [+] Pass: Invariant 4 semantic test passed.")

    # --- Invariant 5 ---
    print("  Testing Invariant 5 (No False ALIVE) semantically...")
    inv_5_query = sparql_blocks[4]
    
    # Valid graph (matching hashes)
    g_valid5 = rdflib.Graph()
    g_valid5.parse(data='''
        @prefix max:  <urn:tower-lsp-max:core:> .
        @prefix urn: <urn:test:> .
        urn:rcpt1 a max:Receipt ;
                  max:resultHash "hash1" ;
                  max:queryHash "q1" ;
                  max:graphHash "g1" .
        urn:rpl1 a max:Replay ;
                 max:resultHash "hash1" ;
                 max:queryHash "q1" ;
                 max:graphHash "g1" .
    ''', format='turtle')
    
    try:
        res_valid5 = list(g_valid5.query(inv_5_query))
        if len(res_valid5) > 0:
            print("    [-] Fail: Invariant 5 falsely matched query/replay with same hashes.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 5 failed on valid graph: {e}")
        return False
        
    # Invalid graph (mismatching hashes)
    g_invalid5 = rdflib.Graph()
    g_invalid5.parse(data='''
        @prefix max:  <urn:tower-lsp-max:core:> .
        @prefix urn: <urn:test:> .
        urn:rcpt1 a max:Receipt ;
                  max:resultHash "hash1" ;
                  max:queryHash "q1" ;
                  max:graphHash "g1" .
        urn:rpl1 a max:Replay ;
                 max:resultHash "hash2" ;
                 max:queryHash "q1" ;
                 max:graphHash "g1" .
    ''', format='turtle')
    
    try:
        res_invalid5 = list(g_invalid5.query(inv_5_query))
        if len(res_invalid5) == 0:
            print("    [-] Fail: Invariant 5 failed to detect mismatching hashes.")
            return False
    except Exception as e:
        print(f"    [-] Fail: Invariant 5 failed on invalid graph: {e}")
        return False
    print("    [+] Pass: Invariant 5 semantic test passed.")

    return True

def validate_mermaid_diagram(diagram_content, filepath, block_idx):
    """Validate Mermaid diagram syntax."""
    lines = [line.strip() for line in diagram_content.splitlines() if line.strip() and not line.strip().startswith("%%")]
    if not lines:
        return ["Empty Mermaid diagram"]
        
    first_line = lines[0]
    errors = []
    
    # Check if it's a sequence diagram
    if first_line.startswith("sequenceDiagram"):
        nested_blocks = []
        for line_num, line in enumerate(lines, 1):
            if line == "sequenceDiagram" or line == "autonumber":
                continue
            
            # Check participant or actor
            if re.match(r'^(participant|actor)\s+\w+(\s+as\s+.+)?$', line):
                continue
                
            # Check Note
            if re.match(r'^Note\s+(over|left\s+of|right\s+of)\s+[\w\s,]+:\s*.+$', line):
                continue
                
            # Check Message exchange
            if re.match(r'^\w+\s*(?:-->>|->>|-->|->|-x|--x|-\)|--\))\s*\w+\s*:\s*.+$', line):
                continue
                
            # Check Activation
            if re.match(r'^(activate|deactivate)\s+\w+$', line):
                continue
                
            # Check block start
            block_match = re.match(r'^(loop|alt|opt|par|rect)(\s+.+)?$', line)
            if block_match:
                nested_blocks.append(block_match.group(1))
                continue
                
            # Check block division
            div_match = re.match(r'^(else|and)(\s+.+)?$', line)
            if div_match:
                if not nested_blocks:
                    errors.append(f"Line {line_num}: Division '{div_match.group(1)}' outside block")
                elif div_match.group(1) == "else" and nested_blocks[-1] != "alt":
                    errors.append(f"Line {line_num}: 'else' inside non-alt block '{nested_blocks[-1]}'")
                elif div_match.group(1) == "and" and nested_blocks[-1] != "par":
                    errors.append(f"Line {line_num}: 'and' inside non-par block '{nested_blocks[-1]}'")
                continue
                
            # Check block end
            if line == "end":
                if not nested_blocks:
                    errors.append(f"Line {line_num}: Mismatched 'end'")
                else:
                    nested_blocks.pop()
                continue
                
            errors.append(f"Line {line_num}: Invalid sequence diagram line: '{line}'")
            
        if nested_blocks:
            errors.append(f"Unclosed blocks remaining: {nested_blocks}")
            
    # Check if it's a flowchart or graph
    elif re.match(r'^(graph|flowchart)\s+(TD|TB|BT|RL|LR)$', first_line):
        for line_num, line in enumerate(lines[1:], 2):
            if not (MERMAID_FLOWCHART_LINE.match(line) or MERMAID_FLOWCHART_NODE.match(line)):
                errors.append(f"Line {line_num}: Invalid flowchart line: '{line}'")
    else:
        errors.append(f"Unsupported Mermaid diagram type: '{first_line}'")
        
    return errors

def main():
    print("====================================================")
    print("  Oxigraph Admitted Graph Control Plane Verifier")
    print("====================================================")
    
    # 1. Check directory presence
    if not os.path.isdir(DOCS_DIR):
        print(f"[-] Error: Documentation directory does not exist: {DOCS_DIR}")
        sys.exit(1)
        
    # 2. Check file presence
    actual_files = set(os.listdir(DOCS_DIR))
    missing_files = EXPECTED_FILES - actual_files
    extra_files = actual_files - EXPECTED_FILES
    
    failed = False
    
    print("\n--- Phase 1: File Presence Verification ---")
    if missing_files:
        print(f"[-] Fail: Missing expected files: {missing_files}")
        failed = True
    else:
        print("[+] Pass: All 8 expected files exist.")
        
    if extra_files:
        print(f"[-] Fail: Unexpected files found in {DOCS_DIR}: {extra_files}")
        failed = True
    else:
        print("[+] Pass: No unexpected files in directory.")
        
    # 3. Verify each file content
    print("\n--- Phase 2: File Content Verification ---")
    for filename in sorted(EXPECTED_FILES):
        filepath = os.path.join(DOCS_DIR, filename)
        if not os.path.isfile(filepath):
            continue
            
        print(f"\nChecking file: {filename}")
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
            
        # Stub check
        stubs = check_stubs(content, filepath)
        if stubs:
            print(f"  [-] Fail: Found {len(stubs)} placeholder/stub violations:")
            for line_no, line_val in stubs:
                print(f"    Line {line_no}: {line_val}")
            failed = True
        else:
            print("  [+] Pass: No placeholders/stubs found.")
            
        # Link check
        broken_links = validate_links(content, filepath)
        if broken_links:
            print(f"  [-] Fail: Found {len(broken_links)} broken relative links: {broken_links}")
            failed = True
        else:
            print("  [+] Pass: All relative markdown links are valid.")
            
        # Extract and validate SPARQL blocks (only for invariants.md, or any file)
        sparql_blocks = re.findall(r'```sparql\n(.*?)\n```', content, re.DOTALL)
        if sparql_blocks:
            print(f"  Found {len(sparql_blocks)} SPARQL blocks.")
            for idx, sparql_query in enumerate(sparql_blocks, 1):
                err = validate_sparql_query(sparql_query)
                if err:
                    print(f"    [-] Fail: SPARQL query #{idx} is invalid: {err}")
                    failed = True
                else:
                    print(f"    [+] Pass: SPARQL query #{idx} syntax is valid.")
            
            if filename == "invariants.md":
                print("  Running semantic verification tests for invariants...")
                if not run_semantic_invariants_tests(content):
                    failed = True
                    
        # Extract and validate Mermaid blocks
        mermaid_blocks = re.findall(r'```mermaid\n(.*?)\n```', content, re.DOTALL)
        if mermaid_blocks:
            print(f"  Found {len(mermaid_blocks)} Mermaid blocks.")
            for idx, mermaid_content in enumerate(mermaid_blocks, 1):
                errors = validate_mermaid_diagram(mermaid_content, filepath, idx)
                if errors:
                    print(f"    [-] Fail: Mermaid diagram #{idx} has errors:")
                    for err in errors:
                        print(f"      - {err}")
                    failed = True
                else:
                    print(f"    [+] Pass: Mermaid diagram #{idx} syntax is valid.")
                    
    print("\n====================================================")
    if failed:
        print("[-] VERIFICATION STATUS: FAILED")
        sys.exit(1)
    else:
        print("[+] VERIFICATION STATUS: 100% SUCCESSFUL")
        sys.exit(0)

if __name__ == "__main__":
    main()
