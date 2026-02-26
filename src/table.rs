use super::sats;
use super::sats::SatsField;
use super::sym;
use super::util::{check_duplicate, check_duplicate_msg, match_meta};
use core::slice;
use ident_case::RenameRule;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::parse::Parse;
use syn::parse::Parser as _;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Ident, Path, Token};

pub struct TableArgs {
    pub access: Option<TableAccess>,
    pub scheduled: Option<ScheduledArg>,
    pub accessor: Ident,
    pub indices: Vec<IndexArg>,
    pub event: Option<()>,
}

pub enum TableAccess {
    Public(Span),
    Private(Span),
}

pub struct ScheduledArg {
    pub span: Span,
    pub reducer_or_procedure: Path,
    pub at: Option<Ident>,
}

pub struct IndexArg {
    pub accessor: Ident,
    pub is_unique: bool,
    pub kind: IndexType,
}

impl IndexArg {
    fn new(accessor: Ident, kind: IndexType) -> Self {
        // We don't know if its unique yet.
        // We'll discover this once we have collected constraints.
        let is_unique = false;
        Self {
            accessor,
            is_unique,
            kind,
        }
    }
}

pub enum IndexType {
    BTree { columns: Vec<Ident> },
    Hash { columns: Vec<Ident> },
    Direct { column: Ident },
}

impl TableArgs {
    pub fn parse(input: TokenStream, item: &syn::DeriveInput) -> syn::Result<Self> {
        let mut access = None;
        let mut scheduled = None;
        let mut accessor = None;
        let mut indices = Vec::new();
        let mut event = None;

        syn::meta::parser(|meta| {
            match_meta!(match meta {
                sym::public => {
                    check_duplicate_msg(&access, &meta, "already specified access level")?;
                    access = Some(TableAccess::Public(meta.path.span()));
                }
                sym::private => {
                    check_duplicate_msg(&access, &meta, "already specified access level")?;
                    access = Some(TableAccess::Private(meta.path.span()));
                }
                sym::name => {}
                sym::accessor => {
                    check_duplicate(&accessor, &meta)?;
                    let value = meta.value()?;
                    accessor = Some(value.parse()?);
                }
                sym::index => indices.push(IndexArg::parse_meta(meta)?),
                sym::scheduled => {
                    check_duplicate(&scheduled, &meta)?;
                    scheduled = Some(ScheduledArg::parse_meta(meta)?);
                }
                sym::event => {
                    check_duplicate(&event, &meta)?;
                    event = Some(());
                }
            });
            Ok(())
        })
        .parse2(input)?;
        let accessor: Ident = accessor.ok_or_else(|| {
            let table = RenameRule::SnakeCase.apply_to_field(item.ident.to_string());
            syn::Error::new(
                Span::call_site(),
                format_args!(
                    "must specify table accessor, e.g. `#[spacetimedb::table(accessor = {table})]"
                ),
            )
        })?;

        Ok(TableArgs {
            access,
            scheduled,
            accessor,
            indices,
            event,
        })
    }
}

impl ScheduledArg {
    fn parse_meta(meta: ParseNestedMeta) -> syn::Result<Self> {
        let span = meta.path.span();
        let mut reducer_or_procedure = None;
        let mut at = None;

        meta.parse_nested_meta(|meta| {
            if meta.input.peek(syn::Token![=]) || meta.input.peek(syn::token::Paren) {
                match_meta!(match meta {
                    sym::at => {
                        check_duplicate(&at, &meta)?;
                        let ident = meta.value()?.parse()?;
                        at = Some(ident);
                    }
                })
            } else {
                check_duplicate_msg(
                    &reducer_or_procedure,
                    &meta,
                    "can only specify one scheduled reducer or procedure",
                )?;
                reducer_or_procedure = Some(meta.path);
            }
            Ok(())
        })?;

        let reducer_or_procedure = reducer_or_procedure.ok_or_else(|| {
            meta.error(
                "must specify scheduled reducer or procedure associated with the table: scheduled(function_name)",
            )
        })?;
        Ok(Self {
            span,
            reducer_or_procedure,
            at,
        })
    }
}

impl IndexArg {
    fn parse_meta(meta: ParseNestedMeta) -> syn::Result<Self> {
        let mut accessor = None;
        let mut algo = None;

        meta.parse_nested_meta(|meta| {
            match_meta!(match meta {
                sym::name => {}
                sym::accessor => {
                    check_duplicate(&accessor, &meta)?;
                    accessor = Some(meta.value()?.parse()?);
                }
                sym::btree => {
                    check_duplicate_msg(&algo, &meta, "index algorithm specified twice")?;
                    algo = Some(Self::parse_btree(meta)?);
                }
                sym::hash => {
                    check_duplicate_msg(&algo, &meta, "index algorithm specified twice")?;
                    algo = Some(Self::parse_hash(meta)?);
                }
                sym::direct => {
                    check_duplicate_msg(&algo, &meta, "index algorithm specified twice")?;
                    algo = Some(Self::parse_direct(meta)?);
                }
            });
            Ok(())
        })?;
        let accessor = accessor
            .ok_or_else(|| meta.error("missing index accessor, e.g. accessor = my_index"))?;
        let kind = algo.ok_or_else(|| {
            meta.error("missing index algorithm, e.g., `btree(columns = [col1, col2])`, `hash(columns = [col1, col2])` or `direct(column = col1)`")
        })?;

        Ok(IndexArg::new(accessor, kind))
    }

    fn parse_columns(meta: &ParseNestedMeta) -> syn::Result<Option<Vec<Ident>>> {
        let mut columns = None;
        meta.parse_nested_meta(|meta| {
            match_meta!(match meta {
                sym::columns => {
                    check_duplicate(&columns, &meta)?;
                    let value = meta.value()?;
                    let inner;
                    syn::bracketed!(inner in value);
                    columns = Some(
                        Punctuated::<Ident, Token![,]>::parse_terminated(&inner)?
                            .into_iter()
                            .collect::<Vec<_>>(),
                    );
                }
            });
            Ok(())
        })?;
        Ok(columns)
    }

    fn parse_btree(meta: ParseNestedMeta) -> syn::Result<IndexType> {
        let columns = Self::parse_columns(&meta)?;
        let columns = columns.ok_or_else(|| {
            meta.error("must specify columns for btree index, e.g. `btree(columns = [col1, col2])`")
        })?;
        Ok(IndexType::BTree { columns })
    }

    fn parse_hash(meta: ParseNestedMeta) -> syn::Result<IndexType> {
        let columns = Self::parse_columns(&meta)?;
        let columns = columns.ok_or_else(|| {
            meta.error("must specify columns for hash index, e.g. `hash(columns = [col1, col2])`")
        })?;
        Ok(IndexType::Hash { columns })
    }

    fn parse_direct(meta: ParseNestedMeta) -> syn::Result<IndexType> {
        let mut column = None;
        meta.parse_nested_meta(|meta| {
            match_meta!(match meta {
                sym::column => {
                    check_duplicate(&column, &meta)?;
                    let value = meta.value()?;
                    let inner;
                    syn::bracketed!(inner in value);
                    column = Some(Ident::parse(&inner)?);
                }
            });
            Ok(())
        })?;
        let column = column.ok_or_else(|| {
            meta.error("must specify the column for direct index, e.g. `direct(column = col1)`")
        })?;
        Ok(IndexType::Direct { column })
    }

    /// Parses an inline `#[index(btree)]`, `#[index(hash)]` or `#[index(direct)]` attribute on a field.
    fn parse_index_attr(field: &Ident, attr: &syn::Attribute) -> syn::Result<Self> {
        let mut kind = None;
        attr.parse_nested_meta(|meta| {
            match_meta!(match meta {
                sym::btree => {
                    check_duplicate_msg(&kind, &meta, "index type specified twice")?;
                    kind = Some(IndexType::BTree {
                        columns: vec![field.clone()],
                    });
                }
                sym::hash => {
                    check_duplicate_msg(&kind, &meta, "index type specified twice")?;
                    kind = Some(IndexType::Hash {
                        columns: vec![field.clone()],
                    });
                }
                sym::direct => {
                    check_duplicate_msg(&kind, &meta, "index type specified twice")?;
                    kind = Some(IndexType::Direct {
                        column: field.clone(),
                    })
                }
            });
            Ok(())
        })?;
        let kind = kind.ok_or_else(|| {
            syn::Error::new_spanned(
                &attr.meta,
                "must specify kind of index (`btree`, `hash` or `direct`)",
            )
        })?;
        let accessor = field.clone();
        Ok(IndexArg::new(accessor, kind))
    }
}

pub struct ColumnArgs<'a> {
    pub original_struct_name: Ident,
    pub fields: Vec<SatsField<'a>>,
    pub columns: Vec<Column<'a>>,
    pub unique_columns: Vec<Column<'a>>,
    pub sequenced_columns: Vec<Column<'a>>,
    pub primary_key_column: Option<Column<'a>>,
}

impl<'a> ColumnArgs<'a> {
    pub fn parse(
        mut table: TableArgs,
        item: &'a syn::DeriveInput,
    ) -> syn::Result<(TableArgs, Self)> {
        let sats_ty = sats::sats_type_from_derive(item, quote!(spacetimedb::spacetimedb_lib))?;

        let original_struct_name = sats_ty.ident.clone();

        let sats::SatsTypeData::Product(fields) = &sats_ty.data else {
            return Err(syn::Error::new(
                Span::call_site(),
                "spacetimedb table must be a struct",
            ));
        };

        if fields.len() > u16::MAX.into() {
            return Err(syn::Error::new_spanned(
                item,
                "too many columns; the most a table can have is 2^16",
            ));
        }

        let mut columns: Vec<Column<'a>> = vec![];
        let mut unique_columns: Vec<Column<'a>> = vec![];
        let mut sequenced_columns: Vec<Column<'a>> = vec![];
        let mut primary_key_column: Option<Column<'a>> = None;

        for (i, field) in fields.iter().enumerate() {
            let col_num = i as u16;
            let field_ident = field.ident.unwrap();

            let mut unique = None;
            let mut auto_inc = None;
            let mut primary_key = None;
            let mut default_value = None;

            for attr in field.original_attrs {
                let Some(attr) = ColumnAttr::parse(attr, field_ident)? else {
                    continue;
                };
                match attr {
                    ColumnAttr::Unique(span) => {
                        check_duplicate(&unique, span)?;
                        unique = Some(span);
                    }
                    ColumnAttr::AutoInc(span) => {
                        check_duplicate(&auto_inc, span)?;
                        auto_inc = Some(span);
                    }
                    ColumnAttr::PrimaryKey(span) => {
                        check_duplicate(&primary_key, span)?;
                        primary_key = Some(span);
                    }
                    ColumnAttr::Index(index_arg) => table.indices.push(index_arg),
                    ColumnAttr::Default(expr, span) => {
                        check_duplicate(&default_value, span)?;
                        default_value = Some(expr);
                    }
                }
            }

            let column: Column<'a> = Column {
                index: col_num,
                ident: field_ident,
                vis: field.vis,
                ty: field.ty,
                default_value,
            };

            if unique.is_some() || primary_key.is_some() {
                unique_columns.push(column.clone());
            }
            if auto_inc.is_some() {
                sequenced_columns.push(column.clone());
            }
            if let Some(span) = primary_key {
                check_duplicate_msg(
                    &primary_key_column,
                    span,
                    "can only have one primary key per table",
                )?;
                primary_key_column = Some(column.clone());
            }

            columns.push(column.clone());
        }

        // Mark all indices with a single column matching a unique constraint as unique.
        // For all the unpaired unique columns, create a unique index.
        for unique_col in &unique_columns {
            if table.indices.iter_mut().any(|index| {
                let covered_by_index = match &index.kind {
                    IndexType::BTree { columns } => columns == slice::from_ref(unique_col.ident),
                    IndexType::Hash { columns } => columns == slice::from_ref(unique_col.ident),
                    IndexType::Direct { column } => column == unique_col.ident,
                };
                index.is_unique |= covered_by_index;
                covered_by_index
            }) {
                continue;
            }
            // NOTE(centril): We pick `btree` here if the user does not specify otherwise,
            // as it's the safest choice of index for the general case,
            // even if isn't optimal in specific cases.
            let accessor = unique_col.ident.clone();
            let columns = vec![accessor.clone()];
            table.indices.push(IndexArg {
                accessor,
                is_unique: true,
                kind: IndexType::BTree { columns },
            })
        }

        Ok((
            table,
            ColumnArgs {
                original_struct_name,
                fields: fields.to_vec(),
                columns,
                unique_columns,
                sequenced_columns,
                primary_key_column,
            },
        ))
    }
}

#[derive(Clone)]
pub struct Column<'a> {
    pub index: u16,
    pub vis: &'a syn::Visibility,
    pub ident: &'a syn::Ident,
    pub ty: &'a syn::Type,
    pub default_value: Option<syn::Expr>,
}

pub enum ColumnAttr {
    Unique(Span),
    AutoInc(Span),
    PrimaryKey(Span),
    Index(IndexArg),
    Default(syn::Expr, Span),
}

impl ColumnAttr {
    fn parse(attr: &syn::Attribute, field_ident: &Ident) -> syn::Result<Option<Self>> {
        let Some(ident) = attr.path().get_ident() else {
            return Ok(None);
        };
        Ok(if ident == sym::index {
            let index = IndexArg::parse_index_attr(field_ident, attr)?;
            Some(ColumnAttr::Index(index))
        } else if ident == sym::unique {
            attr.meta.require_path_only()?;
            Some(ColumnAttr::Unique(ident.span()))
        } else if ident == sym::auto_inc {
            attr.meta.require_path_only()?;
            Some(ColumnAttr::AutoInc(ident.span()))
        } else if ident == sym::primary_key {
            attr.meta.require_path_only()?;
            Some(ColumnAttr::PrimaryKey(ident.span()))
        } else if ident == sym::default {
            Some(ColumnAttr::Default(
                attr.parse_args::<syn::Expr>()?,
                ident.span(),
            ))
        } else {
            None
        })
    }
}
