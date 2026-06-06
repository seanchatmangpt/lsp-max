import re

def compact_file():
    with open("src/language_server.rs", "r") as f:
        content = f.read()

    lines = content.splitlines()
    output = []
    
    # Add imports and module declaration
    in_imports = True
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("pub trait LanguageServer"):
            in_imports = False
            output.append("pub(crate) mod impls;")
            output.append("")
        if in_imports:
            output.append(line)
            i += 1
            continue
        break

    # Now parse the trait and its methods
    trait_body_lines = lines[i:]
    method_regex = re.compile(r'#\[rpc\(name\s*=\s*"([^"]+)"(?:,\s*layer\s*=\s*"[^"]+")?\)\]')
    
    output.append("/// Trait implemented by language server backends to handle LSP requests and notifications.")
    output.append("#[rpc]")
    output.append("#[async_trait]")
    output.append("#[auto_impl(Arc, Box)]")
    output.append("pub trait LanguageServer: Send + Sync + 'static {")
    
    while i < len(lines):
        line = lines[i].strip()
        if not line:
            i += 1
            continue
        
        if line.startswith("///") or line.startswith("//"):
            i += 1
            continue
            
        if line.startswith("#[rpc"):
            rpc_attr = line
            
            # Read method signature
            i += 1
            sig_lines = []
            while i < len(lines):
                l = lines[i]
                sig_lines.append(l)
                if "{" in l or ";" in l:
                    break
                i += 1
            
            sig = " ".join(sig_lines).strip()
            
            # Extract method name and parameters
            fn_match = re.search(r'async\s+fn\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^{;]+))?', sig)
            if fn_match:
                fn_name = fn_match.group(1)
                fn_params = fn_match.group(2)
                fn_ret = fn_match.group(3)
                
                has_body = "{" in sig
                body = ""
                if has_body:
                    brace_count = 1
                    body_start = sig.find("{") + 1
                    body_parts = [sig[body_start:]]
                    
                    for char in sig[body_start:]:
                        if char == "{": brace_count += 1
                        elif char == "}": brace_count -= 1
                    
                    i += 1
                    while brace_count > 0 and i < len(lines):
                        l = lines[i]
                        body_parts.append(l)
                        for char in l:
                            if char == "{": brace_count += 1
                            elif char == "}": brace_count -= 1
                        i += 1
                    body = " ".join([bp.strip() for bp in body_parts]).strip()
                else:
                    i += 1
                
                is_custom = fn_name.startswith("max_") or fn_name == "goto_definition"
                
                # Add a concise doc comment for every method to satisfy deny(missing_docs)
                output.append(f"    /// Handler for the `{fn_name}` endpoint.")
                output.append(f"    {rpc_attr}")
                
                ret_str = f" -> {fn_ret.strip()}" if fn_ret else ""
                
                clean_params = fn_params.replace("&self,", "").replace("&self", "").strip()
                if clean_params:
                    sig_params = f"&self, {clean_params}"
                else:
                    sig_params = "&self"
                
                if is_custom:
                    param_names = []
                    for p in fn_params.split(","):
                        p = p.strip()
                        if not p or p == "&self":
                            continue
                        parts = p.split(":")
                        if parts:
                            param_names.append(parts[0].strip())
                    
                    args_str = ", ".join(param_names)
                    output.append(f"    async fn {fn_name}({sig_params}){ret_str} {{ impls::{fn_name}({args_str}).await }}")
                else:
                    if has_body:
                        # Collapse the body to single line
                        # remove trailing brace if present
                        if body.endswith("}"):
                            body = body[:-1].strip()
                        output.append(f"    async fn {fn_name}({sig_params}){ret_str} {{ {body} }}")
                    else:
                        output.append(f"    async fn {fn_name}({sig_params}){ret_str};")
            else:
                output.append(rpc_attr)
                output.append(sig)
            continue
            
        i += 1

    output.append("}")
    output.append("")
    
    with open("src/language_server.rs", "w") as f:
        f.write("\n".join(output))

compact_file()
