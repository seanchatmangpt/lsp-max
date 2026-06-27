use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("show")]
#[allow(unused_variables)]
pub fn cmd_show(latest: bool) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}

pub fn register_explicit() {
    use clap_noun_verb::cli::registry::{CommandRegistry, ArgMetadata};
    use clap_noun_verb::logic::HandlerOutput;

    CommandRegistry::register_noun("receipt", "Manage and inspect receipts");
    CommandRegistry::register_verb_with_args(
        "receipt",
        "show",
        "Show receipt details",
        vec![ArgMetadata {
            name: "latest".to_string(),
            required: false,
            is_flag: true,
            help: Some("Show the latest receipt".to_string()),
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            short: None,
            default_value: None,
            env: None,
            multiple: false,
            value_name: None,
            aliases: vec![],
            positional: None,
            action: Some(clap::ArgAction::SetTrue),
            group: None,
            requires: vec![],
            conflicts_with: vec![],
            value_parser: None,
            hide: false,
            next_help_heading: None,
            long_help: None,
            next_line_help: false,
            display_order: None,
            exclusive: None,
            trailing_vararg: false,
            allow_negative_numbers: false,
            value_hint: None,
            global: false,
        }],
        |input| {
            let latest = input.args.get("latest")
                .or_else(|| input.opts.get("latest"))
                .map(|v| v.parse::<bool>().unwrap_or(false))
                .unwrap_or(false);
            let result = cmd_show(latest)?;
            HandlerOutput::from_data(result)
        }
    );
}
