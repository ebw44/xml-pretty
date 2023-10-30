use std::{
    fs::{self, write, File},
    path::{Path, PathBuf},
};

use anyhow::Context;
use gumdrop::Options;
use xmlem::{display, Document};

// #[cfg(windows)]
// const LINE_ENDING: &'static str = "\r\n";
// #[cfg(not(windows))]
// const LINE_ENDING: &'static str = "\n";

#[derive(Debug, Options)]
struct Args {
    #[options(help = "display help information")]
    help: bool,

    #[options(free, help = "path to XML document or folder containing XML documents")]
    xml_document_path: Option<PathBuf>,

    #[options(help = "output to file")]
    output_path: Option<PathBuf>,

    #[options(short = "r", long = "replace", help = "replace input file with output")]
    is_replace: bool,

    #[options(help = "number of spaces to indent (default: 2)")]
    indent: Option<usize>,

    #[options(
        short = "e",
        help = "number of spaces to pad the end of an element without separate end-tag (default: 1)"
    )]
    end_pad: Option<usize>,

    #[options(short = "l", help = "max line length (default: 120)")]
    max_line_length: Option<usize>,

    #[options(
        short = "H",
        long = "hex-entities",
        help = "Use hex entity encoding (e.g. &#xNNNN;) for all entities"
    )]
    uses_hex_entities: bool,

    #[options(
        no_short,
        long = "no-text-indent",
        help = "Do not prettify and indent text nodes"
    )]
    is_no_text_indent: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse_args_default_or_exit();

    let input_path = if let Some(path) = args.xml_document_path {
        Some(path)
    } else if atty::is(atty::Stream::Stdin) {
        eprintln!("ERROR: No XML document provided.");
        eprintln!("Run with -h for usage information.");
        return Ok(());
    } else {
        None
    };

    let input_list = match find_xml_files(&input_path) {
        Ok(xml_files) => xml_files,
        Err(err) => {
            eprintln!("Error: {}", err);
            Vec::new() // Return an empty Vec in case of an error
        }
    };

    for file_path in input_list {
        let text = prettify_file(
            &file_path,
            args.indent,
            args.end_pad,
            args.max_line_length,
            args.uses_hex_entities,
            !args.is_no_text_indent,
        )
        .with_context(|| format!("Failed to prettify '{}'", file_path.display()))?;

        let output_path = if args.is_replace {
            Some(file_path.clone())
        } else {
            args.output_path.clone()
        };

        let text_with_crlf = text.replace("\n", "\r\n");

        if let Some(path) = output_path {
            write(&path, text_with_crlf)
                .with_context(|| format!("Failed to write to '{}'", path.display()))?;
        } else {
            println!("{}", text_with_crlf);
        }
    }

    Ok(())
}

fn prettify_file(
    path: &Path,
    indent: Option<usize>,
    end_pad: Option<usize>,
    max_line_length: Option<usize>,
    uses_hex_entities: bool,
    indent_text_nodes: bool,
) -> anyhow::Result<String> {
    let file = File::open(path)?;
    let doc = Document::from_file(file)?;
    Ok(prettify(
        doc,
        indent,
        end_pad,
        max_line_length,
        uses_hex_entities,
        indent_text_nodes,
    ))
}

fn prettify(
    doc: Document,
    indent: Option<usize>,
    end_pad: Option<usize>,
    max_line_length: Option<usize>,
    uses_hex_entities: bool,
    indent_text_nodes: bool,
) -> String {
    doc.to_string_pretty_with_config(&display::Config {
        is_pretty: true,
        indent: indent.unwrap_or(2),
        end_pad: end_pad.unwrap_or(1),
        max_line_length: max_line_length.unwrap_or(120),
        entity_mode: if uses_hex_entities {
            display::EntityMode::Hex
        } else {
            display::EntityMode::Standard
        },
        indent_text_nodes,
    })
}

fn find_xml_files(input_path: &Option<PathBuf>) -> Result<Vec<PathBuf>, std::io::Error> {
    fn find_xml_files_recursive(directory: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut xml_files = Vec::new();

        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension() == Some("xml".as_ref()) {
                xml_files.push(path);
            } else if path.is_dir() {
                xml_files.extend(find_xml_files_recursive(&path)?);
            }
        }

        Ok(xml_files)
    }

    if let Some(path) = input_path {
        if path.is_dir() {
            find_xml_files_recursive(&path)
        } else if path.is_file() && path.extension() == Some("xml".as_ref()) {
            Ok(vec![path.clone()])
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid input path",
            ))
        }
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No input path provided",
        ))
    }
}
