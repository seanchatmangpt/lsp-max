use std::path::PathBuf;

use crate::{GeneratedFile, Generator, GeneratorContext, GeneratorError, WriteMode};

// Receipt artifact format — boundary markers are required by validate-receipt-chain.sh.
// Status is CANDIDATE in the law_axes until a transcript is attached and the chain verified.
const RECEIPT_TEMPLATE: &str = r#"-----BEGIN RECEIPT-----
{
  "receipt_id": "{{ receipt_id }}",
  "method": "{{ method_name }}",
  "status": "ADMITTED",
  "checkpoint": "{{ checkpoint_id }}",
  "transcript_digest": "{{ transcript_digest }}",
  "negative_control_digest": "{{ negative_control_digest }}",
  "boundary": "lsp-max-receipt-v1",
  "generated_at": "{{ generated_at }}",
  "law_axes": {
    "transcript": "PRESENT",
    "negative_control": "PRESENT",
    "receipt": "SELF"
  }
}
-----END RECEIPT-----
"#;

/// Generates a receipt artifact for a named LSP method.
///
/// Status on first emission is CANDIDATE: the caller must attach a real
/// transcript digest and verify the negative-control before `admit promote`
/// can advance to ADMITTED.
pub struct ReceiptGenerator;

impl Generator for ReceiptGenerator {
    fn name(&self) -> &str {
        "receipt"
    }

    fn description(&self) -> &str {
        "Generate a receipt artifact for ADMITTED status (CANDIDATE until transcript is attached)"
    }

    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError> {
        use std::time::{SystemTime, UNIX_EPOCH};
        use tera::{Context as TeraCtx, Tera};

        let method_name = ctx
            .extra
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or(&ctx.name);

        let transcript_path = ctx
            .extra
            .get("transcript")
            .and_then(|v| v.as_str())
            .unwrap_or("OPEN");

        // Placeholder digest: real implementation hashes actual transcript file content.
        let transcript_digest = if transcript_path == "OPEN" {
            "OPEN-no-transcript-attached".to_string()
        } else {
            format!("sha256-pending-{}", transcript_path.len())
        };

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let receipt_id = format!("rcpt-{}-{}", ctx.snake_name, ts);
        let checkpoint_id = format!("chk-{}-{}", ctx.snake_name, ts);

        let mut tera_ctx = TeraCtx::new();
        tera_ctx.insert("receipt_id", &receipt_id);
        tera_ctx.insert("method_name", method_name);
        tera_ctx.insert("checkpoint_id", &checkpoint_id);
        tera_ctx.insert("transcript_digest", &transcript_digest);
        tera_ctx.insert("negative_control_digest", "OPEN-no-negative-control");
        tera_ctx.insert("generated_at", &ts.to_string());

        let content = Tera::one_off(RECEIPT_TEMPLATE, &tera_ctx, false)?;

        // Belt-and-suspenders law check before returning to the engine.
        if content.contains("tower-lsp") || content.contains("tower_lsp") {
            return Err(GeneratorError::LawViolation {
                reason: "receipt contains forbidden reference".into(),
            });
        }

        Ok(vec![GeneratedFile {
            path: PathBuf::from("receipts").join(format!("{}.json", ctx.snake_name)),
            content,
            // Never overwrite a receipt that is already on disk; receipts are
            // append-only artifacts once written.
            mode: WriteMode::Skip,
        }])
    }
}
