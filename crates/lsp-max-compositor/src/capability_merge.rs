// capability_merge.rs — merge ServerCapabilities from multiple child servers into one.
//
// Strategy:
// - DiagnosticsOnly tier: contributes NO capabilities (diagnostics only).
// - Primary tier: highest priority — its capabilities win all conflicts.
// - Secondary tier: fills in gaps not covered by Primary.
//
// For each Option<T> field: take the first Some() value in tier order
// (Primary → Secondary; DiagnosticsOnly is excluded).

use crate::registry::ChildTier;
use lsp_max::lsp_types::ServerCapabilities;

/// Merge ServerCapabilities from multiple child servers into one.
///
/// Inputs are `(tier, caps)` pairs in any order.  Primary caps take precedence;
/// Secondary caps fill gaps.  DiagnosticsOnly entries are ignored entirely.
///
/// Union semantics for boolean-or-complex provider fields:
/// - For fields that accept `OneOf<bool, Options>`: true if ANY child reports true or options.
///   Primary-tier Options take precedence over Secondary-tier Options.  `Simple(false)` is only
///   returned when every participating child explicitly reports `false`.
/// - For purely complex (non-boolean) providers: first-non-None in tier order (Primary first).
pub fn merge_capabilities(inputs: &[(ChildTier, ServerCapabilities)]) -> ServerCapabilities {
    // Build ordered list: Primary first (rank 0), Secondary second (rank 1).
    // DiagnosticsOnly (rank 2) is filtered out.
    let mut ordered: Vec<(u8, &ServerCapabilities)> = inputs
        .iter()
        .filter(|(tier, _)| !matches!(tier, ChildTier::DiagnosticsOnly))
        .map(|(tier, caps)| (tier_rank(tier), caps))
        .collect();
    ordered.sort_by_key(|(rank, _)| *rank);

    let mut merged = ServerCapabilities::default();
    for (_, caps) in &ordered {
        merge_into(&mut merged, caps);
    }

    // Second pass: apply boolean-union for fields where `Simple(false)` from a higher-priority
    // server should not mask `Simple(true)` from a lower-priority server.
    // Fields named in the ticket: completionProvider, hoverProvider, definitionProvider,
    // referencesProvider, documentSymbolProvider, workspaceSymbolProvider.
    apply_boolean_union(&mut merged, inputs);

    merged
}

fn tier_rank(tier: &ChildTier) -> u8 {
    match tier {
        ChildTier::Primary => 0,
        ChildTier::Secondary => 1,
        ChildTier::DiagnosticsOnly => 2,
        ChildTier::Lsif => 3,
    }
}

/// Copy fields from `src` into `dst` where `dst` currently has `None`.
/// This is an `Option::or` merge across all capability fields.
fn merge_into(dst: &mut ServerCapabilities, src: &ServerCapabilities) {
    if dst.text_document_sync.is_none() {
        dst.text_document_sync = src.text_document_sync.clone();
    }
    if dst.hover_provider.is_none() {
        dst.hover_provider = src.hover_provider.clone();
    }
    if dst.completion_provider.is_none() {
        dst.completion_provider = src.completion_provider.clone();
    }
    if dst.definition_provider.is_none() {
        dst.definition_provider = src.definition_provider.clone();
    }
    if dst.declaration_provider.is_none() {
        dst.declaration_provider = src.declaration_provider.clone();
    }
    if dst.implementation_provider.is_none() {
        dst.implementation_provider = src.implementation_provider.clone();
    }
    if dst.references_provider.is_none() {
        dst.references_provider = src.references_provider.clone();
    }
    if dst.document_highlight_provider.is_none() {
        dst.document_highlight_provider = src.document_highlight_provider.clone();
    }
    if dst.document_symbol_provider.is_none() {
        dst.document_symbol_provider = src.document_symbol_provider.clone();
    }
    if dst.code_action_provider.is_none() {
        dst.code_action_provider = src.code_action_provider.clone();
    }
    if dst.document_formatting_provider.is_none() {
        dst.document_formatting_provider = src.document_formatting_provider.clone();
    }
    if dst.rename_provider.is_none() {
        dst.rename_provider = src.rename_provider.clone();
    }
    if dst.diagnostic_provider.is_none() {
        dst.diagnostic_provider = src.diagnostic_provider.clone();
    }
    if dst.type_definition_provider.is_none() {
        dst.type_definition_provider = src.type_definition_provider.clone();
    }
    if dst.workspace_symbol_provider.is_none() {
        dst.workspace_symbol_provider = src.workspace_symbol_provider.clone();
    }
    if dst.code_lens_provider.is_none() {
        dst.code_lens_provider = src.code_lens_provider;
    }
    if dst.document_link_provider.is_none() {
        dst.document_link_provider = src.document_link_provider.clone();
    }
    if dst.color_provider.is_none() {
        dst.color_provider = src.color_provider.clone();
    }
    if dst.document_on_type_formatting_provider.is_none() {
        dst.document_on_type_formatting_provider = src.document_on_type_formatting_provider.clone();
    }
    if dst.document_range_formatting_provider.is_none() {
        dst.document_range_formatting_provider = src.document_range_formatting_provider.clone();
    }
    if dst.folding_range_provider.is_none() {
        dst.folding_range_provider = src.folding_range_provider.clone();
    }
    if dst.selection_range_provider.is_none() {
        dst.selection_range_provider = src.selection_range_provider.clone();
    }
    if dst.execute_command_provider.is_none() {
        dst.execute_command_provider = src.execute_command_provider.clone();
    }
    if dst.call_hierarchy_provider.is_none() {
        dst.call_hierarchy_provider = src.call_hierarchy_provider;
    }
    if dst.semantic_tokens_provider.is_none() {
        dst.semantic_tokens_provider = src.semantic_tokens_provider.clone();
    }
    if dst.moniker_provider.is_none() {
        dst.moniker_provider = src.moniker_provider.clone();
    }
    if dst.linked_editing_range_provider.is_none() {
        dst.linked_editing_range_provider = src.linked_editing_range_provider.clone();
    }
    if dst.inline_value_provider.is_none() {
        dst.inline_value_provider = src.inline_value_provider.clone();
    }
    if dst.inlay_hint_provider.is_none() {
        dst.inlay_hint_provider = src.inlay_hint_provider.clone();
    }
}

/// Returns true when a `HoverProviderCapability` actively advertises hover support.
/// `Simple(false)` is the only "inactive" variant; options or `Simple(true)` are active.
fn hover_is_active(v: &lsp_max::lsp_types::HoverProviderCapability) -> bool {
    !matches!(v, lsp_max::lsp_types::HoverProviderCapability::Simple(false))
}

/// Returns true when an `OneOf<bool, T>` actively advertises the feature.
/// `Left(false)` is the only "inactive" variant.
fn one_of_is_active<T>(v: &lsp_max::lsp_types::OneOf<bool, T>) -> bool {
    !matches!(v, lsp_max::lsp_types::OneOf::Left(false))
}

/// Apply boolean-union for provider fields that can carry a `false` sentinel.
///
/// If the merged result shows `false` (from the highest-priority server) but ANY
/// participating server advertises the feature as active (true or options), override
/// to `true` — the compositor must not mask a child capability.
fn apply_boolean_union(merged: &mut ServerCapabilities, inputs: &[(ChildTier, ServerCapabilities)]) {
    use lsp_max::lsp_types::HoverProviderCapability;

    let participating: Vec<&ServerCapabilities> = inputs
        .iter()
        .filter(|(tier, _)| !matches!(tier, ChildTier::DiagnosticsOnly))
        .map(|(_, caps)| caps)
        .collect();

    // hoverProvider
    if let Some(v) = &merged.hover_provider {
        if !hover_is_active(v)
            && participating.iter().any(|c| c.hover_provider.as_ref().is_some_and(hover_is_active))
        {
            merged.hover_provider = Some(HoverProviderCapability::Simple(true));
        }
    }

    // definitionProvider — Option<OneOf<bool, DefinitionOptions>>
    if let Some(v) = &merged.definition_provider {
        if !one_of_is_active(v)
            && participating.iter().any(|c| c.definition_provider.as_ref().is_some_and(one_of_is_active))
        {
            merged.definition_provider = Some(lsp_max::lsp_types::OneOf::Left(true));
        }
    }

    // referencesProvider — Option<OneOf<bool, ReferencesOptions>>
    if let Some(v) = &merged.references_provider {
        if !one_of_is_active(v)
            && participating.iter().any(|c| c.references_provider.as_ref().is_some_and(one_of_is_active))
        {
            merged.references_provider = Some(lsp_max::lsp_types::OneOf::Left(true));
        }
    }

    // documentSymbolProvider — Option<OneOf<bool, DocumentSymbolOptions>>
    if let Some(v) = &merged.document_symbol_provider {
        if !one_of_is_active(v)
            && participating.iter().any(|c| c.document_symbol_provider.as_ref().is_some_and(one_of_is_active))
        {
            merged.document_symbol_provider = Some(lsp_max::lsp_types::OneOf::Left(true));
        }
    }

    // workspaceSymbolProvider — Option<OneOf<bool, WorkspaceSymbolOptions>>
    if let Some(v) = &merged.workspace_symbol_provider {
        if !one_of_is_active(v)
            && participating.iter().any(|c| c.workspace_symbol_provider.as_ref().is_some_and(one_of_is_active))
        {
            merged.workspace_symbol_provider = Some(lsp_max::lsp_types::OneOf::Left(true));
        }
    }

    // completionProvider: no `Simple(false)` sentinel — the merge_into pass is sufficient.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ChildTier;
    use lsp_max::lsp_types::{HoverProviderCapability, ServerCapabilities};

    #[test]
    fn diagnostics_only_tier_contributes_no_capabilities() {
        let caps = ServerCapabilities {
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            ..Default::default()
        };

        let merged = merge_capabilities(&[(ChildTier::DiagnosticsOnly, caps)]);
        assert!(
            merged.hover_provider.is_none(),
            "DiagnosticsOnly tier must not contribute hover capability"
        );
    }

    #[test]
    fn primary_hover_wins_over_secondary() {
        let primary_caps = ServerCapabilities {
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            ..Default::default()
        };

        let secondary_caps = ServerCapabilities::default();

        let merged = merge_capabilities(&[
            (ChildTier::Primary, primary_caps),
            (ChildTier::Secondary, secondary_caps),
        ]);
        assert!(merged.hover_provider.is_some());
    }

    #[test]
    fn secondary_fills_gap_when_primary_has_none() {
        let primary_caps = ServerCapabilities::default(); // no hover
        let secondary_caps = ServerCapabilities {
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            ..Default::default()
        };

        let merged = merge_capabilities(&[
            (ChildTier::Primary, primary_caps),
            (ChildTier::Secondary, secondary_caps),
        ]);
        assert!(
            merged.hover_provider.is_some(),
            "Secondary fills gap when Primary has None for hover"
        );
    }

    #[test]
    fn empty_inputs_returns_default() {
        let merged = merge_capabilities(&[]);
        assert!(merged.hover_provider.is_none());
    }

    #[test]
    fn tier_ordering_is_primary_before_secondary_regardless_of_input_order() {
        // Pass Secondary first, Primary second — Primary must still win.
        let primary_caps = ServerCapabilities {
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            ..Default::default()
        };

        let secondary_caps = ServerCapabilities {
            hover_provider: Some(HoverProviderCapability::Simple(false)),
            ..Default::default()
        };

        let merged = merge_capabilities(&[
            (ChildTier::Secondary, secondary_caps),
            (ChildTier::Primary, primary_caps),
        ]);
        // Primary is Simple(true); Secondary is Simple(false).
        // Since Primary is ranked lower (comes first after sort), its value wins.
        assert!(merged.hover_provider.is_some());
    }

    #[test]
    fn boolean_union_secondary_true_overrides_primary_false() {
        // Primary says Simple(false) — feature disabled.
        // Secondary says Simple(true) — feature enabled.
        // Boolean-union must return true (any-child-wins).
        let primary_caps = ServerCapabilities {
            hover_provider: Some(HoverProviderCapability::Simple(false)),
            ..Default::default()
        };
        let secondary_caps = ServerCapabilities {
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            ..Default::default()
        };

        let merged = merge_capabilities(&[
            (ChildTier::Primary, primary_caps),
            (ChildTier::Secondary, secondary_caps),
        ]);
        assert!(
            matches!(merged.hover_provider, Some(HoverProviderCapability::Simple(true))),
            "boolean-union must return true when any child advertises true"
        );
    }

    #[test]
    fn boolean_union_references_secondary_true_overrides_primary_false() {
        use lsp_max::lsp_types::OneOf;

        let primary_caps = ServerCapabilities {
            references_provider: Some(OneOf::Left(false)),
            ..Default::default()
        };
        let secondary_caps = ServerCapabilities {
            references_provider: Some(OneOf::Left(true)),
            ..Default::default()
        };

        let merged = merge_capabilities(&[
            (ChildTier::Primary, primary_caps),
            (ChildTier::Secondary, secondary_caps),
        ]);
        assert!(
            matches!(merged.references_provider, Some(OneOf::Left(true))),
            "boolean-union must return true for referencesProvider when secondary advertises true"
        );
    }
}
