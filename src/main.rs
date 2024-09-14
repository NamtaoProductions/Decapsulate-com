/// TODO This MVP needs HUGE refactoring.
/// - Split into modules (core, templates, pages, feed)
/// - Switch to a sass-based css framework, from Tailwind
use std::{
    fs::{read_dir, read_to_string, File},
    io::Write,
    path::PathBuf,
};

use color_eyre::eyre::{eyre, Report, Result, WrapErr};
use hypertext::{
    html_elements, maud, maud_move, rsx, Attribute, GlobalAttributes, Renderable, Rendered,
};
use pfg::{generate_xmls, Episode, Logo, Podcast};
use pulldown_cmark::{html::push_html, Parser};
use serde::{Deserialize, Serialize};

mod pfg;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct Metadata {
    episode: u32,
    title: String,
    date: String,
    description: String,
    keywords: Vec<String>,
    transcript: Option<String>,
}

impl Metadata {
    fn episode(&self, base_url: &str) -> Episode {
        let num = self.episode;
        let url = format!("{base_url}/episodes/{num}"); //TODO refactor this into url
                                                        //method for mp3 and txt
        Episode::builder()
            .title(self.title.clone())
            .url(url.clone())
            .description(self.description.clone())
            .subtitle("episode subtitle")
            .files(vec![format!("audio/DC{num}.mp3")])
            .duration("1") // TODO calculate this
            .publish_date(self.date.clone()) // TODO validate this
            .keywords(vec!["technology".into()])
            .length_bytes(0)
            .transcript_url(format!("{url}.txt")) // TODO
            .build()
    }

    fn url(&self) -> String {
        let num = self.episode;
        format!("/episode/{num}")
    }
}

fn build_podcast_feed(metadatas: &[Metadata]) -> Result<()> {
    let base_url = "https://decapsulate.com";
    let logo = Logo::builder()
        .url(format!("{base_url}/logo-large.jpg"))
        .title("Decapsulate Logo")
        .link(format!("{base_url}/logo-large.jpg"))
        .build();

    let episodes = metadatas.iter().map(|m| m.episode(base_url)).collect();

    let podcast = Podcast::builder()
        .title("Decapsulate")
        .description("Unpacking life.")
        .subtitle("Unpacking things")
        .author("Namtao Productions")
        .author_email("contact@namtao.com")
        .website("https://decapsulate.com")
        .language("English")
        .copyright("Namtao Productions")
        .webmaster("TODO WEBMASTER")
        .managing_editor("TODO MANAGING EDITOR")
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

        let mut file = File::create(&filename).expect("file system accessible");
        let unformatted = data.to_string();
        let formatted = pfg::format_xml(unformatted.as_bytes())
            .expect("machine built xml can be machine formatted");
        println!("Writing {}", &filename);
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

#[allow(clippy::manual_let_else)]
fn get_metadata(md_string: &str) -> Result<Metadata> {
    let frontmatter = md_string.split("---\n").collect::<Vec<&str>>();
    let yaml_string = frontmatter
        .get(1)
        .ok_or_else(|| eyre!("Invalid frontmatter: {}", md_string))?;
    //TODO: Have to wrap the error due to deserialisation trait bug.
    let yaml_metadata: Result<Metadata> =
        serde_yaml::from_str(yaml_string).wrap_err(format!("Bad YAML: \n{md_string}"));
    //TODO md to html
    let transcript = frontmatter.get(2).map(std::string::ToString::to_string);
    yaml_metadata.map(|mut m| {
        m.transcript = transcript;
        m
    })
}

fn main() -> Result<(), Report> {
    color_eyre::install()?;
    let episodes: Result<Vec<Metadata>> = get_files_in_folder("episodes/")?
        .into_iter()
        .map(read_to_string)
        .map(|yaml| get_metadata(&yaml?))
        .collect();
    let mut validated_episodes = episodes?;
    validated_episodes.sort_by_key(|m| m.episode);

    build_podcast_feed(&validated_episodes)?;
    build(
        validated_episodes.clone(),
        vec![(
            "docs/index.html",
            index(validated_episodes.clone()).render(),
        )],
    )?;
    println!("Built site OK!");
    Ok(())
}

fn build_episode(episode: Metadata) -> impl Renderable {
    let transcript_str = episode
        .transcript
        .clone()
        .expect("By this stage, the transcript has been attached");
    let transcript = Markdown(transcript_str);
    let num = episode.episode;

    template(maud_move! {
        h1 .text-4xl { ( episode.title ) }
        div .border-8 .border-transparent {
            @for keyword in episode.keywords {
                label .bg-indigo-500 .border-4 .border-neutral-900  { ( keyword ) }
            }
        }
        p .italic { ( episode.date ) }
        div .border-8 .border-transparent { audio controls src=( format!("/audio/DC{num}.mp3")) {} }
        br;
        div { ( episode.description ) }
        br;
        div { ( transcript ) }
    })
}

fn index(episodes: Vec<Metadata>) -> impl Renderable {
    template(maud! {
        div ."sm:flex" ."s:flex-row" ."gap-20" {
            div."basis-1/3" {
                div .flex.w-full.justify-center {
                    img src="logo.jpg" alt="logo" {}
                }
            }
            div ."basis-2/3" {
                h2 .text-4xl { "Episodes" }
                ol .list-decimal {
                    @for episode in episodes {
                        @let num = episode.episode;
                        li { a.underline href=(format!("/episodes/{num}/")) { (episode.title) } }
                    }
                }
            }
        }
    })
}

#[allow(dead_code)]
struct Markdown(String);

impl Renderable for Markdown {
    fn render_to(self, output: &mut String) {
        let mut output_html = String::new();
        let parser = Parser::new(&self.0);
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
                <link rel="icon" href="/favicon.png"/>
                <script src="/tw.js"></script>

            <script>
                r#" tailwind.config = {
                    theme: {
                        container: {
                            center: true,
                        },
                        fontFamily: {
                            mono: courier, monospace,
                        }
                    }
                }"#
            </script>

            <meta charset="utf-8"/>
            <meta name="description" content="Unpacking life with Tris & Robin."/>

            <meta content="width=device-width, initial-scale=1" name="viewport"/>
            <title class="text-4xl" >"Decapsulate Podcast"</title>

            </head>

                <body class="bg-neutral-900 text-white font-mono text-sm md:text-2xl mx-auto w-full">

                    <nav class="bg-neutral-800 flex items-center justify-between flex-wrap p-6">
                        <div class="flex items-center flex-shrink-0 text-white mr-6">
                            <span class="font-semibold text-xl tracking-tight"><a href="/">Decapsulate Podcast</a></span>
                        </div>
                        <div class="w-full block flex-grow lg:flex lg:items-center lg:w-auto">
                            <div class="text-xl lg:flex-grow">
                                // <a href="index.html#about" class="underline block lg:inline-block lg:mt-0 text-black-200 hover:text-white mr-4">
                                //     About
                                // </a>
                                // <a href="" class="underline block lg:inline-block lg:mt-0 text-black-200 hover:text-white mr-4">
                                //     Listen
                                // </a>
                                // <a href="" class="underline block lg:inline-block lg:mt-0 text-black-200 hover:text-white mr-4">
                                //     Credits
                                // </a>
                                <a href="/decapsulate-mp3.xml" class="underline block lg:inline-block lg:mt-0 text-black-200 hover:text-white mr-4">
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

        <p class="border-neutral-900 border-8 text-xs">"Decapsulate is a NAMTAO production, licensed under CC BY-NC 4.0, made with <3 in 2024"</p>
    }
}

fn build(episodes: Vec<Metadata>, pages: Vec<(&str, Rendered<String>)>) -> Result<(), Report> {
    std::fs::create_dir_all("docs")?;
    for (page, fun) in pages {
        println!("Writing {page}");
        let output = fun.into_inner();
        std::fs::write(page, output)?;
    }
    for episode in episodes {
        let num = episode.episode;
        let folder = format!("docs/episodes/{num}/");
        let path = format!("{folder}index.html");
        std::fs::create_dir_all(&folder)?;
        println!("Writing {path}");
        let output = build_episode(episode.clone());
        std::fs::write(path, output.render().into_inner())?;
    }
    Ok(())
}
