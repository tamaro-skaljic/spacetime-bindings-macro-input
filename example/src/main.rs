use spacetime_bindings_macro_input_example::test;

#[test]
#[table(name = entity, public)]
pub struct Entity {
    /// The unique ID of the Entity.
    #[primary_key]
    #[auto_inc]
    id: u128,

    created_at: Timestamp,
}

fn main() {

}
