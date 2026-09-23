use spacetime_bindings_macro_input_example_macros::test;
use spacetimedb::{procedure, reducer};

#[test]
#[spacetimedb::table(accessor = entity, public)]
pub struct Entity {
    /// The unique ID of the Entity.
    #[primary_key]
    #[auto_inc]
    id: u128,

    created_at: spacetimedb::Timestamp,
}

// This reducer is required, otherwise the SpacetimeDB module wouldn't compile. ("Error: unable to determine ABI of module (may be on an spacetime version < 0.8)")
#[reducer]
pub fn test(_: &spacetimedb::ReducerContext) -> Result<(), Box<str>> {
    Ok(())
}
