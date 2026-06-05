import json
import urllib.request
import re
import sys
import os

def to_snake_case(name):
    s1 = re.sub('(.)([A-Z][a-z]+)', r'\1_\2', name)
    return re.sub('([a-z0-9])([A-Z])', r'\1_\2', s1).lower()

def main():
    print("Fetching LSP 3.18 metaModel.json...")
    url = "https://raw.githubusercontent.com/microsoft/vscode-languageserver-node/main/protocol/metaModel.json"
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req) as response:
        meta = json.loads(response.read().decode())

    lsp_methods = []
    
    # Extract all requests and notifications
    for req in meta.get("requests", []):
        lsp_methods.append(req["method"])
    for notif in meta.get("notifications", []):
        lsp_methods.append(notif["method"])

    # Map LSP methods to likely rust method names
    # e.g., textDocument/hover -> hover
    # textDocument/didOpen -> did_open
    # workspace/executeCommand -> execute_command
    expected_rust_methods = set()
    method_mapping = {}
    for m in lsp_methods:
        parts = m.split('/')
        base_name = parts[-1]
        
        # some exceptions or namespace rules
        if parts[0] == "$":
            base_name = parts[1]
        
        snake_name = to_snake_case(base_name)
        
        # specific mappings typical in tower-lsp
        if m == "workspace/symbol":
            snake_name = "symbol"
        elif m == "workspace/configuration":
            snake_name = "configuration"
        elif m == "workspace/workspaceFolders":
            snake_name = "workspace_folders"
            
        expected_rust_methods.add(snake_name)
        method_mapping[snake_name] = m

    # Read tower-lsp-max LanguageServer trait
    trait_path = sys.argv[1] if len(sys.argv) > 1 else "src/lib.rs"
    if not os.path.exists(trait_path):
        print(f"Error: {trait_path} not found.")
        sys.exit(1)

    with open(trait_path, 'r') as f:
        content = f.read()

    # Extract the trait block
    match = re.search(r"pub trait LanguageServer.*?(?=\{)(.*?^\})", content, re.MULTILINE | re.DOTALL)
    if not match:
        print("Error: Could not find LanguageServer trait.")
        sys.exit(1)

    trait_body = match.group(1)
    
    # Find all async fn declarations
    implemented_methods = set()
    fn_matches = re.finditer(r"async\s+fn\s+([a-zA-Z0-9_]+)\s*\(", trait_body)
    for m in fn_matches:
        implemented_methods.add(m.group(1))

    # Calculate missing
    missing_methods = []
    for expected in sorted(expected_rust_methods):
        if expected not in implemented_methods:
            missing_methods.append(expected)

    print(f"\n--- Tower-LSP-Max LSP 3.18 Capability Detector ---\n")
    print(f"Total LSP 3.18 capabilities analyzed: {len(lsp_methods)}")
    print(f"Implemented in tower-lsp-max: {len(implemented_methods)}")
    print(f"Missing capabilities: {len(missing_methods)}\n")
    
    print("Unimplemented Capabilities (LSP Method -> Expected Rust Method):")
    for expected in missing_methods:
        print(f"  - {method_mapping.get(expected, 'unknown')} -> async fn {expected}()")
        
if __name__ == "__main__":
    main()
