/// Source: <https://github.com/jamesmunns/pfg-rs>
use std::{
    fs::{read_dir, File},
    io::Write,
    path::PathBuf,
};

use color_eyre::{Report, Result};
use hypertext::{html_elements, maud, rsx, Attribute, GlobalAttributes, Renderable, Rendered};
use pfg::{generate_xmls, Episode, Logo, Podcast};
use pulldown_cmark::{html::push_html, Parser};
use serde::{Deserialize, Serialize};

mod pfg;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Metadata {
    head: String,
    assignee: String,
    base: String,
    draft: bool,
    issue: Option<String>,
    reviewers: Vec<String>,
    tags: Vec<String>,
    title: Option<String>,
}

fn get_metadata(md_string: &str) -> Metadata {
    let split_md = md_string.split("---\n").collect::<Vec<&str>>();
    let yaml_string = split_md[1];
    serde_yaml::from_str(yaml_string).expect("valid frontmatter in markdown")
}

fn build_podcast_feed() -> Result<()> {
    let base_url = "https://decapsulate.com";
    let logo = Logo::builder()
        .url(format!("{base_url}/logo-large.jpg"))
        .title("Decapsulate Logo")
        .link(format!("{base_url}/logo-large.jpg"))
        .build();
    let episode = Episode::builder()
        .title("episode title")
        .url(format!("{base_url}/episode/1"))
        .description("episode description")
        .subtitle("episode subtitle")
        .files(vec!["audio/declaration-test.mp3".into()])
        .duration("episode duration")
        .publish_date("episode publish date (probably need chrono)")
        .keywords(vec!["technology".into()])
        .length_bytes(0)
        .transcript_url("transcript_url.txt")
        .build();

    let episodes = vec![episode];

    let podcast = Podcast::builder()
        .title("Decapsulate")
        .description("Unpacking things.")
        .subtitle("Unpacking things")
        .author("Namtao Productions")
        .author_email("contact@namtao.com")
        .website("https://decapsulate.com")
        .language("English")
        .copyright("Namtao Productions")
        .webmaster("web master / dj")
        .managing_editor("managing editor")
        .formats(vec!["mp3".into()])
        .hosting_base_url(base_url)
        .keywords(vec!["Non-fiction".into(), "technology".into()])
        .explicit(false)
        .logo(logo)
        .category("Technology")
        .episodes(episodes)
        .build();

    let xmls = generate_xmls(podcast)?;

    for (format, data) in &xmls {
        let filename = format!("docs/decapsulate-{format}.xml");
        println!("Writing {}", &filename);

        let mut file = File::create(&filename).expect("file system accessible");
        let unformatted = data.to_string();
        let formatted = pfg::format_xml(unformatted.as_bytes())
            .expect("machine built xml can be machine formatted");
        file.write_all(formatted.as_bytes())?;
    }

    Ok(())
}

fn get_files_in_folder(path: &str) -> Result<Vec<PathBuf>> {
    let entries = read_dir(path)?;
    let all: Vec<PathBuf> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .collect();
    Ok(all)
}

fn main() -> Result<(), Report> {
    let episodes = get_files_in_folder("episodes/").map(|e| e);

    build_podcast_feed()?;
    build(vec![
        ("docs/index.html", index().render()),
        //("docs/feed.rss", Rendered(feed())),
    ])?;
    println!("Built site OK!");
    Ok(())
}

fn index() -> impl Renderable {
    template(maud! {
        div ."sm:flex" ."s:flex-row" ."gap-20" {
            div."basis-1/3" {
                div .flex.w-full.justify-center {
                    img src="logo.png" alt="logo" {}
                }
            }
            div ."basis-2/3" {
                h2 .text-4xl { "Episodes" }
                ol .list-decimal {
                    li { a.underline href="" { "Pilot: Writing & Mental Health" } }
                    li {  a.underline href="" { "GPT & Enshittification" } }
                    li {  a.underline href="" { "Future episodes..." } }
                }
            }
        }
    })
}

#[allow(dead_code)]
struct Markdown<'a>(&'a str);

impl Renderable for Markdown<'_> {
    fn render_to(self, output: &mut String) {
        let mut output_html = String::new();
        let parser = Parser::new(self.0);
        push_html(&mut output_html, parser);
        output.push_str(output_html.as_str());
    }
}

fn template(inner: impl Renderable) -> impl Renderable {
    rsx! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta http-equiv="x-clacks-overhead" content="GNU Terry Pratchett" />
                <link rel="icon" href="favicon.png"/>
                <script src="tw.js"></script>

            <script>
                r#" tailwind.config = {
                    theme: {
                        container: {
                            center: true,
                        },
                        fontFamily: {
                            "mono": "courier, monospace",
                        }
                    }
                }"#
            </script>

            <meta charset="utf-8"/>
            <meta name="description" content="When you see this logo on any artwork, whether painting, poetry, or prose, you know that it was made by a human just like you."/>

            <meta content="width=device-width, initial-scale=1" name="viewport"/>
            <title class="text-4xl" >"Decapsulate Podcast"</title>

            </head>

                <body class="bg-neutral-900 text-white font-mono text-sm md:text-2xl mx-auto w-full">

                    <nav class="bg-neutral-800 flex items-center justify-between flex-wrap p-6">
                        <div class="flex items-center flex-shrink-0 text-white mr-6">
                            <span class="font-semibold text-xl tracking-tight">Decapsulate Podcast</span>
                        </div>
                        <div class="w-full block flex-grow lg:flex lg:items-center lg:w-auto">
                            <div class="text-xl lg:flex-grow">
                                <a href="index.html#about" class="underline block lg:inline-block lg:mt-0 text-black-200 hover:text-white mr-4">
                                    About
                                </a>
                                <a href="" class="underline block lg:inline-block lg:mt-0 text-black-200 hover:text-white mr-4">
                                    Listen
                                </a>
                                <a href="" class="underline block lg:inline-block lg:mt-0 text-black-200 hover:text-white mr-4">
                                    Credits
                                </a>
                                <a href="decapsulate-mp3.xml" class="underline block lg:inline-block lg:mt-0 text-black-200 hover:text-white mr-4">
                                   Podcast Feed
                                </a>
                            </div>
                        </div>
                    </nav>

                    <div class="border-neutral-900 border-8 container mx-auto">

                    <br/>
                    <br/>
                    <h2 class="slogan"><b class="text-2xl" > "" </b></h2>
                        {inner}
                    </div>
            { footer() }

            </body>
        </html>
    }
}

#[allow(unused)] // it's used inside `rsx!`
trait HtmxAttributes: GlobalAttributes {
    #[allow(non_upper_case_globals)]
    const xmlns: Attribute = Attribute;
    #[allow(non_upper_case_globals)]
    const property: Attribute = Attribute;
}
impl<T: GlobalAttributes> HtmxAttributes for T {}

fn footer() -> impl Renderable {
    rsx! {
        <br/>
        <br/>
        <br/>
        <br/>
        <p class="border-neutral-900"
           xmlns="http://creativecommons.org/ns#"
           xmlns="http://purl.org/dc/terms/">
            <a property="dct:title" rel="cc:attributionURL" href="https://decapsulate.com">
                Decapsulate Podcast
            </a>
            by
            <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://decapsulate.com">
                Namtao Productions
            </a>
            is licensed under
            <a href="https://creativecommons.org/licenses/by-nc/4.0/?ref=chooser-v1" rel="license noopener noreferrer" style="display:inline-block;">
                CC BY-NC 4.0
                <img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1" alt="">
                <img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1" alt="">
                <img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1" alt="">
            </a>
        </p>

        <p class="border-neutral-900 border-8 text-xs">"Decapsulate is a NAMTAO production, made with <3 in 2024"</p>
    }
}

fn build(pages: Vec<(&str, Rendered<String>)>) -> Result<(), Report> {
    std::fs::create_dir_all("docs")?;
    for (page, fun) in pages {
        println!("Writing {page}");
        let output = fun.into_inner();
        std::fs::write(page, output)?;
    }
    Ok(())
}
