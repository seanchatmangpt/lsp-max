use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let args = parse_args();

    let mut buf: Vec<u8> = Vec::new();

    {
        let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut buf);
        let project_id =
            builder.emit_meta_project(&args.root.to_string_lossy(), args.lang_str())?;
        builder.begin_project(project_id.clone())?;

        for path in collect_files(&args.root, &args.lang) {
            let source = fs::read_to_string(&path)?;
            let uri = path_to_uri(&path);
            match args.lang {
                Lang::Rust => lsif_rust::index_file(&source, &uri, &mut builder)?,
                Lang::TypeScript => lsif_typescript::index_file(
                    &source,
                    &uri,
                    args.package_name.as_deref(),
                    &mut builder,
                )?,
                Lang::Both => match path.extension().and_then(|e| e.to_str()) {
                    Some("rs") => lsif_rust::index_file(&source, &uri, &mut builder)?,
                    Some("ts") | Some("tsx") => lsif_typescript::index_file(
                        &source,
                        &uri,
                        args.package_name.as_deref(),
                        &mut builder,
                    )?,
                    _ => {}
                },
            }
        }

        builder.end_project(project_id)?;
    }

    let final_bytes: Vec<u8> = if args.no_link {
        buf
    } else {
        let mut linked: Vec<u8> = Vec::with_capacity(buf.len() + 4096);
        lsif_linker::link(&buf, &mut linked)?;
        linked
    };

    match &args.out {
        Some(path) => {
            let file = fs::File::create(path)?;
            let mut w = BufWriter::new(file);
            w.write_all(&final_bytes)?;
            eprintln!(
                "lsp-max-lsif: wrote {} bytes to {}",
                final_bytes.len(),
                path.display()
            );
        }
        None => {
            io::stdout().write_all(&final_bytes)?;
        }
    }

    Ok(())
}

// ── Lang ──────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum Lang {
    Rust,
    TypeScript,
    Both,
}

#[derive(Debug)]
struct Args {
    root: PathBuf,
    out: Option<PathBuf>,
    lang: Lang,
    package_name: Option<String>,
    no_link: bool,
}

impl Args {
    fn lang_str(&self) -> &'static str {
        match self.lang {
            Lang::Rust => "rust",
            Lang::TypeScript => "typescript",
            Lang::Both => "mixed",
        }
    }
}

fn parse_args() -> Args {
    let mut root = PathBuf::from(".");
    let mut out = None;
    let mut lang = Lang::Both;
    let mut package_name = None;
    let mut no_link = false;
    let mut iter = std::env::args().skip(1).peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => root = iter.next().map(PathBuf::from).unwrap_or(root),
            "--out" => out = iter.next().map(PathBuf::from),
            "--lang" => match iter.next().as_deref() {
                Some("rust") => lang = Lang::Rust,
                Some("typescript") => lang = Lang::TypeScript,
                _ => lang = Lang::Both,
            },
            "--package" => package_name = iter.next(),
            "--no-link" => no_link = true,
            "--help" | "-h" => {
                eprintln!(
                    "Usage: lsp-max-lsif [--root DIR] [--out FILE]\n\
                     [--lang rust|typescript|both] [--package NAME] [--no-link]\n\n\
                     Emits LSIF JSONL to FILE (or stdout).\n\
                     --no-link  skip the cross-file moniker linker second pass"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }

    Args {
        root,
        out,
        lang,
        package_name,
        no_link,
    }
}

// ── File collection ───────────────────────────────────────────────────────────

fn collect_files(root: &Path, lang: &Lang) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_recursive(root, lang, &mut paths);
    paths.sort();
    paths
}

fn collect_recursive(dir: &Path, lang: &Lang, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "target" | "node_modules" | ".git" | "dist" | "build") {
                continue;
            }
            collect_recursive(&path, lang, out);
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let include = match lang {
                Lang::Rust => ext == "rs",
                Lang::TypeScript => matches!(ext, "ts" | "tsx"),
                Lang::Both => matches!(ext, "rs" | "ts" | "tsx"),
            };
            if include {
                out.push(path);
            }
        }
    }
}

fn path_to_uri(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    format!("file://{}", abs.display())
}
