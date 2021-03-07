//! An HTML renderer for [`lignin`](https://github.com/Tamschi/lignin) that does *some* syntactic and *no* semantic validation.
//!
//! Escaping is performed automatically where necessary, but the output isn't guaranteed to be minimal.
//!
//! **`lignin-html` is not round-trip-safe regarding any HTML parser implementation.**  
//! This is impossible for any HTML renderer that accepts adjacent [`Node::Text`]s, but maybe should still be noted explicitly.
//!
//! # About the Documentation
//!
//! HTML terms are ***bold italic*** and linked to the [HTML specification](https://html.spec.whatwg.org/multipage/syntax.html) (as of 2021-03-04).
//!
//! # Caveats
//!
//! In HTML comments, if illegal comment text is encountered, certain dashes (`-`) are **silently** replaced with equal signs (`=`) and pipe characters (`|`) are **silently** inserted around the comment text as necessary.
//! See [***Comments***](https://html.spec.whatwg.org/multipage/syntax.html#comments).
//!
//! > Originally I was going to use [zero width non-joiner](https://graphemica.com/200C) and [zero width joiner](https://graphemica.com/200D) characters for this,
//! > to make the comment resemble the original better, but this could be a very bad idea if any transport in-between strips Unicode.
#![doc(html_root_url = "https://docs.rs/lignin-html/0.0.5")]
#![forbid(unsafe_code)]
#![no_std]
#![warn(clippy::pedantic)]

//TODO: Much more thorough tests, ideally with coverage.

#[cfg(doctest)]
pub mod readme {
	doc_comment::doctest!("../README.md");
}

use core::{
	fmt::{self, Display, Write},
	ops::Range,
};
use fmt::Debug;
pub use lignin;
use lignin::{Attribute, Element, Node, ThreadSafety};
use logos::{Lexer, Logos};

//TODO: Benchmark and text-size-check using `core::fmt` macros vs. calling `Write` methods.

/// Renders `vdom` into `target` as HTML document *with* [***DOCTYPE***](https://html.spec.whatwg.org/multipage/syntax.html#the-doctype).
///
/// `depth_limit` is measured in [`Node`]s and must be at least `1` to not error on it.
///
/// # Caveats
///
/// See [`render_fragment`#caveats].
///
/// # Errors
///
/// Iff `vdom` is found to represent invalid HTML.
///
/// > **Warning:** This function succeeding does not guarantee that the produced HTML is fully valid!
pub fn render_document<'a, S: ThreadSafety>(
	vdom: &'a Node<'a, S>,
	target: &mut impl Write,
	depth_limit: usize,
) -> Result<(), Error<'a, S>> {
	if depth_limit == 0 {
		return Err(Error(ErrorKind::DepthLimitExceeded(vdom)));
	}
	write!(target, "<!DOCTYPE html>")?;
	render_fragment(vdom, target, depth_limit)
}

/// Renders `vdom` into `target` as HTML fragment *without* [***DOCTYPE***](https://html.spec.whatwg.org/multipage/syntax.html#the-doctype).
///
/// `depth_limit` is measured in [`Node`]s and must be at least `1` to not error on it.
///
/// # Errors
///
/// Iff `vdom` is found to represent invalid HTML.
///
/// > **Warning:** This function succeeding does not guarantee that the produced HTML is fully valid!
#[allow(clippy::items_after_statements)]
#[allow(clippy::too_many_lines)]
pub fn render_fragment<'a, S: ThreadSafety>(
	vdom: &'a Node<'a, S>,
	target: &mut impl Write,
	depth_limit: usize,
) -> Result<(), Error<'a, S>> {
	if depth_limit == 0 {
		return Err(Error(ErrorKind::DepthLimitExceeded(vdom)));
	}
	match *vdom {
		// See <https://html.spec.whatwg.org/multipage/syntax.html#comments>.
		Node::Comment {
			comment,
			dom_binding: _,
		} => {
			// This is just a comment, so it shouldn't break the app.
			target.write_str("<!--")?;
			if comment.starts_with('>') || comment.starts_with("->") {
				target.write_char('|')?
			}

			#[derive(Logos)]
			enum CommentToken {
				#[token("<!--")]
				LtBangDashDash,
				#[token("-->")]
				DashDashGt,
				#[token("--!>")]
				DashDashBangGt,
				#[regex("(?s).", |lex| lex.slice().parse())]
				Other(char),
				#[error]
				Error,
			}

			for token in CommentToken::lexer(comment) {
				let replacement = match token {
					CommentToken::LtBangDashDash => "<!==",
					CommentToken::DashDashGt => "==>",
					CommentToken::DashDashBangGt => "==!>",
					CommentToken::Other(c) => {
						target.write_char(c)?;
						continue;
					}
					CommentToken::Error => unreachable!(),
				};
				target.write_str(replacement)?
			}

			if comment.ends_with("<!-") {
				target.write_char('|')?
			}
			target.write_str("-->")?;
		}

		// See <https://html.spec.whatwg.org/multipage/syntax.html#elements-2>.
		Node::HtmlElement {
			element,
			dom_binding: _,
		}
		| Node::SvgElement {
			element,
			dom_binding: _,
		} => {
			let &Element {
				name,
				attributes,
				ref content,
				event_bindings: _,
			} = element;

			/// See <https://html.spec.whatwg.org/multipage/syntax.html#syntax-attribute-name>.
			fn validate_attribute_name<S: ThreadSafety>(name: &str) -> Result<&str, Error<S>> {
				for c in name.chars() {
					match c {
						// <https://infra.spec.whatwg.org/#control>
						// <https://infra.spec.whatwg.org/#c0-control>
						'\0'..='\u{1F}' | '\u{7F}'..='\u{9F}' |

						// <https://html.spec.whatwg.org/multipage/syntax.html#syntax-attribute-name>
						' ' | '"' | '\'' | '>' | '/' | '=' |

						// <https://infra.spec.whatwg.org/#noncharacter>
						'\u{FDD0}'..='\u{FDEF}' => {
							return Err(Error(ErrorKind::InvalidAttributeName(name)))
						}
						c if ((c as u32) & 0xffff >= 0xfffe) && (c as u32) >> 16 <= 0x10 => {
							return Err(Error(ErrorKind::InvalidAttributeName(name)))
						}
						_ => (),
					}
				}
				Ok(name)
			}

			let kind = ElementKind::detect(name)
				.map_err(|name| Error(ErrorKind::InvalidElementName(name)))?;

			//TODO: Validate distinction between HTML and SVG elements.

			// Opening tag:
			write!(target, "<{}", name)?;
			for &Attribute {
				name: attribute_name,
				value,
			} in attributes
			{
				write!(target, " {}", validate_attribute_name(attribute_name)?,)?;

				let value_mode = AttributeValueMode::detect(value);
				target.write_str(match value_mode {
					AttributeValueMode::Empty => continue,
					AttributeValueMode::Unquoted => "=",
					AttributeValueMode::SingleQuoted => "='",
					AttributeValueMode::DoubleQuoted => "\"",
				})?;
				for c in value.chars() {
					match c {
						'&' => target.write_str("&amp;"),
						'"' if value_mode == AttributeValueMode::DoubleQuoted => {
							target.write_str("&quot;")
						}
						c => target.write_char(c),
					}?
				}
				match value_mode {
					AttributeValueMode::Empty => unreachable!(),
					AttributeValueMode::Unquoted => (),
					AttributeValueMode::SingleQuoted => target.write_char('\'')?,
					AttributeValueMode::DoubleQuoted => target.write_char('"')?,
				}
			}
			if kind == ElementKind::ForeignSelfClosing {
				// Note the space! This is required in case the last attribute was unquoted.
				target.write_str(" />")?
			} else {
				target.write_char('>')?;
			}

			// See <https://html.spec.whatwg.org/multipage/syntax.html#element-restrictions>.
			// Just adding the newline here unconditionally isn't "perfect", but it's most likely faster than checking if it's necessary.
			match kind {
				ElementKind::EscapableRawTextTextarea | ElementKind::NormalPre => {
					target.write_char('\n')?
				}
				_ => (),
			}

			// Content:
			match kind {
				ElementKind::Void | ElementKind::ForeignSelfClosing => {
					if !content.dom_empty() {
						return Err(Error(ErrorKind::NonEmptyVoidElementContent(content)));
					}
				}
				ElementKind::Template
				| ElementKind::Normal
				| ElementKind::NormalPre
				| ElementKind::ForeignNotSelfClosing => render_fragment(content, target, depth_limit - 1)?,
				ElementKind::RawText => render_raw_text(content, target, name, depth_limit - 1)?,

				ElementKind::EscapableRawText | ElementKind::EscapableRawTextTextarea => {
					render_escapable_raw_text(content, target, depth_limit - 1)?
				}
				ElementKind::PotentialCustomElementNameCharacter
				| ElementKind::Dash
				| ElementKind::Invalid => {
					unreachable!()
				}
			}

			// Closing tag:
			match kind {
				ElementKind::Void | ElementKind::ForeignSelfClosing => (),
				ElementKind::Template
				| ElementKind::RawText
				| ElementKind::EscapableRawText
				| ElementKind::EscapableRawTextTextarea
				| ElementKind::ForeignNotSelfClosing
				| ElementKind::Normal
				| ElementKind::NormalPre => write!(target, "</{}>", name)?,
				ElementKind::PotentialCustomElementNameCharacter
				| ElementKind::Dash
				| ElementKind::Invalid => {
					unreachable!()
				}
			}
		}

		Node::Memoized {
			state_key: _,
			content,
		} => render_fragment(content, target, depth_limit - 1)?,

		Node::Multi(nodes) => {
			for node in nodes {
				render_fragment(node, target, depth_limit - 1)?;
			}
		}
		Node::Keyed(reorderable_fragments) => {
			for fragment in reorderable_fragments {
				render_fragment(&fragment.content, target, depth_limit - 1)?
			}
		}

		Node::Text {
			text,
			dom_binding: _,
		} => {
			//FIXME: I haven't found the actual reference on this yet.

			#[derive(Logos)]
			enum PlainTextToken<'a> {
				/// This could close this element or start a new one.
				#[token("<")]
				Lt,
				/// See <https://html.spec.whatwg.org/multipage/syntax.html#character-references>.
				///
				/// This could be an ambiguous ampersand or part something that would be parsed as character reference, so it's escaped unconditionally.
				#[token("&")]
				Ampersand,
				#[regex("[^<&]+")]
				SafeVerbatim(&'a str),
				#[error]
				Error,
			}

			for token in PlainTextToken::lexer(text) {
				match token {
					PlainTextToken::Lt => target.write_str("&lt;"),
					PlainTextToken::Ampersand => target.write_str("&amp;"),
					PlainTextToken::SafeVerbatim(str) => target.write_str(str),
					PlainTextToken::Error => unreachable!(),
				}?
			}
		}

		Node::RemnantSite(_) => todo!("`RemnantSite`"),
	};
	Ok(())
}

#[allow(clippy::items_after_statements)]
#[allow(clippy::too_many_lines)]
fn render_raw_text<'a, S: ThreadSafety>(
	vdom: &'a Node<'a, S>,
	target: &mut impl Write,
	element_name: &'a str,
	depth_limit: usize,
) -> Result<(), Error<'a, S>> {
	if depth_limit == 0 {
		return Err(Error(ErrorKind::DepthLimitExceeded(vdom)));
	}

	match vdom {
		Node::Comment { .. } | Node::HtmlElement { .. } | Node::SvgElement { .. } => {
			return Err(Error(ErrorKind::NonTextDomNodeInRawTextPosition(vdom)))
		}
		Node::Memoized {
			state_key: _,
			content,
		} => render_raw_text(content, target, element_name, depth_limit - 1)?,
		Node::Multi(nodes) => {
			for node in *nodes {
				render_raw_text(node, target, element_name, depth_limit - 1)?
			}
		}
		Node::Keyed(pairs) => {
			for pair in *pairs {
				render_raw_text(&pair.content, target, element_name, depth_limit - 1)?
			}
		}
		Node::Text {
			text,
			dom_binding: _,
		} => {
			/// See <https://html.spec.whatwg.org/multipage/syntax.html#elements-2> and <https://html.spec.whatwg.org/multipage/syntax.html#cdata-rcdata-restrictions>.
			///
			/// Unlike with escapable raw text, it's not possible to run escape the sequence (of course), so the error has to be a lot more precise.
			#[derive(Logos)]
			#[logos(extras = &'s mut RawTextExtras<'s>)]
			enum RawTextToken<'a> {
				#[token("<")]
				Lt,
				#[token("</", check_for_error)]
				LtSolidus(Result<(), Range<usize>>),
				#[regex("[^<]+")]
				SafeVerbatim(&'a str),
				#[error]
				Error,
			}

			struct RawTextExtras<'a> {
				pub element_name: &'a str,
				pub text: &'a str,
			}

			fn check_for_error<'a>(
				lex: &mut Lexer<'a, RawTextToken<'a>>,
			) -> Result<(), Range<usize>> {
				let start = lex.span().start;
				let end = lex.span().end;
				let extras = &mut *lex.extras;

				let name_range = end..end + extras.element_name.len();
				if name_range.end + 1 > extras.text.len() {
					return Ok(());
				}

				if !extras.text[name_range.clone()].eq_ignore_ascii_case(extras.element_name) {
					return Ok(());
				}

				// It is more clear to say we're slicing one past the name.
				#[allow(clippy::range_plus_one)]
				match extras.text.as_bytes()[name_range.end] {
					b'\t' | b'\n' | 0xC /* FORM FEED */ | b'\r' | b' ' | b'>' | b'/' => {
						Err(start..name_range.end+1)
					}
					_ => Ok(())
				}
			}

			let mut extras = RawTextExtras { element_name, text };
			for token in RawTextToken::lexer_with_extras(text, &mut extras) {
				match token {
					RawTextToken::Lt => target.write_char('<'),
					RawTextToken::LtSolidus(Ok(())) => target.write_str("</"),
					RawTextToken::LtSolidus(Err(invalid_range)) => {
						return Err(Error(ErrorKind::ElementClosedInRawText(
							&text[invalid_range],
						)))
					}
					RawTextToken::SafeVerbatim(str) => target.write_str(str),
					RawTextToken::Error => unreachable!(),
				}?
			}
		}
		Node::RemnantSite(_) => todo!("`RemnantSite`"),
	}
	Ok(())
}

#[allow(clippy::items_after_statements)]
#[allow(clippy::too_many_lines)]
fn render_escapable_raw_text<'a, S: ThreadSafety>(
	vdom: &'a Node<'a, S>,
	target: &mut impl Write,
	depth_limit: usize,
) -> Result<(), Error<'a, S>> {
	if depth_limit == 0 {
		return Err(Error(ErrorKind::DepthLimitExceeded(vdom)));
	}
	match vdom {
		Node::Comment { .. } | Node::HtmlElement { .. } | Node::SvgElement { .. } => {
			return Err(Error(ErrorKind::NonTextDomNodeInEscapableRawTextPosition(
				vdom,
			)))
		}
		Node::Memoized {
			state_key: _,
			content,
		} => render_escapable_raw_text(content, target, depth_limit - 1)?,
		Node::Multi(nodes) => {
			for node in *nodes {
				render_escapable_raw_text(node, target, depth_limit - 1)?
			}
		}
		Node::Keyed(pairs) => {
			for pair in *pairs {
				render_escapable_raw_text(&pair.content, target, depth_limit - 1)?
			}
		}
		Node::Text {
			text,
			dom_binding: _,
		} => {
			/// See <https://html.spec.whatwg.org/multipage/syntax.html#elements-2> and <https://html.spec.whatwg.org/multipage/syntax.html#cdata-rcdata-restrictions>.
			///
			/// Escaping with this model is a bit overzealous, but won't do harm and is fairly fast.
			#[derive(Logos)]
			enum EscapableRawTextToken<'a> {
				#[token("<")]
				Lt,
				#[token("</")]
				LtSolidus,
				/// See <https://html.spec.whatwg.org/multipage/syntax.html#character-references>.
				///
				/// This could be an ambiguous ampersand or part something that would be parsed as character reference, so it's escaped unconditionally.
				#[token("&")]
				Ampersand,
				#[regex("[^<&]+")]
				SafeVerbatim(&'a str),
				#[error]
				Error,
			}

			for token in EscapableRawTextToken::lexer(text) {
				match token {
					EscapableRawTextToken::Lt => target.write_char('<'),
					EscapableRawTextToken::LtSolidus => target.write_str("&lt;/"),
					EscapableRawTextToken::Ampersand => target.write_str("&amp;"),
					EscapableRawTextToken::SafeVerbatim(str) => target.write_str(str),
					EscapableRawTextToken::Error => unreachable!(),
				}?
			}
		}
		Node::RemnantSite(_) => todo!("`RemnantSite`"),
	}
	Ok(())
}

//FIXME?: This probably blows up the text size. Check and, if necessary, replace it with a better categorization algorithm.
#[derive(Logos, PartialEq)]
enum ElementKind {
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#void-elements>.
	#[regex("(?i)AREA")]
	#[regex("(?i)BASE")]
	#[regex("(?i)BR")]
	#[regex("(?i)COL")]
	#[regex("(?i)EMBED")]
	#[regex("(?i)HR")]
	#[regex("(?i)IMG")]
	#[regex("(?i)INPUT")]
	#[regex("(?i)LINK")]
	#[regex("(?i)META")]
	#[regex("(?i)PARAM")]
	#[regex("(?i)SOURCE")]
	#[regex("(?i)TRACK")]
	#[regex("(?i)WBR")]
	Void,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#the-template-element-2>.
	#[regex("(?i)TEMPLATE")]
	Template,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#raw-text-elements>.
	#[regex("(?i)SCRIPT")]
	#[regex("(?i)STYLE")]
	RawText,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#escapable-raw-text-elements>.
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#element-restrictions> for special handling.
	#[regex("(?i)TEXTAREA")]
	EscapableRawTextTextarea,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#escapable-raw-text-elements>.
	#[regex("(?i)TITLE")]
	EscapableRawText,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#foreign-elements>.
	//TODO
	ForeignSelfClosing,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#foreign-elements>.
	//TODO
	ForeignNotSelfClosing,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#normal-elements>.
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#element-restrictions> for special handling.
	NormalPre,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#normal-elements>,
	/// <https://html.spec.whatwg.org/multipage/syntax.html#syntax-tag-name>  
	/// => <https://infra.spec.whatwg.org/#ascii-alphanumeric>  
	/// => <https://infra.spec.whatwg.org/#ascii-digit> | <https://infra.spec.whatwg.org/#ascii-alpha>  
	/// =>  <https://infra.spec.whatwg.org/#ascii-digit> | <https://infra.spec.whatwg.org/#ascii-upper-alpha> | <https://infra.spec.whatwg.org/#ascii-lower-alpha>.
	#[regex("[0-9A-Za-z]+")]
	Normal,
	/// See <https://html.spec.whatwg.org/multipage/custom-elements.html#valid-custom-element-name>,
	/// except that this variant occurs as continuation and may never occur first and anything [`Normal`](`ElementKind::Normal`) is exempted here.
	// I'm not sure if splitting it up like this is the fastest options, but it's the most readable.
	#[token(".")]
	#[token("_")]
	#[token("\u{00B7}")]
	#[regex("[\u{C0}-\u{D6}]")]
	#[regex("[\u{D8}-\u{F6}]")]
	#[regex("[\u{F8}-\u{37D}]")]
	#[regex("[\u{37F}-\u{1FFF}]")]
	#[regex("[\u{200C}-\u{200D}]")]
	#[regex("[\u{203F}-\u{2040}]")]
	#[regex("[\u{2070}-\u{218F}]")]
	#[regex("[\u{2C00}-\u{2FEF}]")]
	#[regex("[\u{3001}-\u{D7FF}]")]
	#[regex("[\u{F900}-\u{FDCF}]")]
	#[regex("[\u{FDF0}-\u{FFFD}]")]
	#[regex("[\u{10000}-\u{EFFFF}]")]
	PotentialCustomElementNameCharacter,
	/// To flag custom elements as valid, see above.
	#[token("-")]
	Dash,
	#[error]
	Invalid,
}

impl ElementKind {
	pub fn detect(element_name: &str) -> Result<Self, &str> {
		let mut lexer = Self::lexer(element_name);
		let mut kind = match lexer.next() {
			// These may not appear first.
			None
			| Some(Self::Dash)
			| Some(Self::PotentialCustomElementNameCharacter)
			| Some(Self::Invalid) => return Err(element_name),
			Some(kind) => kind,
		};
		let mut dashed = false;
		let mut custom = false;
		for next in lexer {
			// If more than one token can be found, it's either a normal element starting with one of the others' names or invalid.
			match next {
				ElementKind::Invalid => return Err(element_name),
				ElementKind::PotentialCustomElementNameCharacter => custom = true,
				ElementKind::Dash => dashed = true,
				_ => (),
			}
			kind = ElementKind::Normal;
		}
		if custom && !dashed {
			Err(element_name)
		} else {
			Ok(kind)
		}
	}
}

/// See <https://html.spec.whatwg.org/multipage/syntax.html#attributes-2>.
#[derive(PartialEq, Eq)]
enum AttributeValueMode {
	Empty,
	Unquoted,
	SingleQuoted,
	DoubleQuoted,
}

impl AttributeValueMode {
	pub fn detect(value: &str) -> AttributeValueMode {
		if value.is_empty() {
			return Self::Empty;
		}

		let mut unquoted = true;
		let mut double_quoted = true;
		let mut single_quoted = true;

		for c in value.chars() {
			match c {
                // See <https://infra.spec.whatwg.org/#ascii-whitespace>.
                '\t' | '\n' | '\u{C}' /* FF */ | '\r' | ' ' |'=' | '<' | '>' | '`' => unquoted = false,
				'"' => { unquoted = false; double_quoted = false; }
				'\'' => { unquoted = false; single_quoted = false; }
                _ => (),
            }
		}

		if unquoted {
			Self::Unquoted
		} else if double_quoted {
			Self::DoubleQuoted
		} else if single_quoted {
			Self::SingleQuoted
		} else {
			Self::DoubleQuoted
		}
	}
}

#[derive(Debug)]
pub struct Error<'a, S: ThreadSafety>(ErrorKind<'a, S>);

#[derive(Debug)]
enum ErrorKind<'a, S: ThreadSafety> {
	InvalidElementName(&'a str),
	InvalidAttributeName(&'a str),
	NonEmptyVoidElementContent(&'a Node<'a, S>),
	NonTextDomNodeInRawTextPosition(&'a Node<'a, S>),
	NonTextDomNodeInEscapableRawTextPosition(&'a Node<'a, S>),
	ElementClosedInRawText(&'a str),
	DepthLimitExceeded(&'a Node<'a, S>),
	FmtError(fmt::Error),
}

impl<'a, S: ThreadSafety> From<fmt::Error> for Error<'a, S> {
	fn from(fmt_error: fmt::Error) -> Self {
		Self(ErrorKind::FmtError(fmt_error))
	}
}

impl<'a, S: ThreadSafety> Display for Error<'a, S> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self.0 {
			ErrorKind::InvalidElementName(str) => write!(f, "Invalid element name {:?}", str),
			ErrorKind::InvalidAttributeName(str) => write!(f, "Invalid attribute name {:?}", str),
			ErrorKind::NonEmptyVoidElementContent(node) => {
				write!(f, "Non-empty void element content {:?}", node)
			}
			ErrorKind::NonTextDomNodeInRawTextPosition(node) => {
				write!(f, "Non-text DOM node in raw text position {:?}", node)
			}
			ErrorKind::NonTextDomNodeInEscapableRawTextPosition(node) => {
				write!(
					f,
					"Non-text DOM node in escapable raw text position {:?}",
					node
				)
			}
			ErrorKind::ElementClosedInRawText(str) => {
				write!(f, "Element closed in raw text: {:?}", str)
			}
			ErrorKind::DepthLimitExceeded(_) => write!(f, "Depth limit exceeded"),
			ErrorKind::FmtError(fmt_error) => Display::fmt(fmt_error, f),
		}
	}
}

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
impl<'a, S: ThreadSafety> std::error::Error for Error<'a, S> {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		if let ErrorKind::FmtError(fmt_error) = &self.0 {
			Some(fmt_error)
		} else {
			None
		}
	}
}
