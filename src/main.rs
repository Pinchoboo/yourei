use clap::Parser;
use regex::Regex;
use scraper::{Html, Selector};
use std::{fmt::Display, io::Read};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Word to search examples for
    word: String,
    /// Number of examples to return (default = 1)
    #[arg(short, long)]
    number: Option<u64>,
    /// Number of examples to skip from the example list before returning the next <NUMBER> of examples
    #[arg(short, long)]
    offset: Option<u64>,
    /// Show underlined furigana where furigana is used in the source text
    #[arg(short, long)]
    furigana: bool,
    /// Emphasize the searched word with a green color
    #[arg(short, long)]
    emphasize: bool,
}

fn main() {
    let cli = Cli::parse();
    let url = format!(
        "https://yourei.jp/{}?n={}&start={}",
        cli.word,
        cli.number.unwrap_or(1),
        cli.offset.unwrap_or(0) + 1
    );

    let mut page_content = String::new();
    reqwest::blocking::get(url)
        .unwrap()
        .read_to_string(&mut page_content)
        .unwrap();

    for e in extract_examples(&page_content, cli) {
        println!("{e}\n");
    }
}

#[derive(Default, Debug)]
struct Example {
    prev: Option<String>,
    sentence: Option<String>,
    next: Option<String>,
    source: Option<String>,
}

impl Display for Example {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            &self.prev.as_deref().unwrap_or_default(),
            &self.sentence.as_deref().unwrap_or_default(),
            &self.next.as_deref().unwrap_or_default()
        )?;
        if let Some(source) = &self.source {
            write!(f, "\n{source}")?;
        }
        Ok(())
    }
}

const UNDERLINE: &str = "\x1b[4m";
const NOLINE: &str = "\x1b[24m";
const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

fn extract_examples(html_content: &str, cli: Cli) -> Vec<Example> {
    let rt_regex = Regex::new(r"<rt>(?<reading>.*?)<\/rt>").unwrap();
    let word_regex = word_regex(&cli.word);
    let kanji_only_regex = kanji_only_regex(&cli.word);

    let reading_format = if cli.furigana {
        format!("{UNDERLINE}$reading{NOLINE}")
    } else {
        "".to_string()
    };
    let word_in_green = format!("{GREEN}$word{RESET}");

    let html_to_ansi = |mut s: String| {
        s = s.replace("<ruby>", "").replace("</ruby>", "");
        let s = rt_regex.replace_all(&s, &reading_format);
        if cli.emphasize {
            if word_regex.is_match(&s) {
                word_regex.replace_all(&s, &word_in_green)
            } else {
                kanji_only_regex.replace_all(&s, &word_in_green)
            }
        } else {
            s
        }
        .to_string()
    };

    Html::parse_document(html_content)
        .select(&Selector::parse("ul.sentence-list > [id^=\"sentence-\"]").unwrap())
        .map(|example| {
            let inner_html = |query| {
                example
                    .select(&Selector::parse(query).unwrap())
                    .next()
                    .map(|a| html_to_ansi(a.inner_html()))
            };
            Example {
                prev: inner_html(".prev-sentence"),
                sentence: inner_html(".the-sentence"),
                next: inner_html(".next-sentence"),
                source: inner_html(".sentence-source-title > *"),
            }
        })
        .collect()
}

fn word_regex(word: &str) -> Regex {
    Regex::new(
        &(word.chars().fold(r"(?<word>".to_string(), |acc, c| {
            format!(
                r"{acc}{c}({})?",
                format!("{UNDERLINE}.*?{NOLINE}").replace('[', "\\[")
            )
        }) + ")"),
    )
    .unwrap()
}

fn kanji_only_regex(word: &str) -> Regex {
    Regex::new(
        &(Regex::new(r"(\p{Script=Katakana}|\p{Script=Hiragana})")
            .unwrap()
            .replace_all(word, "")
            .chars()
            .fold("(?<word>".to_string(), |acc, c| {
                format!(
                    r"{acc}{c}({})?({})?",
                    r"(\p{Script=Katakana}|\p{Script=Hiragana})*?",
                    format!("{UNDERLINE}.*?{NOLINE}").replace('[', "\\[")
                )
            })
            + ")"),
    )
    .unwrap()
}
