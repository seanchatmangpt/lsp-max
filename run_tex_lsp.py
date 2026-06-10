import subprocess
import json
import os
import glob
import time
import select

def encode_message(msg_dict):
    body = json.dumps(msg_dict).encode('utf-8')
    header = f"Content-Length: {len(body)}\r\n\r\n".encode('utf-8')
    return header + body

# Launch with 'server start' command
p = subprocess.Popen(
    ["cargo", "run", "-q", "-p", "tex-lsp", "--", "server", "start"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

init_req = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "processId": None,
        "rootUri": None,
        "capabilities": {}
    }
}

p.stdin.write(encode_message(init_req))
p.stdin.flush()

# Read initialize response
while True:
    line = p.stdout.readline().decode('utf-8')
    if line.startswith("Content-Length:"):
        length = int(line.split(":")[1].strip())
        p.stdout.readline()
        p.stdout.read(length)
        break

chapters = glob.glob("docs/thesis/ggen/**/*.tex", recursive=True)

for chapter in chapters:
    with open(chapter, 'r') as f:
        text = f.read()
    uri = f"file://{os.path.abspath(chapter)}"
    
    did_open = {
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "latex",
                "version": 1,
                "text": text
            }
        }
    }
    p.stdin.write(encode_message(did_open))
    p.stdin.flush()

time.sleep(1)

# Read responses and notifications
while True:
    reads, _, _ = select.select([p.stdout], [], [], 0.5)
    if not reads:
        break
    
    try:
        line = p.stdout.readline().decode('utf-8')
        if not line:
            break
        if line.startswith("Content-Length:"):
            length = int(line.split(":")[1].strip())
            p.stdout.readline()
            response = p.stdout.read(length).decode('utf-8')
            msg = json.loads(response)
            
            if "method" in msg and msg["method"] == "textDocument/publishDiagnostics":
                uri = msg["params"]["uri"]
                diags = msg["params"]["diagnostics"]
                if len(diags) > 0:
                    print(f"--- Diagnostics for {os.path.basename(uri)} ---")
                    for d in diags:
                        print(f"Line {d['range']['start']['line']+1}: {d['message']}")
    except Exception as e:
        print(e)
        break

p.terminate()
