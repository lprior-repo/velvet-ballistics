#[path = "../../../boundary_inventory/src/kani_json_helper/mod.rs"]
mod kani_json_helper;

fn main() {
    std::process::exit(kani_json_helper::run());
}
