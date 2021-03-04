//! An HTML renderer for [`lignin`](https://github.com/Tamschi/lignin) that does *some* syntactic and *no* semantic validation.
//!
//! Escaping is performed automatically where necessary, but the output isn't guaranteed to be minimal.
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

#[cfg(doctest)]
pub mod readme {
	doc_comment::doctest!("../README.md");
}

use core::fmt::{self, Write};
use fmt::Debug;
pub use lignin;
use lignin::{Attribute, Element, Node, ThreadSafety};
use logos::Logos;

//TODO: Benchmark and text-size-check using `core::fmt` macros vs. calling `Write` methods.

/// Renders `vdom` into `target` as HTML document *with* [**DOCTYPE***](https://html.spec.whatwg.org/multipage/syntax.html#the-doctype).
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
) -> Result<(), Error<'a, S>> {
	write!(target, "<!DOCTYPE html>")?;
	render_fragment(vdom, target)
}

/// Renders `vdom` into `target` as HTML fragment *without* [**DOCTYPE***](https://html.spec.whatwg.org/multipage/syntax.html#the-doctype).
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
) -> Result<(), Error<'a, S>> {
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
				#[regex(".", |lex| lex.slice().parse())]
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
		Node::Element {
			element:
				&Element {
					name,
					attributes,
					ref content,
					event_bindings: _,
				},
			dom_binding: _,
		} => {
			fn validate_attribute_name<S: ThreadSafety>(name: &str) -> Result<&str, Error<S>> {
				todo!()
			}

			let kind = ElementKind::detect(name)
				.map_err(|name| Error(ErrorKind::InvalidElementName(name)))?;

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
				| ElementKind::ForeignNotSelfClosing => render_fragment(content, target)?,
				ElementKind::RawText => {
					todo!("RawText content")
				}
				ElementKind::EscapableRawText => {
					todo!("EscapableRawText content")
				}
				ElementKind::PotentialCustomElementNameCharacter
				| ElementKind::Dash
				| ElementKind::Invalid => {
					unreachable!()
				}
			}
			render_fragment(content, target)?;

			// Closing tag:
			match kind {
				ElementKind::Void | ElementKind::ForeignSelfClosing => (),
				ElementKind::Template
				| ElementKind::RawText
				| ElementKind::EscapableRawText
				| ElementKind::ForeignNotSelfClosing
				| ElementKind::Normal => write!(target, "</{}>", name)?,
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
		} => render_fragment(content, target)?,

		Node::Multi(nodes) => {
			for node in nodes {
				render_fragment(node, target)?;
			}
		}
		Node::Keyed(reorderable_fragments) => {
			for fragment in reorderable_fragments {
				render_fragment(&fragment.content, target)?
			}
		}
		Node::Text {
			text,
			dom_binding: _,
		} => {
			todo!()
		}
		Node::RemnantSite(_) => todo!("`RemnantSite`"),
	};
	Ok(())
}

//FIXME: This probably blows up the text size. Check and, if necessary, replace it with a better categorization algorithm.
#[derive(Logos, PartialEq)]
enum ElementKind {
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#void-elements>.
	#[regex("[aA][rR][eE][aA]")]
	#[regex("[bB][aA][sS][eE]")]
	#[regex("[bB][rR]")]
	#[regex("[cC][oO][lL]")]
	#[regex("[eE][mM][bB][eE][dD]")]
	#[regex("[hH][rR]")]
	#[regex("[iI][mM][gG]")]
	#[regex("[iI][nN][pP][uU][tT]")]
	#[regex("[lL][iI][nN][kK]")]
	#[regex("[mM][eE][tT][aA]")]
	#[regex("[pP][aA][rR][aA][mM]")]
	#[regex("[sS][oO][uU][rR][cC][eE]")]
	#[regex("[tT][rR][aA][cC][kK]")]
	#[regex("[wW][bB][rR]")]
	Void,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#the-template-element-2>.
	#[regex("[tT][eE][mM][pP][lL][aA][tT][eE]")]
	Template,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#raw-text-elements>.
	#[regex("[sS][cC][rR][iI][pP][tT]")]
	#[regex("[sS][tT][yY][lL][eE]")]
	RawText,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#escapable-raw-text-elements>.
	#[regex("[tT][eE][xX][tT][aA][rR][eE][aA]")]
	#[regex("[tT][iI][tT][lL][eE]")]
	EscapableRawText,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#foreign-elements>.
	//TODO
	ForeignSelfClosing,
	/// See <https://html.spec.whatwg.org/multipage/syntax.html#foreign-elements>.
	//TODO
	ForeignNotSelfClosing,
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

pub struct Error<'a, S: ThreadSafety>(ErrorKind<'a, S>);

enum ErrorKind<'a, S: ThreadSafety> {
	InvalidElementName(&'a str),
	InvalidAttributeName(&'a str),
	NonEmptyVoidElementContent(&'a Node<'a, S>),
	FmtError(fmt::Error),
}

impl<'a, S: ThreadSafety> From<fmt::Error> for Error<'a, S> {
	fn from(fmt_error: fmt::Error) -> Self {
		Self(ErrorKind::FmtError(fmt_error))
	}
}

impl<'a, S: ThreadSafety> Debug for Error<'a, S> {
	fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
		todo!()
	}
}
