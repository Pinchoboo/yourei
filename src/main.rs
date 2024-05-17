use ansi_term::{Color::Green, Style};
use clap::Parser;
use regex::Regex;
use std::mem;
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
    let mut buf = String::new();
    let url = format!(
        "https://yourei.jp/{}?n={}&start={}",
        cli.word,
        cli.number.unwrap_or(1),
        cli.offset.unwrap_or(0) + 1
    );
    //let _ = File::open("./out.html").unwrap().read_to_string(&mut buf);
    reqwest::blocking::get(url)
        .unwrap()
        .read_to_string(&mut buf)
        .unwrap();
    let mut examples = extract_examples(&buf);

    let underline = Style::new().underline();
    let noline = "\x1b[24m".to_string();

    let rt_tag = Regex::new(r"<rt>(?<reading>.*?)<\/rt>").unwrap();
    let mut s = cli.word.chars().fold(r"(?<word>".to_string(), |acc, c| {
        format!(
            r"{acc}{c}({})?",
            (underline.prefix().to_string() + ".*?" + &noline).replace('[', "\\[")
        )
    }) + ")";
    let word_finder = Regex::new(&s).unwrap();
    s = Regex::new(r"(\p{Script=Katakana}|\p{Script=Hiragana})")
        .unwrap()
        .replace_all(&cli.word, "")
        .chars()
        .fold("(?<word>".to_string(), |acc, c| {
            format!(
                r"{acc}{c}({})?({})?",
                (r"(\p{Script=Katakana}|\p{Script=Hiragana})*?"),
                (underline.prefix().to_string() + ".*?" + &noline).replace('[', "\\[")
            )
        })
        + ")";
    let kanji_finder = Regex::new(&s).unwrap();

    for e in &mut examples {
        e.map(|s| s.replace("<ruby>", "").replace("</ruby>", ""));
        if cli.furigana {
            e.map(|s| {
                rt_tag
                    .replace_all(&s, underline.prefix().to_string() + "$reading" + &noline)
                    .to_string()
            });
        } else {
            e.map(|s| rt_tag.replace_all(&s, "").to_string());
        }
        if cli.emphasize {
            e.map(|s| {
                if word_finder.is_match(&s) {
                    word_finder
                        .replace_all(&s, Green.paint("$word").to_string())
                        .to_string()
                } else {
                    kanji_finder
                        .replace_all(&s, Green.paint("$word").to_string())
                        .to_string()
                }
            });
        }
    }

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
        let text = self.prev.to_owned().unwrap_or_default()
            + &self.sentence.to_owned().unwrap_or_default()
            + &self.next.to_owned().unwrap_or_default();
        if let Some(source) = &self.source {
            write!(f, "{}\n{}", text, source)
        } else {
            write!(f, "{}", text)
        }
    }
}

fn extract_examples(html_content: &str) -> Vec<Example> {
    let document = scraper::Html::parse_document(html_content);
    document
        .select(&scraper::Selector::parse("ul.sentence-list > [id^=\"sentence-\"]").unwrap())
        .filter_map(|example| {
            let sentence = example
                .select(&scraper::Selector::parse(".the-sentence").unwrap())
                .next()
                .map(|a| a.inner_html());
            sentence.as_ref()?;
            Some(Example {
                prev: example
                    .select(&scraper::Selector::parse(".prev-sentence").unwrap())
                    .next()
                    .map(|a| a.inner_html()),
                sentence,
                next: example
                    .select(&scraper::Selector::parse(".next-sentence").unwrap())
                    .next()
                    .map(|a| a.inner_html()),
                source: example
                    .select(&scraper::Selector::parse(".sentence-source-title").unwrap())
                    .next()
                    .map(|a| a.text().next().unwrap().to_owned()),
            })
        })
        .collect()
}
