#![allow(clippy::unwrap_used)]
#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]

use bon::builder;
use color_eyre::{Report, Result};
use rss::{
    extension::{
        itunes::{ITunesCategory, ITunesChannelExtension, ITunesItemExtension, ITunesOwner},
        Extension, ExtensionBuilder, ExtensionMap,
    },
    Channel, ChannelBuilder, Enclosure, Guid, Image, Item, ItemBuilder,
};
use serde::{Deserialize, Serialize};
use xml::{reader::ParserConfig, writer::EmitterConfig};

use std::collections::{BTreeMap, HashMap, HashSet};

#[builder]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Podcast {
    title: String,
    description: String,
    subtitle: String,
    author: String,
    author_email: String,
    website: String,
    language: String,
    copyright: String,
    webmaster: String,
    managing_editor: String,
    formats: Vec<String>,
    hosting_base_url: String,

    keywords: Vec<String>,
    explicit: bool,

    // TODO: Do we even need separate Logo data?
    logo: Logo,
    category: String,

    episodes: Vec<Episode>,
}

#[builder]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Logo {
    url: String,
    title: String,
    link: String,
}

#[builder]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ItunesOwner {
    name: String,
    email: String,
}

#[builder]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ItunesCategory {
    // ?
    text: String,
    itunesu_category: String,
}

// TODO: Some of these are actually optional fields, we could be more permissive
// here with many fields, at least for itunes.
//
// See https://help.apple.com/itc/podcasts_connect/#/itcb54353390
#[builder]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Episode {
    title: String,
    url: String,
    description: String,
    subtitle: String,
    files: Vec<String>,
    duration: String,     // TODO: NaiveTime?
    publish_date: String, // TODO: NaiveDateTime/DateTime, kebab-case?
    keywords: Vec<String>,
    length_bytes: usize,
    transcript_url: Option<String>,
}

pub fn generate_xmls(pod: Podcast) -> Result<HashMap<String, Channel>, Report> {
    let mut cb = ChannelBuilder::default();

    let mut itunes = ITunesChannelExtension::default();

    let mut itunes_category = ITunesCategory::default();
    itunes_category.set_text(pod.category);

    let mut itunes_owner = ITunesOwner::default();
    itunes_owner.set_name(Some(pod.author.clone()));
    itunes_owner.set_email(Some(pod.author_email.clone()));

    itunes.set_author(pod.author.clone());
    itunes.set_categories(vec![itunes_category]);
    itunes.set_image(pod.logo.url.clone());
    itunes.set_explicit(Some(
        if pod.explicit { "true" } else { "false" }.to_string(),
    ));

    itunes.set_owner(itunes_owner);
    itunes.set_subtitle(pod.subtitle);
    itunes.set_summary(pod.description.clone());
    itunes.set_keywords(pod.keywords.join(", "));

    // itunes.set_complete();
    // itunes.set_new_feed_url();
    // itunes.set_block();

    let mut namespaces: BTreeMap<String, String> = BTreeMap::new();

    namespaces.insert("atom".into(), "http://www.w3.org/2005/Atom".into());
    namespaces.insert(
        "itunes".into(),
        "http://www.itunes.com/dtds/podcast-1.0.dtd".into(),
    );
    namespaces.insert(
        "podcast".into(),
        "https://podcastindex.org/namespace/1.0".into(),
    );

    let mut image = Image::default();
    image.set_url(pod.logo.url.clone());
    image.set_title(format!("{} Logo", &pod.title));
    image.set_link(pod.website.clone());

    // Generate everything EXCEPT the format
    let base_builder = cb
        .title(pod.title)
        .link(pod.website)
        .description(pod.description)
        .language(pod.language)
        .copyright(pod.copyright)
        .managing_editor(pod.managing_editor)
        .webmaster(pod.webmaster)
        .pub_date(Some("".into())) // TODO! - This should be RIGHT NOW
        .last_build_date(Some("".into())) // TODO! - This should be RIGHT NOW
        .generator(Some("pfg-rs".into())) // TODO!
        .image(image)
        .itunes_ext(itunes)
        .namespaces(namespaces);

    // Currently unused items
    //
    // .text_input(todo!())
    // .skip_hours(todo!())
    // .skip_days(todo!())
    // .categories(todo!())
    // .docs(todo!())
    // .cloud(todo!())
    // .rating(todo!())
    // .ttl(todo!())
    // .dublin_core_ext(todo!())
    // .syndication_ext(todo!())

    let mut map = HashMap::new();

    let mut item_map: HashMap<String, Vec<Item>> = HashMap::new();

    let base_set: HashSet<_> = pod.formats.clone().drain(..).collect();

    for episode in pod.episodes {
        let mut itunes_item = ITunesItemExtension::default();
        itunes_item.set_author(Some(pod.author.clone()));
        // itunes_item.set_block();
        itunes_item.set_image(Some(pod.logo.url.clone()));
        itunes_item.set_duration(Some(episode.duration)); // lol
        itunes_item.set_explicit(Some(
            if pod.explicit { "true" } else { "false" }.to_string(),
        ));
        itunes_item.set_summary(episode.description.clone());
        itunes_item.set_subtitle(episode.subtitle.clone());
        itunes_item.set_keywords(episode.keywords.join(", "));

        // Make "base" builder
        let mut base_item = ItemBuilder::default();

        base_item
            .title(episode.title.clone())
            .description(episode.description.clone())
            .author(pod.author_email.clone()) // email
            .pub_date(episode.publish_date.clone()) // RFC822
            .itunes_ext(itunes_item);

        let mut cur_set = base_set.clone();

        for file in episode.files {
            let ext = file.split('.').last().unwrap();
            let full_path = format!("{}/{}", pod.hosting_base_url, file);

            match (base_set.contains(ext), cur_set.contains(ext)) {
                (true, true) => {
                    let mut guid = Guid::default();
                    guid.set_value(full_path.clone());
                    guid.set_permalink(true);

                    let mut encl = Enclosure::default();
                    encl.set_url(full_path.clone());

                    let mime = match ext.to_lowercase().as_str() {
                        "mp3" => "audio/mpeg",
                        "m4a" => "audio/mp4",
                        "flac" => "audio/flac",
                        _ => "",
                    }
                    .to_string();

                    encl.set_mime_type(mime);
                    encl.set_length(episode.length_bytes.to_string());

                    let mut this_item = base_item.clone();
                    this_item.link(episode.url.clone());
                    this_item.enclosure(encl);

                    this_item.guid(Some(guid));
                    cur_set.remove(ext);

                    let mut item = this_item.build();

                    if let Some(transcript) = episode.transcript_url.clone() {
                        let xc_ext = transcript.split('.').last().unwrap();
                        let xcript_kind = match xc_ext {
                            "vtt" => "text/vtt",
                            "srt" => "application/srt",
                            "txt" => "text/plain",
                            _ => panic!("Unknown transcript extension?"),
                        }
                        .to_string();

                        // Build an extension...
                        let mut extension = ExtensionBuilder::default();
                        extension.name("podcast:transcript");
                        let mut attrs = BTreeMap::new();
                        attrs.insert("url".to_string(), transcript);
                        attrs.insert("type".to_string(), xcript_kind);
                        extension.attrs(attrs);

                        // I don't really understand what any of the following items do, they
                        // are required for setting the extension however. None of the names seem
                        // to actually matter?
                        let mut im = BTreeMap::<String, Vec<Extension>>::new();
                        im.insert("ext:name".to_string(), vec![extension.build()]);
                        let mut extension_map = ExtensionMap::default();
                        extension_map.insert("podcast".to_string(), im);

                        item.set_extensions(extension_map);
                    }

                    item_map.entry(ext.to_string()).or_default().push(item);
                }
                (true, false) => {
                    eprintln!("We've already added a file of format '{}' for episode '{}'. Skipping file '{}' with duplicate format.", ext, episode.title, file)
                }
                (false, _) => {
                    eprintln!("This podcast does not have '{}' in the listed 'formats'! Skipping '{}' in episode '{}'.", ext, file, episode.title);
                }
            }
        }
    }

    for (ext, items) in item_map.drain() {
        let mut this_builder = base_builder.clone();
        //println!("{:#?}", items);
        this_builder.items(items);
        map.insert(ext.to_string(), this_builder.build());
    }

    Ok(map)
}

// https://users.rust-lang.org/t/pretty-printing-xml/76372/3
pub fn format_xml(src: &[u8]) -> Result<String, xml::reader::Error> {
    let mut dest = Vec::new();
    let reader = ParserConfig::new()
        .trim_whitespace(true)
        .ignore_comments(false)
        .create_reader(src);
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .normalize_empty_elements(true)
        .autopad_comments(false)
        .create_writer(&mut dest);
    for event in reader {
        if let Some(event) = event?.as_writer_event() {
            writer.write(event).unwrap();
        }
    }
    Ok(String::from_utf8(dest).unwrap())
}
