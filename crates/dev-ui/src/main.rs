//! Placeholder developer UI. Step 07 replaces this with a real developer UI.
//!
//! For now it only prints the engine's name and version, which shows that the UI crate
//! builds against the engine's public interface.

use policyspace_engine::{ENGINE_NAME, ENGINE_VERSION};

fn main() {
    println!("{ENGINE_NAME} {ENGINE_VERSION}");
}
