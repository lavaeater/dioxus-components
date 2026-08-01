use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::naming::IconNames;

#[derive(Debug, Clone)]
pub struct Icon {
    pub source_name: String,
    pub component: String,
    pub module: String,
    pub module_ref: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub view_box: String,
    pub elements: Vec<SvgElement>,
    pub svg: String,
}

#[derive(Debug, Clone)]
pub struct SvgElement {
    pub tag: String,
    pub attrs: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct IconMetadata {
    tags: Vec<String>,
    categories: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Manifest {
    pub version: String,
    pub source: String,
    pub icons: Vec<ManifestIcon>,
}

#[derive(Debug, Serialize)]
pub struct ManifestIcon {
    pub name: String,
    pub module: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub svg: String,
}

pub fn parse_icon(svg_path: &Path, json_path: &Path, names: IconNames) -> Result<Icon> {
    let svg = fs::read_to_string(svg_path)
        .with_context(|| format!("reading SVG {}", svg_path.display()))?;
    let metadata = fs::read_to_string(json_path)
        .with_context(|| format!("reading metadata {}", json_path.display()))?;
    let metadata: IconMetadata = serde_json::from_str(&metadata)
        .with_context(|| format!("parsing metadata {}", json_path.display()))?;

    let source_name = svg_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("SVG file stem is not valid UTF-8")?
        .to_owned();
    let svg = compact_svg(&svg);
    let (view_box, elements) =
        parse_svg(&svg, svg_path).with_context(|| format!("parsing SVG {}", svg_path.display()))?;

    Ok(Icon {
        source_name,
        component: names.component,
        module: names.module,
        module_ref: names.module_ref,
        tags: metadata.tags,
        categories: metadata.categories,
        view_box,
        elements,
        svg,
    })
}

pub fn manifest(version: &str, icons: &[Icon]) -> Manifest {
    Manifest {
        version: version.to_owned(),
        source: "lucide".to_owned(),
        icons: icons
            .iter()
            .map(|icon| ManifestIcon {
                name: icon.component.clone(),
                module: icon.module.clone(),
                tags: icon.tags.clone(),
                categories: icon.categories.clone(),
                svg: icon.svg.clone(),
            })
            .collect(),
    }
}

fn compact_svg(svg: &str) -> String {
    svg.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_svg(svg: &str, svg_path: &Path) -> Result<(String, Vec<SvgElement>)> {
    let root_start = svg.find("<svg").context("finding opening <svg> tag")?;
    let root_end = svg[root_start..]
        .find('>')
        .map(|offset| root_start + offset)
        .context("finding closing > for <svg> tag")?;
    let root = &svg[root_start + 1..root_end];
    let (root_tag, root_attrs) = parse_tag(root)?;
    if root_tag != "svg" {
        bail!("expected root <svg>, found <{root_tag}>");
    }

    for attr in root_attrs.keys() {
        if !matches!(
            attr.as_str(),
            "xmlns"
                | "width"
                | "height"
                | "viewBox"
                | "fill"
                | "stroke"
                | "stroke-width"
                | "stroke-linecap"
                | "stroke-linejoin"
        ) {
            bail!(
                "unsupported root SVG attribute `{attr}` in {}",
                svg_path.display()
            );
        }
    }

    let view_box = root_attrs
        .get("viewBox")
        .cloned()
        .context("SVG root is missing viewBox")?;
    let close_svg = svg.rfind("</svg>").context("finding closing </svg> tag")?;
    if close_svg < root_end {
        bail!("closing </svg> appears before opening <svg>");
    }
    let body = &svg[root_end + 1..close_svg];
    let mut elements = Vec::new();
    let mut offset = 0;

    while let Some(relative_start) = body[offset..].find('<') {
        let start = offset + relative_start;
        if body[start + 1..].starts_with('/') {
            break;
        }

        let end = body[start..]
            .find('>')
            .map(|relative_end| start + relative_end)
            .context("finding closing > for child SVG tag")?;
        let mut raw = body[start + 1..end].trim();
        let self_closing = raw.ends_with('/');
        if self_closing {
            raw = raw[..raw.len() - 1].trim_end();
        }

        let (tag, attrs) = parse_tag(raw)?;
        elements.push(SvgElement { tag, attrs });

        offset = end + 1;
        if !self_closing {
            let close = format!("</{}>", elements.last().unwrap().tag);
            let close_start = body[offset..]
                .find(&close)
                .map(|relative_close| offset + relative_close)
                .with_context(|| format!("finding closing {close}"))?;
            offset = close_start + close.len();
        }
    }

    Ok((view_box, elements))
}

fn parse_tag(raw: &str) -> Result<(String, BTreeMap<String, String>)> {
    let raw = raw.trim();
    let tag_end = raw.find(char::is_whitespace).unwrap_or(raw.len());
    let tag = raw[..tag_end].to_owned();
    if tag.is_empty() {
        bail!("empty SVG tag");
    }

    let mut attrs = BTreeMap::new();
    let mut rest = &raw[tag_end..];
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        let name_end = rest
            .find(|ch: char| ch == '=' || ch.is_whitespace())
            .unwrap_or(rest.len());
        let name = &rest[..name_end];
        if name.is_empty() {
            bail!("empty SVG attribute name in <{tag}>");
        }
        rest = rest[name_end..].trim_start();
        if !rest.starts_with('=') {
            bail!("SVG attribute `{name}` in <{tag}> is missing `=`");
        }
        rest = rest[1..].trim_start();
        if !rest.starts_with('"') {
            bail!("SVG attribute `{name}` in <{tag}> is missing opening quote");
        }
        rest = &rest[1..];
        let value_end = rest.find('"').with_context(|| {
            format!("SVG attribute `{name}` in <{tag}> is missing closing quote")
        })?;
        attrs.insert(name.to_owned(), rest[..value_end].to_owned());
        rest = &rest[value_end + 1..];
    }

    Ok((tag, attrs))
}
