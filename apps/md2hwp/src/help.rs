// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Euisoon Ahn

const BASIC: &str = "md2hwp <source.md> [source.output.hwp] [--template <template.hwp>] [OPTIONS]";
const FULL: &str = "md2hwp <source.md> [OPTIONS]";
const WORKER: &str = "  --worker FILE       Backend executable (default: md2hwp-backend.exe).\n  --dotnet FILE       .NET host for the Hancom backend.";
const HELP: &str = "  -h, --help          Show this help.";
const FORMATS: &str = "Hancom output/template formats: .hwp or .hwpx";
const COMMANDS: &[(&str, &str, &str)] = &[
    (
        "md2ir",
        "<source.md> [source.ir.json] [OPTIONS]",
        "Convert Markdown or Pandoc JSON to validated IR.",
    ),
    (
        "ir2hwp",
        "--ir <source.ir.json> --output <source.output.hwp> [OPTIONS]",
        "Generate a document from validated IR.",
    ),
    (
        "validate",
        "<source.md> [OPTIONS]",
        "Check strict manuscript rules without creating output.",
    ),
    ("setup-pandoc", "", "Download and prepare Pandoc."),
    (
        "check-runtime",
        "[OPTIONS]",
        "Check the Hancom backend's .NET runtime.",
    ),
    (
        "validate-template",
        "[template.hwp] [OPTIONS]",
        "Validate template declarations and prototypes.",
    ),
    (
        "init-template",
        "[template.hwp] [OPTIONS]",
        "Create a template with Hancom default formatting.",
    ),
];

pub fn brief() -> String {
    format!("{BASIC}\nUse --help for options and commands.")
}

pub fn command_brief(name: &str) -> Option<String> {
    let (_, syntax, _) = COMMANDS.iter().find(|(key, _, _)| *key == name)?;
    Some(format!(
        "md2hwp {name} {syntax}\nUse md2hwp {name} --help for command options."
    ))
}

pub fn full() -> String {
    let mut text = format!(
        "{FULL}\n\nOptions:\n  [--output] FILE     Optional output path (default: source.output.hwp in the source folder).\n  --template FILE     Template (default: template.hwp).\n{WORKER}\n  --verbose           Show backend details.\n  --version           Show application and selected backend versions.\n{HELP}"
    );
    legacy(&mut text);
    text.push_str(&format!("\n\n{FORMATS}\n\nSubcommands:\n"));
    for (name, syntax, _) in COMMANDS {
        text.push_str(&format!("  md2hwp {name}"));
        if !syntax.is_empty() {
            text.push(' ');
            text.push_str(syntax);
        }
        text.push('\n');
    }
    text.push_str("\nUse md2hwp COMMAND --help for command options.");
    text.push('\n');
    text.push_str(super::LICENSE_NOTICE);
    text
}

pub fn command(name: &str) -> Option<String> {
    let (_, syntax, description) = COMMANDS.iter().find(|(key, _, _)| *key == name)?;
    let syntax = if name == "md2ir" {
        "<source.md> [OPTIONS]"
    } else {
        syntax
    };
    let mut text = format!("md2hwp {name}");
    if !syntax.is_empty() {
        text.push(' ');
        text.push_str(syntax);
    }
    text.push_str(&format!("\n{description}\n\nOptions:\n"));
    match name {
        "md2ir" => text.push_str(
            "  [--output] FILE     Optional output path (default: source.ir.json in the source folder).\n  --from FORMAT       commonmark (default) or pandoc-json.\n  --pandoc FILE       Pandoc executable for commonmark input.\n  --force             Replace an existing IR file.\n",
        ),
        "ir2hwp" => {
            text.push_str(
                "  --ir FILE           Input IR (required).\n  --output FILE       Output document (required).\n  --template FILE     Template (default: template.hwp).\n",
            );
            text.push_str(WORKER);
            text.push_str("\n  --verbose           Show backend details.\n");
        }
        "validate" => text.push_str(
            "  --input FILE        Markdown input; may also be positional.\n  --pandoc FILE       Pandoc executable.\n  --require-meta KEY  Require a nonempty metadata value; repeatable.\n",
        ),
        "setup-pandoc" => {}
        "check-runtime" => text.push_str("  --dotnet FILE       .NET host to check.\n"),
        "validate-template" => {
            text.push_str("  [--template] FILE   Template path.\n");
            text.push_str(WORKER);
            text.push('\n');
        }
        "init-template" => {
            text.push_str("  [--output] FILE     New template (default: template.hwp).\n");
            text.push_str(WORKER);
            text.push('\n');
        }
        _ => unreachable!(),
    }
    text.push_str(HELP);
    if name == "ir2hwp" {
        legacy(&mut text);
    }
    match name {
        "md2ir" => text
            .push_str("\n\nInput formats: Markdown (default) or Pandoc JSON (--from pandoc-json)."),
        "ir2hwp" => text.push_str(&format!("\n\n{FORMATS}")),
        "validate-template" => {
            text.push_str("\n\nDefault: template.hwp. Hancom also accepts .hwpx templates.")
        }
        "init-template" => text
            .push_str("\n\nExisting files are not overwritten. Hancom also accepts .hwpx output."),
        "check-runtime" => text.push_str("\n\nRequires a stable .NET 10 x64 runtime."),
        "setup-pandoc" => text.push_str(
            "\n\nInstalls into the managed dependency directory; no PATH edit is required.",
        ),
        _ => {}
    }
    Some(text)
}

fn legacy(text: &mut String) {
    #[cfg(debug_assertions)]
    text.push_str("\n  --legacy-com        Use the original COM insertion path for comparison.");
    #[cfg(not(debug_assertions))]
    let _ = text;
}
