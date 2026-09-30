//! Prints the OpenAPI spec. The frontend generates its API types from it:
//! `pnpm gen:api` in `frontend/`.

fn main() {
    let spec = coaching_backend::openapi()
        .to_pretty_json()
        .expect("the OpenAPI spec serializes to JSON");
    println!("{spec}");
}
