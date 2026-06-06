use crate::jsonrpc::{Error, Result};
use lsp_types_max::{
    TypeHierarchyItem, TypeHierarchyPrepareParams, TypeHierarchySubtypesParams,
    TypeHierarchySupertypesParams,
};
use url::Url;

/// Prepares the type hierarchy for a symbol.
pub async fn prepare_type_hierarchy(
    params: TypeHierarchyPrepareParams,
) -> Result<Option<Vec<TypeHierarchyItem>>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(items) =
        tower_lsp_max_runtime::control_plane::views::lookup_type_hierarchy_prepare(views, &url, pos)
    {
        Ok(Some(items))
    } else {
        Ok(None)
    }
}

/// Resolves supertypes for a type hierarchy item.
pub async fn supertypes(
    params: TypeHierarchySupertypesParams,
) -> Result<Option<Vec<TypeHierarchyItem>>> {
    let uri = &params.item.uri;
    let pos = params.item.selection_range.start;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(items) =
        tower_lsp_max_runtime::control_plane::views::lookup_type_hierarchy_supertypes(
            views, &url, pos,
        )
    {
        Ok(Some(items))
    } else {
        Ok(None)
    }
}

/// Resolves subtypes for a type hierarchy item.
pub async fn subtypes(
    params: TypeHierarchySubtypesParams,
) -> Result<Option<Vec<TypeHierarchyItem>>> {
    let uri = &params.item.uri;
    let pos = params.item.selection_range.start;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(items) = tower_lsp_max_runtime::control_plane::views::lookup_type_hierarchy_subtypes(
        views, &url, pos,
    ) {
        Ok(Some(items))
    } else {
        Ok(None)
    }
}
