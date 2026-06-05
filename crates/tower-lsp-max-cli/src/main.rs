#![allow(clippy::new_without_default)]
#![allow(clippy::io_other_error)]

pub mod nouns;

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}
