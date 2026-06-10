/*
This file is part of lsp-max-ast.
Copyright (C) 2025 CLAUZEL Adrien

lsp-max-ast is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>
*/

//! # Auto LSP Core
//! Core crate for lsp_max_ast

pub mod ast;

/// Semantic tokens builder
pub mod semantic_tokens_builder;

/// Document symbols builder
pub mod document_symbols_builder;

/// Document handling
pub mod document;

pub mod errors;
pub mod parsers;
pub mod regex;
pub mod utils;
