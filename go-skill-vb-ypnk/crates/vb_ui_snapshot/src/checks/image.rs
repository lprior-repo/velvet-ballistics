#![forbid(unsafe_code)]

#[cfg(feature = "std")]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use image::{DynamicImage, GenericImageView};

#[cfg(feature = "std")]
const APPROVED_WORDS: &[&str] = &[
    "velvet",
    "ballistics",
    "workflow",
    "execution",
    "run",
    "step",
    "action",
    "slot",
    "digest",
    "blob",
    "journal",
    "snapshot",
    "replay",
    "incident",
    "failure",
    "success",
    "running",
    "pending",
    "skipped",
    "cancelled",
    "transform",
    "validate",
    "fetch",
    "load",
    "save",
    "sink",
    "source",
    "schema",
    "checkpoint",
    "certificate",
    "verify",
    "idempotent",
    "retry",
    "capability",
    "taint",
    "durable",
    "safe",
    "unsafe",
    "overview",
    "graph",
    "authoring",
    "details",
    "theater",
    "registry",
    "storage",
    "doctor",
    "context",
    "ai",
    "seq",
    "shard",
    "index",
    "health",
    "uptime",
    "queue",
    "depth",
    "batch",
    "corrupt",
    "trim",
    "repair",
    "merge",
    "branch",
    "parallel",
    "foreach",
    "sequence",
    "switch",
    "start",
    "finish",
    "do",
    "onerror",
    "if",
];

#[cfg(feature = "std")]
pub fn is_word_approved(word: &str) -> bool {
    let lower = word.to_lowercase();
    APPROVED_WORDS.iter().any(|&w| w == lower)
}

#[cfg(feature = "std")]
pub fn extract_words_from_image(img: &DynamicImage) -> Vec<String> {
    let mut words = Vec::new();
    let (w, h) = img.dimensions();
    let gray = img.to_luma8();
    let rgba = img.to_rgba8();
    let mut word_buffer: Vec<u8> = Vec::new();
    let mut in_word = false;
    scan_image_words(
        w,
        h,
        &gray,
        &rgba,
        &mut word_buffer,
        &mut in_word,
        &mut words,
    );
    flush_word_buffer(&mut word_buffer, &mut in_word, &mut words);
    words
}

#[cfg(feature = "std")]
fn scan_image_words(
    w: u32,
    h: u32,
    gray: &image::GrayImage,
    rgba: &image::RgbaImage,
    buffer: &mut Vec<u8>,
    in_word: &mut bool,
    words: &mut Vec<String>,
) {
    for y in 0..h {
        scan_image_row(w, y, gray, rgba, buffer, in_word, words);
    }
}

#[cfg(feature = "std")]
fn scan_image_row(
    w: u32,
    y: u32,
    gray: &image::GrayImage,
    rgba: &image::RgbaImage,
    buffer: &mut Vec<u8>,
    in_word: &mut bool,
    words: &mut Vec<String>,
) {
    for x in 0..w {
        scan_image_pixel(x, y, gray, rgba, buffer, in_word, words);
    }
}

#[cfg(feature = "std")]
fn scan_image_pixel(
    x: u32,
    y: u32,
    gray: &image::GrayImage,
    rgba: &image::RgbaImage,
    buffer: &mut Vec<u8>,
    in_word: &mut bool,
    words: &mut Vec<String>,
) {
    let r = rgba.get_pixel(x, y)[0];
    let darkness = u8::MAX.saturating_sub(gray.get_pixel(x, y)[0]);
    if darkness > 80 && r > 200 {
        push_word_byte(r, buffer, in_word);
    } else {
        flush_word_buffer(buffer, in_word, words);
    }
}

#[cfg(feature = "std")]
fn push_word_byte(r: u8, buffer: &mut Vec<u8>, in_word: &mut bool) {
    if !*in_word {
        *in_word = true;
        buffer.clear();
    }
    if buffer.len() < 64 {
        buffer.push(r);
    }
}

#[cfg(feature = "std")]
fn flush_word_buffer(buffer: &mut Vec<u8>, in_word: &mut bool, words: &mut Vec<String>) {
    if *in_word && buffer.len() >= 3 {
        push_clean_word(buffer, words);
    }
    buffer.clear();
    *in_word = false;
}

#[cfg(feature = "std")]
fn push_clean_word(buffer: &[u8], words: &mut Vec<String>) {
    let s = String::from_utf8_lossy(buffer).to_string();
    let cleaned: String = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if cleaned.len() >= 2 {
        words.push(cleaned);
    }
}
