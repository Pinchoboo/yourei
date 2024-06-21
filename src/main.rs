use clap::Parser;
use regex::Regex;
use scraper::{Html, Selector};
use std::{fmt::Display, io::Read, mem};

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

    let mut examples = extract_examples(&page_content);
    format_examples(&cli, &mut examples);

    for e in examples {
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

impl Example {
    fn map<F: Fn(String) -> String>(&mut self, f: F) {
        self.prev = mem::take(&mut self.prev).map(&f);
        self.sentence = mem::take(&mut self.sentence).map(&f);
        self.next = mem::take(&mut self.next).map(&f);
        self.source = mem::take(&mut self.source).map(&f);
    }
}

impl Display for Example {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}\n{}",
            &self.prev.as_deref().unwrap_or_default(),
            &self.sentence.as_deref().unwrap_or_default(),
            &self.next.as_deref().unwrap_or_default(),
            &self.source.as_deref().unwrap_or_default(),
        )
    }
}

fn extract_examples(html_content: &str) -> Vec<Example> {
    Html::parse_document(html_content)
        .select(&Selector::parse("ul.sentence-list > [id^=\"sentence-\"]").unwrap())
        .filter_map(|example| {
            let inner_html = |query| {
                example
                    .select(&Selector::parse(query).unwrap())
                    .next()
                    .map(|a| a.inner_html())
            };
            Some(Example {
                prev: inner_html(".prev-sentence"),
                sentence: inner_html(".the-sentence"),
                next: inner_html(".next-sentence"),
                source: inner_html(".sentence-source-title > *"),
            })
        })
        .collect()
}

fn format_examples(cli: &Cli, examples: &mut [Example]) {
    let underline = "\x1b[4m";
    let noline = "\x1b[24m";
    let green = "\x1b[32m";
    let reset = "\x1b[0m";

    let any_underlined = format!("{underline}.*?{noline}").replace('[', "\\[");

    let rt_tag = Regex::new(r"<rt>(?<reading>.*?)<\/rt>").unwrap();
    let mut word_pattern = cli.word.chars().fold(r"(?<word>".to_string(), |acc, c| {
        format!(r"{acc}{c}({any_underlined})?",)
    }) + ")";
    let word_finder = Regex::new(&word_pattern).unwrap();
    word_pattern = Regex::new(r"(\p{Script=Katakana}|\p{Script=Hiragana})")
        .unwrap()
        .replace_all(&cli.word, "")
        .chars()
        .fold("(?<word>".to_string(), |acc, c| {
            format!(
                r"{acc}{c}({})?({any_underlined})?",
                (r"(\p{Script=Katakana}|\p{Script=Hiragana})*?")
            )
        })
        + ")";
    let kanji_only_finder = Regex::new(&word_pattern).unwrap();

    for e in examples {
        e.map(|s| s.replace("<ruby>", "").replace("</ruby>", ""));
        if cli.furigana {
            e.map(|s| {
                rt_tag
                    .replace_all(&s, format!("{underline}$reading{noline}"))
                    .to_string()
            });
        } else {
            e.map(|s| rt_tag.replace_all(&s, "").to_string());
        }
        if cli.emphasize {
            e.map(|s| {
                if word_finder.is_match(&s) {
                    word_finder
                        .replace_all(&s, format!("{green}$word{reset}"))
                        .to_string()
                } else {
                    kanji_only_finder
                        .replace_all(&s, format!("{green}$word{reset}"))
                        .to_string()
                }
            });
        }
    }
}
