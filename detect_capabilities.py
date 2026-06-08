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
    meta = None
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            meta = json.loads(response.read().decode())
            print("Successfully fetched metamodel from GitHub.")
    except Exception as e:
        print(f"Error fetching metamodel from GitHub: {e}")
        local_path = os.path.join(os.path.dirname(__file__), "crates/tower-lsp-max-specgen/fixtures/metaModel-3.18.json")
        if os.path.exists(local_path):
            print(f"Attempting to load local metamodel from {local_path}...")
            try:
                with open(local_path, "r") as f:
                    meta = json.load(f)
                print("Successfully loaded local metamodel.")
            except Exception as le:
                print(f"Error reading local metamodel: {le}")
                sys.exit(1)
        else:
            print(f"Local metamodel file not found at {local_path}.")
            sys.exit(1)

    # We only care about client-to-server requests and notifications
    # as these must be implemented by the LanguageServer trait.
    # Server-to-client requests are sent by the server to the client.
    client_to_server_methods = {}
    
    for r in meta.get("requests", []):
        if r.get("messageDirection") in ("clientToServer", "both", None):
            client_to_server_methods[r["method"]] = "request"
            
    for n in meta.get("notifications", []):
        if n.get("messageDirection") in ("clientToServer", "both", None):
            client_to_server_methods[n["method"]] = "notification"

    # Read tower-lsp-max LanguageServer trait
    trait_path = sys.argv[1] if len(sys.argv) > 1 else "src/language_server.rs"
    if not os.path.exists(trait_path):
        print(f"Error: {trait_path} not found.")
        sys.exit(1)

    with open(trait_path, 'r') as f:
        content = f.read()

    # Parse #[rpc(name = "...")] attributes and find the next async fn
    # We match: #[rpc(name = "LSP_METHOD_NAME")]
    # followed by optional comments/whitespace, then async fn RUST_FN_NAME
    implemented_lsp_methods = {}
    
    # We do a sequential scan or regex finditer
    pattern = re.compile(
        r'#\[rpc\(name\s*=\s*"([^"]+)"[^\]]*\)\]\s*(?:\/\/\/[^\n]*\s*)*\s*async\s+fn\s+([a-zA-Z0-9_]+)',
        re.MULTILINE
    )
    
    for match in pattern.finditer(content):
        lsp_method = match.group(1)
        rust_fn = match.group(2)
        implemented_lsp_methods[lsp_method] = rust_fn

    # Whitelist methods that are handled internally by tower-lsp runtime
    # and don't need to be implemented by the user in LanguageServer trait:
    runtime_handled_methods = {
        "exit": "exit",
        "$/cancelRequest": "cancel_request"
    }

    missing_methods = []
    for method in sorted(client_to_server_methods.keys()):
        if method not in implemented_lsp_methods and method not in runtime_handled_methods:
            missing_methods.append(method)

    print(f"\n--- Tower-LSP-Max LSP 3.18 Capability Detector ---\n")
    print(f"Total Client-to-Server LSP 3.18 methods: {len(client_to_server_methods)}")
    print(f"Implemented in tower-lsp-max LanguageServer trait: {len(implemented_lsp_methods)}")
    print(f"Handled internally by tower-lsp-max runtime: {len(runtime_handled_methods)}")
    print(f"Missing methods: {len(missing_methods)}\n")
    
    if missing_methods:
        print("Unimplemented Client-to-Server Methods:")
        for method in missing_methods:
            expected_fn = to_snake_case(method.split('/')[-1])
            print(f"  - {method} (expected: async fn {expected_fn}())")
    else:
        print("All client-to-server LSP 3.18 capabilities are fully supported!")

if __name__ == "__main__":
    main()

