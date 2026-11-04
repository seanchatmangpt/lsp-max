"""Async JSON-RPC (LSP-framed) client for branchless-lsp.

Stdlib only: asyncio, json, os, urllib.parse. Spawn the server binary via
subprocess, run the initialize handshake, didOpen/didSave, consume
publishDiagnostics, and request max/conformanceVector.
"""

import asyncio
import json
import os
import tempfile
import urllib.parse

__all__ = [
    "analyze",
    "analyze_text",
    "diagnostics_for_files",
    "LspError",
    "LspProtocolError",
    "LspServerCrashed",
    "LspTimeout",
]

DEFAULT_TIMEOUT = 30.0

_MAX_HEADER_BYTES = 16 * 1024


class LspError(Exception):
    """Base class for typed client failures."""


class LspProtocolError(LspError):
    """Malformed frame: bad header, bad JSON, or undecodable body."""

    def __init__(self, detail):
        super().__init__("malformed frame: %s" % (detail,))


class LspServerCrashed(LspError):
    """Server process died before completing the session."""

    def __init__(self, code):
        super().__init__("server exited unexpectedly (code=%s)" % (code,))


class LspTimeout(LspError):
    """A bounded wait on a server response or notification elapsed."""


class BranchlessLspClient:
    """Spawn `server_bin`, speak LSP over stdio, collect diagnostics."""

    def __init__(self, server_bin, timeout=DEFAULT_TIMEOUT):
        self._bin = server_bin
        self._timeout = timeout
        self._proc = None
        self._next_id = 0
        self._responses = {}
        self._notifications = asyncio.Queue()
        self._stderr_buf = bytearray()
        self._stderr_task = None
        self._reader_task = None

    # -- framing ---------------------------------------------------------

    def _stdin(self):
        """Narrowed stdin: valid only between start() and stop()."""
        if self._proc is None or self._proc.stdin is None:
            raise LspServerCrashed(
                None if self._proc is None else self._proc.returncode
            )
        return self._proc.stdin

    def _stdout(self):
        """Narrowed stdout: valid only between start() and stop()."""
        if self._proc is None or self._proc.stdout is None:
            raise LspServerCrashed(
                None if self._proc is None else self._proc.returncode
            )
        return self._proc.stdout

    @staticmethod
    def _encode(message):
        body = json.dumps(message, separators=(",", ":")).encode("utf-8")
        header = ("Content-Length: %d\r\n\r\n" % len(body)).encode("ascii")
        return header + body

    async def _drain_stderr(self):
        """Accumulate server stderr into a bounded tail buffer."""
        stream = self._proc.stderr
        while True:
            chunk = await stream.read(4096)
            if not chunk:
                return
            self._stderr_buf.extend(chunk)
            if len(self._stderr_buf) > 8192:
                del self._stderr_buf[:-8192]

    def stderr_text(self):
        """Decoded tail of the server's stderr (empty if silent)."""
        return bytes(self._stderr_buf).decode("utf-8", "replace").strip()

    def _annotate(self, exc):
        """Attach the stderr tail to a typed failure and return it."""
        tail = self.stderr_text()
        if tail:
            base = exc.args[0] if exc.args else type(exc).__name__
            exc.stderr = tail
            exc.args = (
                "%s | server stderr tail: %s" % (base, tail[-2000:]),
            )
        return exc

    def _returncode(self):
        """Narrowed returncode: None-safe even before start()."""
        return None if self._proc is None else self._proc.returncode

    async def _read_header(self):
        raw = bytearray()
        while len(raw) < _MAX_HEADER_BYTES:
            byte = await self._stdout().read(1)
            if not byte:
                raise LspServerCrashed(self._returncode())
            raw += byte
            if raw.endswith(b"\r\n\r\n"):
                headers = {}
                for line in bytes(raw[:-4]).decode("ascii").split("\r\n"):
                    name, _, value = line.partition(":")
                    headers[name.strip().lower()] = value.strip()
                if "content-length" not in headers:
                    raise LspProtocolError("missing Content-Length header")
                try:
                    return int(headers["content-length"])
                except ValueError:
                    raise LspProtocolError(
                        "bad Content-Length: %r" % headers["content-length"]
                    ) from None
        raise LspProtocolError("header block exceeded %d bytes" % _MAX_HEADER_BYTES)

    async def _read_frame(self):
        try:
            length = await asyncio.wait_for(
                self._read_header(), timeout=self._timeout
            )
        except (asyncio.TimeoutError, TimeoutError):
            raise LspTimeout(
                "frame header: no bytes within %.1fs" % self._timeout
            ) from None
        try:
            body = await asyncio.wait_for(
                self._stdout().readexactly(length), timeout=self._timeout
            )
        except asyncio.IncompleteReadError as exc:
            raise LspServerCrashed(
                None if self._proc is None else self._proc.returncode
            ) from None
        try:
            return json.loads(body.decode("utf-8"))
        except (ValueError, UnicodeDecodeError) as exc:
            raise LspProtocolError("invalid JSON body: %s" % (exc,)) from None

    async def _pump(self):
        """Route frames: responses by id, notifications into the queue.

        On any framing/EOF failure, poison the queue with the typed error
        so a pending `_request` fails with LspServerCrashed instead of
        hanging until a generic TimeoutError.
        """
        try:
            while True:
                message = await self._read_frame()
                if "id" in message and (
                    "result" in message or "error" in message
                ):
                    self._responses[message["id"]] = message
                    self._notifications.put_nowait(
                        {"_wake": True, "id": message["id"]}
                    )
                else:
                    self._notifications.put_nowait(message)
        except (LspError, asyncio.IncompleteReadError) as exc:
            crash = exc if isinstance(exc, LspError) else LspServerCrashed(
                None if self._proc is None else self._proc.returncode
            )
            self._notifications.put_nowait({"_crash": crash})

    def _send(self, message):
        self._stdin().write(self._encode(message))

    async def _request(self, method, params):
        self._next_id += 1
        request_id = self._next_id
        self._send(
            {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
        )
        await self._stdin().drain()
        deadline = asyncio.get_running_loop().time() + self._timeout
        while request_id not in self._responses:
            remaining = deadline - asyncio.get_running_loop().time()
            if remaining <= 0:
                raise LspTimeout(
                    "%s: no response within %.1fs" % (method, self._timeout)
                )
            try:
                item = await asyncio.wait_for(
                    self._notifications.get(), timeout=remaining
                )
            except (asyncio.TimeoutError, TimeoutError):
                raise LspTimeout(
                    "%s: no response within %.1fs" % (method, self._timeout)
                ) from None
            if isinstance(item, dict) and item.get("_crash"):
                raise self._annotate(item["_crash"])
        response = self._responses.pop(request_id)
        if "error" in response:
            err = response["error"]
            raise LspProtocolError(
                "%s -> %s: %s" % (method, err.get("code"), err.get("message"))
            )
        return response["result"]

    async def _next_notification(self, deadline):
        remaining = deadline - asyncio.get_running_loop().time()
        if remaining <= 0:
            raise LspTimeout(
                "notification: not observed within %.1fs" % self._timeout
            )
        try:
            item = await asyncio.wait_for(
                self._notifications.get(), timeout=remaining
            )
        except (asyncio.TimeoutError, TimeoutError):
            raise LspTimeout(
                "notification: not observed within %.1fs" % self._timeout
            ) from None
        if isinstance(item, dict):
            if item.get("_crash"):
                raise self._annotate(item["_crash"])
            if item.get("_wake"):
                return await self._next_notification(deadline)
        return item

    # -- session ---------------------------------------------------------

    async def start(self):
        argv = self._bin.split() if isinstance(self._bin, str) else list(self._bin)
        self._proc = await asyncio.create_subprocess_exec(
            *argv,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        self._stderr_buf = bytearray()
        self._stderr_task = asyncio.get_running_loop().create_task(
            self._drain_stderr()
        )
        self._reader_task = asyncio.get_running_loop().create_task(
            self._pump()
        )
        result = await self._request(
            "initialize",
            {
                "processId": os.getpid(),
                "rootUri": None,
                "capabilities": {},
                "workspaceFolders": None,
            },
        )
        self._capabilities = result.get("capabilities", {}) if result else {}
        self._send({"jsonrpc": "2.0", "method": "initialized", "params": {}})
        await self._stdin().drain()
        return self._capabilities

    async def did_open(self, path, source):
        uri = _path_to_uri(path)
        self._send(
            {
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "rust",
                        "version": 1,
                        "text": source,
                    }
                },
            }
        )
        await self._stdin().drain()

    async def did_save(self, path, source=None):
        params = {"textDocument": {"uri": _path_to_uri(path)}}
        if source is not None:
            params["text"] = source
        self._send(
            {"jsonrpc": "2.0", "method": "textDocument/didSave", "params": params}
        )
        await self._stdin().drain()

    async def wait_for_diagnostics(self, uri, deadline=None, skip=0):
        """Consume notifications until the (skip+1)-th publishDiagnostics
        for `uri` arrives.

        The server publishes an always-empty Tier-1 advisory on didOpen
        before the Tier-2 court verdict publish on didSave; wire order is
        FIFO (one notification queue), so `skip=1` deterministically
        drains the Tier-1 publish and returns the Tier-2 verdict -- no
        sleeps, bounded by `deadline`.
        """
        if deadline is None:
            deadline = asyncio.get_running_loop().time() + self._timeout
        skipped = 0
        while True:
            note = await self._next_notification(deadline)
            if (
                note.get("method") == "textDocument/publishDiagnostics"
                and note.get("params", {}).get("uri") == uri
            ):
                if skipped < skip:
                    skipped += 1
                    continue
                return note["params"].get("diagnostics", [])

    async def conformance_vector(self):
        # WG2 (disclosed unblock): the server's max/conformanceVector takes
        # Option<SnapshotId>; params must be null (a map deserializes as
        # -32602 invalid type: map, expected a string).
        return await self._request("max/conformanceVector", None)

    async def stop(self):
        try:
            result = await self._request("shutdown", None)
        except (LspError, asyncio.TimeoutError):
            result = None
        self._send({"jsonrpc": "2.0", "method": "exit", "params": None})
        proc = self._proc
        if proc is None:
            raise LspServerCrashed(None)
        try:
            await asyncio.wait_for(proc.wait(), timeout=self._timeout)
        except (asyncio.TimeoutError, AttributeError):
            proc.kill()
            await proc.wait()
        if self._reader_task is not None:
            self._reader_task.cancel()
            try:
                await self._reader_task
            except (asyncio.CancelledError, LspError):
                pass
        if self._stderr_task is not None:
            self._stderr_task.cancel()
            try:
                await self._stderr_task
            except (asyncio.CancelledError, LspError):
                pass
        return result

    # -- crash detection ---------------------------------------------------

    def assert_alive(self):
        if self._proc is None or self._proc.returncode is not None:
            raise LspServerCrashed(
                None if self._proc is None else self._proc.returncode
            )


def _path_to_uri(path):
    return "file://" + urllib.parse.quote(os.path.abspath(path))


async def analyze_text(
    source, server_bin, filename="bl9_snippet.rs", timeout=DEFAULT_TIMEOUT
):
    """Analyze in-memory source; spools to a temp file for a real URI."""
    tmp = tempfile.NamedTemporaryFile(
        prefix="bl9_", suffix=os.path.splitext(filename)[1] or ".rs", delete=False
    )
    try:
        tmp.write(source.encode("utf-8"))
        tmp.close()
        return await analyze(tmp.name, source=source, server_bin=server_bin,
                             timeout=timeout)
    finally:
        try:
            os.unlink(tmp.name)
        except OSError:
            pass


async def analyze(
    path, source=None, server_bin=None, timeout=DEFAULT_TIMEOUT
):
    """Full session: initialize -> didOpen -> didSave -> diagnostics ->
    max/conformanceVector -> shutdown/exit. Returns parsed dict."""
    if server_bin is None:
        server_bin = os.environ.get("BRANCHLESS_LSP_BIN")
    if not server_bin:
        raise LspError(
            "no server: pass server_bin or set BRANCHLESS_LSP_BIN"
        )
    if source is None:
        with open(path, "r", encoding="utf-8") as handle:
            source = handle.read()
    uri = _path_to_uri(path)
    client = BranchlessLspClient(server_bin, timeout=timeout)
    await client.start()
    try:
        client.assert_alive()
        await client.did_open(path, source)
        await client.did_save(path, source)
        # skip=1 drains the always-empty Tier-1 didOpen publish (FIFO ahead
        # of the didSave Tier-2 verdict) so the returned diagnostics are the
        # court verdict, not the advisory.
        diagnostics = await client.wait_for_diagnostics(uri, skip=1)
        try:
            conformance = await client.conformance_vector()
        except LspError:
            conformance = None
        receipt = None
        if isinstance(conformance, dict):
            receipt = conformance.get("receipt")
    except LspError as exc:
        raise client._annotate(exc) from None
    finally:
        await client.stop()
    return {
        "diagnostics": diagnostics,
        "conformance": conformance,
        "receipt": receipt,
        "uri": uri,
        "stderr": client.stderr_text(),
    }


def diagnostics_for_files(paths, server_bin=None, timeout=DEFAULT_TIMEOUT):
    """Sync batch adapter for the BL10 admission gate.

    Drives ONE client session for all files (never per-file servers),
    didOpen/didSave each path, collects publishDiagnostics per URI, and
    aggregates: {"<path>": [diagnostics...], ..., "n_refused": total}.

    `n_refused` counts ERROR-severity diagnostics across all files.
    Raises LspError subclasses on transport failure; refuses nothing
    itself -- severity classification belongs to the gate.
    """
    if not paths:
        return {"n_refused": 0}
    if server_bin is None:
        server_bin = os.environ.get("BRANCHLESS_LSP_BIN")
    if not server_bin:
        raise LspError("no server: pass server_bin or set BRANCHLESS_LSP_BIN")

    sources = {}
    for path in paths:
        with open(path, "r", encoding="utf-8") as handle:
            sources[path] = handle.read()

    async def _run():
        client = BranchlessLspClient(server_bin, timeout=timeout)
        await client.start()
        try:
            client.assert_alive()
            result = {}
            for path in paths:
                uri = _path_to_uri(path)
                await client.did_open(path, sources[path])
                await client.did_save(path, sources[path])
                # Same mechanism as analyze(): FIFO skip=1 deterministically
                # drops the Tier-1 didOpen publish; the next publish for the
                # URI is the didSave Tier-2 court verdict.
                result[path] = await client.wait_for_diagnostics(uri, skip=1)
            return result
        finally:
            await client.stop()

    per_file = asyncio.run(_run())
    n_refused = sum(
        1
        for diags in per_file.values()
        for d in diags
        if (d.get("severity") if isinstance(d, dict) else getattr(d, "severity", None))
        in ("ERROR", "error", 1)
    )
    out: dict = {path: list(per_file.get(path, [])) for path in paths}
    out["n_refused"] = n_refused
    return out
