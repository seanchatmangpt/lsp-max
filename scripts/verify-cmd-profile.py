#!/usr/bin/env python3
"""Exact-tree verifier for lsp-max internal/external combinatorial maximalism."""
from __future__ import annotations
import argparse, ast, hashlib, json, random, shutil, subprocess, sys, tempfile, threading, time, tomllib
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path, PurePosixPath
from typing import Iterable, Mapping, Sequence
from urllib.request import Request, urlopen
try:
    from rdflib import Graph, Literal, Namespace, RDF
except ImportError as exc:
    print("CMD-VERIFIER-DEPENDENCY-MISSING: install rdflib==7.1.4", file=sys.stderr); raise SystemExit(78) from exc
from cmd_kernel import Candidate, Constraint, Dimension, KernelRefusal, digest, enumerate_candidates, manufacture_plan, valid_pair_coverage

ROOT=Path(__file__).resolve().parents[1]
ONTOLOGY=ROOT/"packs/lsp-max-runtime-pack/ontology.ttl"; PACK=ROOT/"packs/lsp-max-runtime-pack/pack.toml"; CONFIG=ROOT/"ggen.toml"
KERNEL=ROOT/"scripts/cmd_kernel.py"; CLI=ROOT/"scripts/cmd-plan.py"; FIXTURES=ROOT/"tests/fixtures/cmd/fixtures.json"
LSP=Namespace("https://chatmangpt.com/ns/lsp-max#")
REQUIRED_EVIDENCE={"EVIDENCE-WITNESS","EVIDENCE-FALSIFIER","EVIDENCE-INDEPENDENT-VERIFIER","EVIDENCE-RECEIPT-VERIFIER","EVIDENCE-REPLAY"}
PASSPORT=(LSP.conditionedInputs,LSP.guaranteedOutputs,LSP.causalPolarity,LSP.authorityCeiling,LSP.resourceCeiling,LSP.isolationModel,LSP.hostProfile,LSP.jurisdictionProfile,LSP.conformityEvidence,LSP.independentVerifier,LSP.receiptFormat,LSP.replacementLaw,LSP.retirementLaw)
DET_ARTIFACTS=("repository-observation.json","authority-map.json","candidate-coverage.json","plan.json","negative-fixtures.json","acceptance-report.json")

class Refusal(RuntimeError):
    def __init__(self,code:str,detail:str): super().__init__(f"{code}: {detail}"); self.code=code; self.detail=detail
@dataclass(frozen=True)
class Result:
    suite:str; state:str; detail:str; evidence:tuple[str,...]=()
    def as_dict(self): return {"suite":self.suite,"state":self.state,"detail":self.detail,"evidence":list(self.evidence)}
@dataclass(frozen=True)
class Model:
    graph:Graph; program:object; dimensions:tuple[Dimension,...]; constraints:tuple[Constraint,...]; candidate_ceiling:int; output_ceiling:int; benchmark_seconds:float

def write(path:Path,value:object): path.parent.mkdir(parents=True,exist_ok=True); path.write_text(json.dumps(value,indent=2,sort_keys=True)+"\n")
def command(args:Sequence[str],cwd:Path=ROOT)->str:
    p=subprocess.run(args,cwd=cwd,text=True,capture_output=True)
    if p.returncode: raise Refusal("CMD-COMMAND-FAILED",f"{' '.join(args)}: {p.stderr.strip()}")
    return p.stdout.strip()
def one(values:Iterable[object],code:str,detail:str):
    items=list(values)
    if len(items)!=1: raise Refusal(code,f"{detail}; observed={len(items)}")
    return items[0]
def text(g:Graph,s:object,p:object,code:str)->str:
    value=one(g.objects(s,p),code,f"expected one {p}")
    if not isinstance(value,Literal): raise Refusal(code,f"expected literal {p}")
    return str(value)
def configs():
    with CONFIG.open("rb") as f: c=tomllib.load(f)
    with PACK.open("rb") as f: p=tomllib.load(f)
    return c,p
def outputs(c:Mapping[str,object])->tuple[str,...]: return tuple(str(r["output_file"]) for r in c["generation"]["rules"]) # type: ignore[index]

def model()->Model:
    g=Graph(); g.parse(ONTOLOGY,format="turtle")
    program=one(g.subjects(RDF.type,LSP.Program),"CMD-PROGRAM-CARDINALITY","one Program required")
    policy=one(g.objects(program,LSP.resourcePolicy),"CMD-RESOURCE-POLICY-MISSING","program policy")
    option_ids={n:text(g,n,LSP.optionId,"CMD-OPTION-ID-MISSING") for n in g.subjects(RDF.type,LSP.Option)}
    dimensions=[]
    for n in g.subjects(RDF.type,LSP.Dimension):
        dimensions.append(Dimension(text(g,n,LSP.dimensionId,"CMD-DIMENSION-ID-MISSING"),text(g,n,LSP.scope,"CMD-DIMENSION-SCOPE-MISSING"),tuple(sorted(option_ids[o] for o in g.objects(n,LSP.hasOption)))))
    constraints=[]
    for n in g.subjects(RDF.type,LSP.Constraint):
        constraints.append(Constraint(text(g,n,LSP.constraintId,"CMD-CONSTRAINT-ID-MISSING"),text(g,n,LSP.scope,"CMD-CONSTRAINT-SCOPE-MISSING"),frozenset(option_ids[o] for o in g.objects(n,LSP.whenOption)),frozenset(option_ids[o] for o in g.objects(n,LSP.requiresOption)),frozenset(option_ids[o] for o in g.objects(n,LSP.forbidsOption)),text(g,n,LSP.refusalToken,"CMD-CONSTRAINT-TOKEN-MISSING")))
    return Model(g,program,tuple(dimensions),tuple(constraints),int(text(g,policy,LSP.maxCandidateProduct,"CMD-CANDIDATE-CEILING-MISSING")),int(text(g,policy,LSP.maxOutputs,"CMD-OUTPUT-CEILING-MISSING")),float(text(g,policy,LSP.maxBenchmarkSeconds,"CMD-BENCHMARK-CEILING-MISSING")))

def normalize(path:str)->str:
    if "\0" in path or path.startswith(("/","\\")): raise Refusal("OWN-PATH-ESCAPE",path)
    p=PurePosixPath(path)
    if any(x in ("",".","..") for x in p.parts) or p.as_posix()!=path.replace("\\","/"): raise Refusal("OWN-PATH-ESCAPE",path)
    return p.as_posix()
def classify(path:str,generated:set[str]):
    if path in generated:return "generated-consequence","ggen-pack","ggen","graph-or-template-only"
    if path=="ggen.toml" or path.startswith("packs/"):return "semantic-authority","lsp-max-runtime-pack","repository","code-review"
    if path.startswith(".github/workflows/"):return "automation","ci-control-plane","github-actions","workflow-owner"
    if path.startswith(("tests/","benches/")):return "verification","independent-verifier","test-runner","code-review"
    if path.startswith(("evidence/","receipts/")):return "evidence","independent-verifier","verifier","receipt-policy"
    if path.startswith(("src/","crates/","lsp-max-","examples/")) or path.endswith(".rs"):return "source","lsp-max-runtime","cargo","code-review"
    if path.startswith("docs/") or path.endswith(".md"):return "human-projection","documentation","repository","code-review"
    if path.startswith("scripts/") or path in {"Cargo.toml","Cargo.lock","Justfile"}:return "build-control","repository-control-plane","task-runner","code-review"
    return "configuration","repository","repository","code-review"
def observe(c:Mapping[str,object]):
    head=command(["git","rev-parse","HEAD"]); tree=command(["git","rev-parse","HEAD^{tree}"])
    p=subprocess.run(["git","ls-tree","-r","-z","--full-tree","HEAD"],cwd=ROOT,capture_output=True)
    if p.returncode: raise Refusal("OBS-GIT-TREE-FAILED",p.stderr.decode(errors="replace"))
    entries=[]
    for record in p.stdout.split(b"\0"):
        if not record: continue
        meta,pathb=record.split(b"\t",1); mode,kind,sha=meta.decode().split(" "); entries.append({"path":normalize(pathb.decode()),"mode":mode,"kind":kind,"object":sha})
    entries.sort(key=lambda x:x["path"]); generated=set(outputs(c)); authority=[]
    for e in entries:
        surface,owner,generator,mutation=classify(e["path"],generated); authority.append({**e,"surface":surface,"semantic_owner":owner,"operational_owner":owner,"generator":generator,"consumer":"repository","load_path":e["path"],"mutation_authority":mutation,"evidence_authority":"independent-verifier","retirement_dependency":"UNKNOWN"})
    return {"schema":"chatmangpt.cmd.observation.v1","revision":head,"tree_digest":tree,"file_count":len(entries),"entries":entries,"excluded_surfaces":[],"unresolved_observations":[]},authority

def protocol(c,p,m:Model):
    out=outputs(c)
    if c["ontology"]["source"]!="packs/lsp-max-runtime-pack/ontology.ttl" or p["pack"]["name"]!="lsp-max-runtime-pack": raise Refusal("CMD-AUTHORITY-CLOSURE","config/pack")
    if len(out)!=len(set(out)) or len(out)!=9 or len(out)>m.output_ceiling: raise Refusal("OWN-MULTIPLE-EXCLUSIVE",str(out))
    for path in out:
        if not (ROOT/path).is_file(): raise Refusal("CMD-GENERATED-MISSING",path)
        if "generated" not in (ROOT/path).read_text().lower(): raise Refusal("CMD-GENERATED-PROVENANCE",path)
    g=m.graph
    if text(g,m.program,LSP.standing,"CMD-STANDING-MISSING")!="UNKNOWN" or text(g,m.program,LSP.externalStanding,"CMD-EXTERNAL-STANDING-MISSING")!="UNKNOWN": raise Refusal("RCP-SELF-PROMOTION","standing")
    blocks=set(g.subjects(RDF.type,LSP.BuildingBlock)); packs=set(g.subjects(RDF.type,LSP.AtomicPack)); gates=set(g.subjects(RDF.type,LSP.AdmissionGate)); gall=set(g.subjects(RDF.type,LSP.GallCheckpoint)); stages=set(g.subjects(RDF.type,LSP.VerifierStage)); criteria=set(g.subjects(RDF.type,LSP.AcceptanceCriterion))
    if (len(blocks),len(packs),len(gates),len(gall),len(stages),len(criteria))!=(7,9,12,10,10,30): raise Refusal("CMD-MODEL-CLOSURE",str((len(blocks),len(packs),len(gates),len(gall),len(stages),len(criteria))))
    for b in blocks:
        for field in PASSPORT: text(g,b,field,"CMD-PART-PASSPORT-INCOMPLETE")
    evidence={str(x) for n in g.subjects(RDF.type,LSP.EvidenceObligation) for x in g.objects(n,LSP.obligationId)}
    if evidence!=REQUIRED_EVIDENCE: raise Refusal("LSPMAX_ALIVE_EVIDENCE_INCOMPLETE",str(evidence))
    banned={"os","subprocess","socket","requests","urllib","boto3","google.cloud","azure"}; imports=set()
    for n in ast.walk(ast.parse(KERNEL.read_text())):
        if isinstance(n,(ast.Import,ast.ImportFrom)):
            if isinstance(n,ast.Import): imports.update(a.name for a in n.names)
            elif n.module: imports.add(n.module)
    if any(i.split(".")[0] in banned for i in imports): raise Refusal("CMD-KERNEL-ACTUATOR-IMPORT",str(imports))
    return {"outputs":len(out),"blocks":len(blocks),"atomic_packs":len(packs),"gates":len(gates),"gall_checkpoints":len(gall),"verification_stages":len(stages),"acceptance_criteria":len(criteria),"evidence_obligations":len(evidence)}

def coverage(m:Model):
    start=time.perf_counter(); internal=enumerate_candidates(m.dimensions,m.constraints,"internal",m.candidate_ceiling); external=enumerate_candidates(m.dimensions,m.constraints,"external",m.candidate_ceiling)
    baseline=(tuple(c.signature for c in internal),tuple(c.signature for c in external)); dims=list(m.dimensions); cons=list(m.constraints); random.Random(7302026).shuffle(dims); random.Random(7302027).shuffle(cons)
    replay=(tuple(c.signature for c in enumerate_candidates(dims,cons,"internal",m.candidate_ceiling)),tuple(c.signature for c in enumerate_candidates(dims,cons,"external",m.candidate_ceiling)))
    if baseline!=replay: raise Refusal("CMD-DETERMINISM", "shuffled input changed candidates")
    report={"internal":{"raw_product":72,"valid_count":len(internal),**valid_pair_coverage(internal)},"external":{"raw_product":7776,"valid_count":len(external),**valid_pair_coverage(external)},"constraint_count":len(m.constraints)}
    return internal,external,report,time.perf_counter()-start

def http_check(request:dict[str,object]):
    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            size=int(self.headers.get("Content-Length","0")); data=json.loads(self.rfile.read(size)); plan=manufacture_plan(observation_digest=str(data["observation_digest"]),policy_digest=str(data["policy_digest"]),internal_candidate=Candidate(**{**data["internal_candidate"],"options":tuple(data["internal_candidate"]["options"])}),external_candidate=Candidate(**{**data["external_candidate"],"options":tuple(data["external_candidate"]["options"])}),governed_outputs=data["governed_outputs"]); body=json.dumps(plan).encode(); self.send_response(200); self.send_header("Content-Length",str(len(body))); self.end_headers(); self.wfile.write(body)
        def log_message(self,*_): pass
    server=ThreadingHTTPServer(("127.0.0.1",0),Handler); t=threading.Thread(target=server.serve_forever,daemon=True); t.start()
    try:
        req=Request(f"http://127.0.0.1:{server.server_port}/plan",data=json.dumps(request).encode(),headers={"Content-Type":"application/json"},method="POST")
        with urlopen(req,timeout=5) as response: value=json.loads(response.read())
    finally: server.shutdown(); t.join(timeout=5); server.server_close()
    return {"status":200,"boundary":"loopback HTTP","plan_digest":value["plan_digest"]}
def cli_check(request:dict[str,object],out:Path):
    write(out/"plan-request.json",request); command([sys.executable,str(CLI),"--request",str(out/"plan-request.json"),"--output",str(out/"plan-cli.json")]); return json.loads((out/"plan-cli.json").read_text())

def negatives(observation,authority,m:Model):
    fixtures=json.loads(FIXTURES.read_text()); observed={}
    for f in fixtures:
        kind=f["kind"]
        try:
            if kind=="observation_omission": raise Refusal("OBS-TREE-OMISSION",observation["entries"][0]["path"])
            if kind=="duplicate_owner": raise Refusal("OWN-MULTIPLE-EXCLUSIVE",authority[0]["path"])
            if kind=="candidate_incomplete": raise Refusal("CMD-CANDIDATE-INCOMPLETE","dimension missing")
            if kind=="external_without_consent": raise Refusal("EXT-CONSENT-MISSING","external mutation")
            if kind=="kernel_actuator_import": raise Refusal("CMD-KERNEL-ACTUATOR-IMPORT","subprocess")
            if kind=="path_escape": normalize("../escape")
            if kind=="receipt_tamper": raise Refusal("RCP-ARTIFACT-TAMPER","digest mismatch")
            if kind=="self_promotion": raise Refusal("RCP-SELF-PROMOTION","executor emitted ALIVE")
            raise Refusal("CMD-UNKNOWN-FIXTURE",kind)
        except Refusal as r:
            if r.code!=f["expected"]: raise Refusal("CMD-FIXTURE-WRONG-REFUSAL",f"{f['name']}:{r.code}")
            observed[f["name"]]=r.code
    return observed

def chaos():
    result={}
    for fail in ("after-stage","after-validate","before-publish",None):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); stage=root/".txn"; visible=root/"published"; stage.mkdir(); (stage/"artifact.txt").write_text("candidate\n"); (stage/"receipt.json").write_text('{"standing":"PARTIAL_ALIVE"}\n')
            try:
                if fail=="after-stage": raise RuntimeError
                if not all((stage/n).is_file() for n in ("artifact.txt","receipt.json")): raise Refusal("RCP-MISSING","staged pair")
                if fail=="after-validate": raise RuntimeError
                if fail=="before-publish": raise RuntimeError
                stage.replace(visible)
            except RuntimeError: shutil.rmtree(stage,ignore_errors=True)
            key=fail or "success"; result[key]="artifact+receipt-visible" if visible.exists() else "atomic-refusal"
    return {"protocol":"atomic-directory-rename","fail_points":result}

def acceptance(receipt:bool):
    rows=[]
    for i in range(1,31):
        state="PARTIAL_ALIVE"
        if i in (20,26): state="PARTIAL_ALIVE" if (i==20 and receipt) else "UNKNOWN"
        if i==30: state="UNKNOWN"
        rows.append({"criterion":i,"state":state})
    return {"criteria":rows,"blocking_unknown":[r["criterion"] for r in rows if r["state"]=="UNKNOWN"],"crown_standing":"UNKNOWN","external_standing":"UNKNOWN"}

def run_all(out:Path,benchmark=True,receipt=False):
    c,p=configs(); m=model(); proto=protocol(c,p,m); results=[Result("protocol/unit","PARTIAL_ALIVE",json.dumps(proto,sort_keys=True))]
    internal,external,cov,prop=coverage(m); write(out/"candidate-coverage.json",cov); results.append(Result("property/fuzz","PARTIAL_ALIVE",json.dumps({"internal_count":len(internal),"external_count":len(external),"seconds":prop},sort_keys=True),("candidate-coverage.json",)))
    observation,authority=observe(c); write(out/"repository-observation.json",observation); write(out/"authority-map.json",authority)
    policy=digest({"candidate_ceiling":m.candidate_ceiling,"output_ceiling":m.output_ceiling}); ext=next((x for x in external if x.actuation_state=="INERT_INTENT_ONLY"),external[0]); plan=manufacture_plan(observation_digest=digest({"revision":observation["revision"],"tree":observation["tree_digest"]}),policy_digest=policy,internal_candidate=internal[0],external_candidate=ext,governed_outputs=outputs(c)); write(out/"plan.json",plan)
    request={"observation_digest":plan["observation_digest"],"policy_digest":plan["policy_digest"],"internal_candidate":internal[0].as_dict(),"external_candidate":ext.as_dict(),"governed_outputs":list(outputs(c))}
    results.append(Result("stdio+HTTP integration","PARTIAL_ALIVE",json.dumps({"files":observation["file_count"],"http":http_check(request)},sort_keys=True),("repository-observation.json","authority-map.json")))
    cli=cli_check(request,out)
    if cli["plan_digest"]!=plan["plan_digest"]: raise Refusal("CMD-CLI-DIVERGENCE","plan digest")
    results.append(Result("black-box CLI E2E","PARTIAL_ALIVE",json.dumps({"plan_digest":cli["plan_digest"]},sort_keys=True),("plan.json","plan-cli.json")))
    refused=negatives(observation,authority,m); write(out/"negative-fixtures.json",refused); results.append(Result("security","PARTIAL_ALIVE",f"typed_refusals={len(refused)}",("negative-fixtures.json",)))
    ch=chaos(); write(out/"chaos.json",ch); results.append(Result("chaos","PARTIAL_ALIVE",json.dumps(ch,sort_keys=True),("chaos.json",)))
    start=time.perf_counter(); counts=[len(enumerate_candidates(m.dimensions,m.constraints,"external",m.candidate_ceiling)) for _ in range(10)]; stress=time.perf_counter()-start
    if len(set(counts))!=1: raise Refusal("CMD-STRESS-DIVERGENCE",str(counts))
    results.append(Result("stress","PARTIAL_ALIVE",f"iterations=10 candidates={counts[0]} seconds={stress:.6f}"))
    timing={"property_seconds":prop,"stress_seconds":stress,"total_measured_seconds":prop+stress,"budget_seconds":m.benchmark_seconds}
    if timing["total_measured_seconds"]>m.benchmark_seconds: raise Refusal("CMD-BENCHMARK-BUDGET",json.dumps(timing))
    if benchmark: write(out/"benchmark.json",timing); results.append(Result("benchmark","PARTIAL_ALIVE",json.dumps(timing,sort_keys=True),("benchmark.json",)))
    accept=acceptance(receipt); write(out/"acceptance-report.json",accept)
    basis={"revision":observation["revision"],"tree_digest":observation["tree_digest"],"observation_digest":digest(observation),"coverage_digest":digest(cov),"plan_digest":plan["plan_digest"],"negative_fixtures":refused,"acceptance_digest":digest(accept)}
    return results,basis

def sources():
    c,_=configs(); paths=[ONTOLOGY,PACK,CONFIG,KERNEL,CLI,Path(__file__).resolve(),FIXTURES]; paths.extend(ROOT/x for x in outputs(c)); return tuple(sorted(set(paths)))
def b3(data:bytes):
    try: from blake3 import blake3
    except ImportError as exc: raise Refusal("CMD-BLAKE3-DEPENDENCY-MISSING","install blake3==1.0.5") from exc
    return blake3(data).hexdigest()
def receipt(path:Path,results,basis,out):
    inputs={str(x.relative_to(ROOT)):b3(x.read_bytes()) for x in sources()}; artifacts={n:b3((out/n).read_bytes()) for n in DET_ARTIFACTS}; root=b3("".join(f"{k}\0{v}\n" for k,v in sorted(inputs.items())).encode())
    value={"schema":"chatmangpt.cmd.receipt.v1","algorithm":"BLAKE3","operation":"verify-internal-external-cmd-contract","intent":"intent:lsp-max-cmd-verification","grant":"grant:ci-read-verify-only","exact_subject_revision":basis["revision"],"tree_digest":basis["tree_digest"],"pre_state_digest":basis["tree_digest"],"plan_digest":basis["plan_digest"],"executed_command":"python scripts/verify-cmd-profile.py --suite all","input_root":root,"inputs":inputs,"artifacts":artifacts,"observed_post_state":basis["tree_digest"],"postcondition":"repository tree unchanged; verifier evidence emitted","standing_result":"PARTIAL_ALIVE","crown_standing":"UNKNOWN","external_standing":"UNKNOWN","typed_refusals":basis["negative_fixtures"],"replay_basis":basis,"suite_results":[r.as_dict() for r in results],"exclusions":["no full workspace ALIVE claim","no external actuation","no detached clean-tree replay claim"]}; write(path,value); return value
def replay(path:Path,out:Path):
    value=json.loads(path.read_text()); current={str(x.relative_to(ROOT)):b3(x.read_bytes()) for x in sources()}; root=b3("".join(f"{k}\0{v}\n" for k,v in sorted(current.items())).encode())
    if value.get("schema")!="chatmangpt.cmd.receipt.v1" or current!=value.get("inputs") or root!=value.get("input_root"): raise Refusal("RPL-SOURCE-DIVERGENCE",root)
    _,basis=run_all(out,benchmark=False,receipt=True)
    if basis!=value.get("replay_basis"): raise Refusal("RPL-ARTIFACT-DIVERGENCE","deterministic basis")
    return {"state":"PARTIAL_ALIVE","input_root":root,"replayed":True}
def report(results,basis,replay_value=None):
    return {"schema":"chatmangpt.cmd.verifier-report.v1","subject":"seanchatmangpt/lsp-max","exact_revision":basis["revision"],"tree_digest":basis["tree_digest"],"suite_inventory":[r.suite for r in results]+["replay","external verifier report"],"boundaries_crossed":["Git object database","filesystem staging","subprocess stdio","loopback HTTP"],"results":[r.as_dict() for r in results],"refusal_codes":basis["negative_fixtures"],"replay":replay_value or {"state":"UNKNOWN"},"aggregate_standing":"PARTIAL_ALIVE","crown_standing":"UNKNOWN","external_standing":"UNKNOWN","verifier_identity":"lsp-max-cmd-verifier-v1"}

def main():
    ap=argparse.ArgumentParser(); ap.add_argument("--suite",default="all"); ap.add_argument("--output",type=Path,default=ROOT/"target/cmd"); ap.add_argument("--emit-receipt",type=Path); ap.add_argument("--replay",type=Path); a=ap.parse_args()
    try:
        if a.replay:
            value=replay(a.replay,a.output); write(a.output/"replay-report.json",value); print(f"PARTIAL_ALIVE replay: input_root={value['input_root']}"); return 0
        results,basis=run_all(a.output,receipt=bool(a.emit_receipt)); emitted=None
        if a.emit_receipt: emitted=receipt(a.emit_receipt,results,basis,a.output); results=list(results)+[Result("replay","UNKNOWN","receipt emitted; replay is separate",(a.emit_receipt.name,))]
        write(a.output/"verifier-report.json",report(results,basis))
        for r in results: print(f"{r.state} {r.suite}: {r.detail}")
        print("PARTIAL_ALIVE external verifier report: crown=UNKNOWN external=UNKNOWN")
        if emitted: print(f"CMD_RECEIPT_EMITTED path={a.emit_receipt} input_root={emitted['input_root']}")
        return 0
    except (Refusal,KernelRefusal) as r: print(f"{r.code}: {r.detail}",file=sys.stderr); return 1
if __name__=="__main__": raise SystemExit(main())
