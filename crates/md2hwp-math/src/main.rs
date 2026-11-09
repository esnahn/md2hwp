// SPDX-License-Identifier: AGPL-3.0-only
//! Development CLI for isolated equation conversion and corpus inspection.
use md2hwp_math::{hml_to_tex, tex_to_hml, tex_to_hml_with_display};
use serde_json::{Value, json};
use std::{
    env, fs,
    io::{self, Read},
    path::Path,
    process::ExitCode,
    time::Instant,
};

fn convert(direction: &str, input: &str) -> Result<String, String> {
    match direction {
        "tex-to-hml" => tex_to_hml(input).map_err(|error| error.to_string()),
        "hml-to-tex" => hml_to_tex(input).map_err(|error| error.to_string()),
        _ => Err(format!("Unknown conversion direction: {direction}")),
    }
}

fn batch(direction: &str, input_path: &str, output_path: &str) -> Result<(), String> {
    if !["tex-to-hml", "hml-to-tex"].contains(&direction) {
        return Err(format!("Unknown conversion direction: {direction}"));
    }
    let canonical_input = fs::canonicalize(input_path).map_err(|error| error.to_string())?;
    if Path::new(input_path) == Path::new(output_path)
        || fs::canonicalize(output_path).is_ok_and(|path| path == canonical_input)
    {
        return Err("Input and output paths must differ".into());
    }
    let data: Value = serde_json::from_slice(&fs::read(input_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let source = data
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("Expected cases array")?;
    let mut cases = Vec::with_capacity(source.len());
    let start = Instant::now();
    let mut converted = 0;
    for (index, record) in source.iter().enumerate() {
        let key = if direction == "tex-to-hml" {
            "tex"
        } else {
            "hancom_script"
        };
        let input = record
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Missing {key} in case {}", index + 1))?;
        let display = record
            .get("options")
            .and_then(|v| v.get("displayMode"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let result = if direction == "tex-to-hml" {
            tex_to_hml_with_display(input, display).map_err(|error| error.to_string())
        } else {
            convert(direction, input)
        };
        converted += usize::from(result.is_ok());
        let (output, error) = match result {
            Ok(value) => (Some(value), None),
            Err(error) => (None, Some(error)),
        };
        cases.push(json!({"case_id": format!("{:05}", index + 1), "source_index":index+1,
                          "source_record":record, "input":input, "output":output,
                          "status":if error.is_none() {"converted"} else {"unsupported"}, "error":error}));
    }
    let result = json!({"direction":direction,"source":input_path,"count":source.len(),
                       "converted":converted,"unsupported":source.len()-converted,
                       "elapsed_ms":start.elapsed().as_millis(),"cases":cases});
    let output = Path::new(output_path);
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{} / {} converted; {}",
        converted,
        source.len(),
        output.display()
    );
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" {
        println!(
            "usage: md2hwp-math <tex-to-hml|hml-to-tex> [EXPRESSION]\n       md2hwp-math batch <DIRECTION> <source.json> <output.json>\n\nOmit EXPRESSION to read stdin. hml means native equation SCRIPT, not a whole XML document."
        );
        return Ok(());
    }
    if args[0] == "batch" {
        if args.len() != 4 {
            return Err("batch requires direction, source JSON and output JSON".into());
        }
        return batch(&args[1], &args[2], &args[3]);
    }
    let input = match args.len() {
        1 => {
            let mut text = String::new();
            io::stdin()
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            text
        }
        2 => args[1].clone(),
        _ => return Err("Provide one quoted expression, or pipe it on stdin".into()),
    };
    println!("{}", convert(&args[0], &input)?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("md2hwp-math: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_uses_each_record_display_mode() {
        let directory = env::temp_dir().join(format!(
            "md2hwp-math-display-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("input.json");
        let output = directory.join("output.json");
        fs::write(&source, br#"{"cases":[{"tex":"\\sum_i","options":{"displayMode":true}},{"tex":"\\sum_i","options":{"displayMode":false}},{"tex":"\\sum_i"}]}"#).unwrap();
        batch(
            "tex-to-hml",
            source.to_str().unwrap(),
            output.to_str().unwrap(),
        )
        .unwrap();
        let result: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(result["cases"][0]["output"], "sum _{i}");
        assert_eq!(result["cases"][1]["output"], "{sum} _{i}");
        assert_eq!(result["cases"][2]["output"], "{sum} _{i}");
        fs::remove_file(source).unwrap();
        fs::remove_file(output).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn batch_preserves_input_even_when_output_uses_a_path_alias() {
        let directory = env::temp_dir().join(format!(
            "md2hwp-math-source-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.json");
        let bytes = br#"{"cases":[]}"#;
        fs::write(&source, bytes).unwrap();
        let alias = directory.join(".").join("source.json");
        let result = batch(
            "tex-to-hml",
            source.to_str().unwrap(),
            alias.to_str().unwrap(),
        );
        assert!(result.unwrap_err().contains("must differ"));
        assert_eq!(fs::read(&source).unwrap(), bytes);
        fs::remove_file(source).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
