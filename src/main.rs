use clap::{Parser, ValueEnum};
use regex::Regex;
use reqwest::Client;
use scraper::{ElementRef, Html, Selector};

use std::fmt::Write;

#[derive(Debug, Copy, Clone, ValueEnum, Default)]
enum OutputFormat {
    Json,
    Xml,
    #[default]
    Csv,
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct CommandLine {
    #[arg(
        short,
        long,
        required = true,
        default_value = "csv",
        default_missing_value = "csv"
    )]
    /// Set the output format
    format: OutputFormat,
}

#[derive(Debug)]
enum Component {
    Name(String),
    Rgb(u8, u8, u8),
}

#[derive(Debug)]
struct Color {
    name: String,
    red: u8,
    green: u8,
    blue: u8,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let nodes = load_colors().await?;
    let args = CommandLine::parse();

    let data = match args.format {
        OutputFormat::Json => generate_json(nodes)?,
        OutputFormat::Xml => generate_xml(nodes)?,
        OutputFormat::Csv => generate_csv(nodes)?,
    };

    println!("{}", data);

    Ok(())
}

fn generate_json(nodes: Vec<Color>) -> anyhow::Result<String> {
    let mut buf = String::new();

    writeln!(buf, "[")?;

    for (index, color) in nodes.iter().enumerate() {
        write!(
            buf,
            r#"  {{"name":"{}","red":{},"green":{},"blue":{}}}"#,
            color.name, color.red, color.green, color.blue
        )?;

        if index < nodes.len() - 1 {
            writeln!(buf, ",")?;
        }
    }

    writeln!(buf)?;
    writeln!(buf, "]")?;

    Ok(buf)
}

fn generate_csv(nodes: Vec<Color>) -> anyhow::Result<String> {
    let mut buf = String::new();

    writeln!(buf, "name,red,green,blue")?;

    for color in nodes {
        writeln!(
            buf,
            "{},{},{},{}",
            color.name, color.red, color.green, color.blue
        )?;
    }

    Ok(buf)
}

fn generate_xml(nodes: Vec<Color>) -> anyhow::Result<String> {
    let mut buf = String::new();

    writeln!(buf, r#""<?xml version="1.0" encoding="UTF-8"?>""#)?;
    writeln!(buf, "<colors>")?;

    for color in nodes {
        writeln!(
            buf,
            r#"  <color name="{}" red="{}" green="{}" blue="{}" />"#,
            color.name, color.red, color.green, color.blue
        )?;
    }

    writeln!(buf, "</colors>")?;

    Ok(buf)
}

async fn load_colors() -> anyhow::Result<Vec<Color>> {
    const URL: &str = "https://en.wikipedia.org/wiki/List_of_colors_(alphabetical)";

    let client = Client::new();
    let html = client.get(URL).send().await?.text().await?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("div.mw-content-ltr > div > p").unwrap();
    let color_data_regex =
        Regex::new(r"𝗥𝗚𝗕\s+\((?<red>\d+)\s+(?<green>\d+)\s+(?<blue>\d+)\)").unwrap();
    let color_extractor = move |e: ElementRef| {
        let text = e.text().collect::<String>();
        let text = text.trim();

        if text.is_empty() {
            let values = e.attr("title").unwrap();
            let caps = color_data_regex.captures(values).unwrap();
            let red = caps["red"].parse::<u8>().unwrap();
            let green = caps["green"].parse::<u8>().unwrap();
            let blue = caps["blue"].parse::<u8>().unwrap();

            Component::Rgb(red, green, blue)
        } else {
            let text = text
                .replace(move |c: char| c == '(' || c == ')', "")
                .replace(move |c: char| !c.is_ascii_alphanumeric(), "_")
                .to_lowercase()
                .to_string();

            Component::Name(text)
        }
    };

    let nodes = document
        .select(&selector)
        .map(color_extractor)
        .collect::<Vec<_>>();
    let nodes = nodes
        .chunks(2)
        .map(move |pair| {
            let mut color = Color {
                name: String::new(),
                red: 0,
                green: 0,
                blue: 0,
            };

            for component in pair.iter() {
                match component {
                    Component::Name(name) => color.name.clone_from(name),
                    Component::Rgb(red, green, blue) => {
                        color.red = *red;
                        color.green = *green;
                        color.blue = *blue;
                    }
                }
            }

            color
        })
        .collect();

    Ok(nodes)
}
