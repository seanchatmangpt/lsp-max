import re

with open('Cargo.toml', 'r') as f:
    content = f.read()

content = re.sub(r'lsp-max-ast = \{ path = "crates/lsp-max-adapters/lsp-max-ast", version = "\d+\.\d+\.\d+" \}', 
                 r'lsp-max-ast = { path = "crates/lsp-max-adapters/lsp-max-ast", version = "26.6.24" }', 
                 content)

with open('Cargo.toml', 'w') as f:
    f.write(content)

