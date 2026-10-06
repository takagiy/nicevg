//! A small owned XML tree that parses and serializes the way the original
//! DOM implementation did, so repaired documents keep their exact text.
//! Updates are pure: they return a new document.

use quick_xml::Reader;
use quick_xml::events::Event;

pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Element(Element),
    Text(String),
    CData(String),
    Comment(String),
    Instruction { target: String, data: String },
    Doctype(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// Set on elements created in the SVG namespace where no ancestor
    /// declares it, so serialization adds the declaration.
    pub declares_svg_namespace: bool,
}

/// Position of an element: child indices from the document downwards.
pub type Path = Vec<usize>;

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(name: &str) -> Element {
        Element {
            name: name.to_owned(),
            attributes: Vec::new(),
            children: Vec::new(),
            declares_svg_namespace: false,
        }
    }

    /// The attribute value, or "" when absent.
    pub fn attr(&self, name: &str) -> &str {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map_or("", |(_, value)| value.as_str())
    }

    pub fn has_attr(&self, name: &str) -> bool {
        self.attributes.iter().any(|(key, _)| key == name)
    }

    /// Replaces an attribute in place, or appends it when new.
    pub fn with_attr(mut self, name: &str, value: &str) -> Element {
        match self.attributes.iter_mut().find(|(key, _)| key == name) {
            Some(entry) => entry.1 = value.to_owned(),
            None => self.attributes.push((name.to_owned(), value.to_owned())),
        }
        self
    }

    /// Concatenated text of all descendant text and CDATA nodes.
    pub fn text_content(&self) -> String {
        self.children
            .iter()
            .map(|child| match child {
                Node::Text(text) | Node::CData(text) => text.clone(),
                Node::Element(element) => element.text_content(),
                _ => String::new(),
            })
            .collect()
    }
}

impl Document {
    pub fn root_index(&self) -> usize {
        self.children
            .iter()
            .position(|child| matches!(child, Node::Element(_)))
            .expect("a parsed document has a root element")
    }

    pub fn root(&self) -> &Element {
        match &self.children[self.root_index()] {
            Node::Element(element) => element,
            _ => unreachable!("root_index points at an element"),
        }
    }

    pub fn element(&self, path: &[usize]) -> &Element {
        let (first, rest) = path.split_first().expect("paths are never empty");
        let mut current = match &self.children[*first] {
            Node::Element(element) => element,
            _ => panic!("path does not lead to an element"),
        };
        for index in rest {
            current = match &current.children[*index] {
                Node::Element(element) => element,
                _ => panic!("path does not lead to an element"),
            };
        }
        current
    }

    /// Every element with its path, in document order.
    pub fn elements(&self) -> Vec<(Path, &Element)> {
        fn walk<'a>(element: &'a Element, path: Path, out: &mut Vec<(Path, &'a Element)>) {
            out.push((path.clone(), element));
            for (index, child) in element.children.iter().enumerate() {
                if let Node::Element(child) = child {
                    let mut child_path = path.clone();
                    child_path.push(index);
                    walk(child, child_path, out);
                }
            }
        }
        let mut out = Vec::new();
        let root = self.root_index();
        walk(self.root(), vec![root], &mut out);
        out
    }

    pub fn elements_named(&self, name: &str) -> Vec<(Path, &Element)> {
        self.elements()
            .into_iter()
            .filter(|(_, element)| element.name == name)
            .collect()
    }

    /// The element and its ancestors, nearest first.
    pub fn lineage(&self, path: &[usize]) -> Vec<&Element> {
        (1..=path.len())
            .rev()
            .map(|depth| self.element(&path[..depth]))
            .collect()
    }

    /// A new document with the element at `path` transformed.
    pub fn update(&self, path: &[usize], change: impl FnOnce(Element) -> Element) -> Document {
        fn rebuild(nodes: &[Node], path: &[usize], change: Box<dyn FnOnce(Element) -> Element + '_>) -> Vec<Node> {
            let (first, rest) = path.split_first().expect("paths are never empty");
            let mut nodes = nodes.to_vec();
            if let Node::Element(element) = &nodes[*first] {
                let updated = if rest.is_empty() {
                    change(element.clone())
                } else {
                    Element {
                        children: rebuild(&element.children, rest, change),
                        ..element.clone()
                    }
                };
                nodes[*first] = Node::Element(updated);
            }
            nodes
        }
        Document {
            children: rebuild(&self.children, path, Box::new(change)),
        }
    }

    /// A new document with the element at `path` replaced by `replacement`.
    pub fn replace(&self, path: &[usize], replacement: Element) -> Document {
        self.update(path, |_| replacement)
    }

    /// Whether an SVG-namespace element placed at `path` would need its own
    /// namespace declaration.
    pub fn needs_svg_namespace(&self, path: &[usize]) -> bool {
        self.lineage(path)
            .iter()
            .find(|element| element.has_attr("xmlns"))
            .is_none_or(|element| element.attr("xmlns") != SVG_NAMESPACE)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

/// Parses a document the way the original DOM parser did: line breaks are
/// normalized, literal whitespace in attribute values becomes spaces, text
/// before the root is kept only when it is whitespace, and trailing
/// whitespace at the end of the input is dropped.
#[tracing::instrument(name = "xml_parse", skip_all, fields(bytes = source.len()))]
pub fn parse(source: &str) -> Result<Document, ParseError> {
    if source.is_empty() {
        return Err(ParseError("invalid doc source".to_owned()));
    }
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let mut reader = Reader::from_str(&normalized);
    reader.config_mut().check_end_names = true;
    reader.config_mut().trim_text(false);

    let mut stack: Vec<Element> = Vec::new();
    let mut document: Vec<Node> = Vec::new();
    let error = |message: String| ParseError(message);

    let push = |stack: &mut Vec<Element>, document: &mut Vec<Node>, node: Node| match stack.last_mut() {
        Some(parent) => parent.children.push(node),
        None => document.push(node),
    };
    let push_text = |stack: &mut Vec<Element>, document: &mut Vec<Node>, text: String| {
        let siblings = match stack.last_mut() {
            Some(parent) => &mut parent.children,
            None => document,
        };
        match siblings.last_mut() {
            Some(Node::Text(existing)) => existing.push_str(&text),
            _ => siblings.push(Node::Text(text)),
        }
    };

    loop {
        let event = reader.read_event().map_err(|e| error(e.to_string()))?;
        match event {
            Event::Start(start) => stack.push(element_of(&start)?),
            Event::Empty(start) => {
                let element = element_of(&start)?;
                push(&mut stack, &mut document, Node::Element(element));
            }
            Event::End(_) => {
                let element = stack.pop().ok_or_else(|| error("unexpected end tag".to_owned()))?;
                push(&mut stack, &mut document, Node::Element(element));
            }
            Event::Text(text) => {
                let text = text.as_ref().to_owned();
                push_text(&mut stack, &mut document, text);
            }
            Event::GeneralRef(reference) => {
                let resolved = match reference.resolve_char_ref().map_err(|e| error(e.to_string()))? {
                    Some(character) => character.to_string(),
                    None => {
                        let name = reference.as_ref().to_owned();
                        quick_xml::escape::resolve_predefined_entity(&name)
                            .map(str::to_owned)
                            .ok_or_else(|| error(format!("entity not found:&{name};")))?
                    }
                };
                push_text(&mut stack, &mut document, resolved);
            }
            Event::CData(data) => {
                push(&mut stack, &mut document, Node::CData(data.as_ref().to_owned()));
            }
            Event::Comment(comment) => {
                push(&mut stack, &mut document, Node::Comment(comment.as_ref().to_owned()));
            }
            Event::Decl(declaration) => {
                let content = declaration.as_ref().to_owned();
                let data = content.strip_prefix("xml").unwrap_or(&content).trim_start().to_owned();
                push(
                    &mut stack,
                    &mut document,
                    Node::Instruction {
                        target: "xml".to_owned(),
                        data,
                    },
                );
            }
            Event::PI(instruction) => {
                let target = instruction.target().to_owned();
                let data = instruction.content().trim_start().to_owned();
                push(&mut stack, &mut document, Node::Instruction { target, data });
            }
            Event::DocType(doctype) => {
                let content = doctype.as_ref().trim().to_owned();
                push(&mut stack, &mut document, Node::Doctype(content));
            }
            Event::Eof => break,
        }
    }
    if !stack.is_empty() {
        return Err(error("unclosed xml element".to_owned()));
    }

    let roots = document.iter().filter(|node| matches!(node, Node::Element(_))).count();
    if roots != 1 {
        return Err(error(
            if roots == 0 {
                "invalid doc source"
            } else {
                "multiple root elements"
            }
            .to_owned(),
        ));
    }
    let root_index = document
        .iter()
        .position(|node| matches!(node, Node::Element(_)))
        .unwrap_or(0);
    let mut children: Vec<Node> = document
        .into_iter()
        .enumerate()
        .filter(|(index, node)| !(*index < root_index && matches!(node, Node::Text(text) if !text.trim().is_empty())))
        .map(|(_, node)| node)
        .collect();
    if matches!(children.last(), Some(Node::Text(text)) if text.trim().is_empty()) {
        children.pop();
    }
    let parsed = Document { children };
    if !parsed.root().name.eq_ignore_ascii_case("svg") {
        return Err(error("Input root element must be an SVG element.".to_owned()));
    }
    Ok(parsed)
}

fn element_of(start: &quick_xml::events::BytesStart) -> Result<Element, ParseError> {
    let name = start.name().as_ref().to_owned();
    let attributes = start
        .attributes()
        .with_checks(true)
        .map(|attribute| {
            let attribute = attribute.map_err(|e| ParseError(e.to_string()))?;
            let key = attribute.key.as_ref().to_owned();
            let raw = attribute.value.to_string();
            Ok((key, decode_attribute(&raw).map_err(ParseError)?))
        })
        .collect::<Result<Vec<_>, ParseError>>()?;
    Ok(Element {
        name,
        attributes,
        children: Vec::new(),
        declares_svg_namespace: false,
    })
}

/// Resolves references in an attribute value and turns literal whitespace
/// characters into spaces, as attribute-value normalization requires.
fn decode_attribute(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(position) = rest.find(['&', '\t', '\n', '\r']) {
        out.push_str(&rest[..position]);
        let tail = &rest[position..];
        if tail.starts_with('&') {
            let end = tail
                .find(';')
                .ok_or_else(|| format!("unterminated reference in {raw}"))?;
            out.push_str(&resolve_reference(&tail[1..end])?);
            rest = &tail[end + 1..];
        } else {
            out.push(' ');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    Ok(out)
}

fn resolve_reference(name: &str) -> Result<String, String> {
    let numeric = name
        .strip_prefix("#x")
        .map(|hex| u32::from_str_radix(hex, 16))
        .or_else(|| name.strip_prefix('#').map(str::parse::<u32>));
    match numeric {
        Some(Ok(code)) => char::from_u32(code)
            .map(|character| character.to_string())
            .ok_or_else(|| format!("invalid character reference &{name};")),
        Some(Err(_)) => Err(format!("invalid character reference &{name};")),
        None => quick_xml::escape::resolve_predefined_entity(name)
            .map(str::to_owned)
            .ok_or_else(|| format!("entity not found:&{name};")),
    }
}

fn escape_text(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '&' => "&amp;".to_owned(),
            other => other.to_string(),
        })
        .collect()
}

fn escape_attribute(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '&' => "&amp;".to_owned(),
            '"' => "&quot;".to_owned(),
            '\t' => "&#9;".to_owned(),
            '\n' => "&#10;".to_owned(),
            '\r' => "&#13;".to_owned(),
            other => other.to_string(),
        })
        .collect()
}

fn serialize_node(node: &Node) -> String {
    match node {
        Node::Element(element) => serialize_element(element),
        Node::Text(text) => escape_text(text),
        Node::CData(data) => format!("<![CDATA[{}]]>", data.replace("]]>", "]]]]><![CDATA[>")),
        Node::Comment(comment) => format!("<!--{comment}-->"),
        Node::Instruction { target, data } => format!("<?{target} {data}?>"),
        Node::Doctype(content) => format!("<!DOCTYPE {content}>"),
    }
}

fn serialize_element(element: &Element) -> String {
    let attributes: String = element
        .attributes
        .iter()
        .map(|(name, value)| format!(" {name}=\"{}\"", escape_attribute(value)))
        .chain(
            element
                .declares_svg_namespace
                .then(|| format!(" xmlns=\"{SVG_NAMESPACE}\"")),
        )
        .collect();
    if element.children.is_empty() {
        format!("<{}{attributes}/>", element.name)
    } else {
        let children: String = element.children.iter().map(serialize_node).collect();
        format!("<{}{attributes}>{children}</{}>", element.name, element.name)
    }
}

#[tracing::instrument(name = "xml_serialize", skip_all)]
pub fn serialize(document: &Document) -> String {
    document.children.iter().map(serialize_node).collect()
}
